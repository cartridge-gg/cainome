use cainome_parser::{
    tokens::{Function, FunctionOutputKind, NamedToken, Token},
    TokenizedAbi,
};
use proc_macro2::TokenStream;

use crate::{
    expand::{
        types::CairoToRust, utils, Expandable, ExpansionContext, ExpansionResult, ROOT_MODULE_NAME,
    },
    ExecutionVersion,
};
use quote::quote;

pub struct Contract {
    pub name: String,
    pub derives: Vec<String>,
    pub readonly_methods: Vec<Function>,
    pub mutating_methods: Vec<Function>,
}

impl Contract {
    pub fn new(name: &str, derives: Vec<String>, abi: &TokenizedAbi) -> Self {
        let mut readonly_methods = vec![];
        let mut mutating_methods = vec![];

        for token in abi.functions.iter() {
            let Token::Function(func) = &*token.borrow() else {
                continue;
            };
            match func.state_mutability {
                cainome_parser::tokens::StateMutability::External => {
                    mutating_methods.push(func.clone());
                }
                cainome_parser::tokens::StateMutability::View => {
                    readonly_methods.push(func.clone());
                }
            }
        }

        Self {
            name: name.to_string(),
            derives: derives,
            mutating_methods,
            readonly_methods,
        }
    }

    fn get_inputs_for_func(
        f: &Function,
        ctx: &ExpansionContext,
    ) -> Vec<(TokenStream, TokenStream)> {
        let mut out = vec![];

        for NamedToken { name, token } in f.inputs.iter() {
            let name = utils::str_to_ident(name);
            let token = &*token.borrow();
            let ty = utils::str_to_type(&token.to_rust_type_path(ctx));
            out.push((quote!(#name), quote!(&#ty)));
        }

        out
    }

    fn get_serializations_for_func(f: &Function, ctx: &ExpansionContext) -> Vec<TokenStream> {
        let mut serializations: Vec<TokenStream> = vec![];

        for NamedToken { name, token } in f.inputs.iter() {
            let name = utils::str_to_ident(name);
            let token = &*token.borrow();
            let ty = utils::str_to_type(&token.to_rust_type_path(ctx));

            let ser = match token {
                Token::Tuple(_) => quote! {
                    __calldata.extend(<#ty>::cairo_serialize(#name));
                },
                _ => quote!(__calldata.extend(#ty::cairo_serialize(#name));),
            };

            serializations.push(ser);
        }
        return serializations;
    }

    fn get_func_output_type(f: &Function, ctx: &ExpansionContext) -> syn::Type {
        match f.get_output_kind() {
            FunctionOutputKind::NoOutput => utils::str_to_type("()"),
            FunctionOutputKind::Cairo1 => {
                let output_token = &*f.outputs[0].borrow();
                utils::str_to_type(&output_token.to_rust_type_path(ctx))
            }
            FunctionOutputKind::Cairo0 => utils::str_to_type(&f.get_cairo0_output_name()),
        }
    }

    fn expand_mutable_method(f: &Function, ctx: &ExpansionContext) -> TokenStream {
        let func_name = &f.name;
        let func_name_ident = utils::str_to_ident(func_name);

        let serializations = Self::get_serializations_for_func(f, ctx);

        let inputs = Self::get_inputs_for_func(&f, ctx);
        let input_names = inputs
            .iter()
            .map(|(name, _ty)| name.clone())
            .collect::<Vec<_>>();
        let inputs_sub = inputs
            .iter()
            .map(|(name, ty)| quote!(#name:#ty))
            .collect::<Vec<_>>();

        let func_name_call = utils::str_to_ident(&format!("{}_getcall", func_name));

        let ccs = utils::cainome_cairo_serde();

        let exec_type = utils::str_to_type(match &ctx.execution_version {
            ExecutionVersion::V1 => "starknet::accounts::ExecutionV1<A>",
            ExecutionVersion::V3 => "starknet::accounts::ExecutionV3<A>",
        });

        let exec_call = match &ctx.execution_version {
            ExecutionVersion::V1 => quote!(self.account.execute_v1(vec![__call])),
            ExecutionVersion::V3 => quote!(self.account.execute_v3(vec![__call])),
        };

        quote! {
            #[allow(clippy::ptr_arg)]
            #[allow(clippy::too_many_arguments)]
            pub fn #func_name_call(
                &self,
                #(#inputs_sub),*
            ) -> starknet::core::types::Call {
                use #ccs::CairoSerde;

                let mut __calldata = vec![];
                #(#serializations)*

                starknet::core::types::Call {
                    to: self.address,
                    selector: starknet::macros::selector!(#func_name),
                    calldata: __calldata,
                }
            }

            #[allow(clippy::ptr_arg)]
            #[allow(clippy::too_many_arguments)]
            pub fn #func_name_ident(
                &self,
                #(#inputs_sub),*
            ) -> #exec_type {
                let __call = self.#func_name_call(#(#input_names),*);
                #exec_call
            }
        }
    }

    fn expand_readonly_method(
        f: &Function,
        type_param: &str,
        ctx: &ExpansionContext,
    ) -> TokenStream {
        let type_param_ident = utils::str_to_type(type_param);
        let func_name = &f.name;
        let func_name_ident = utils::str_to_ident(func_name);

        let out_type = {
            let var = Self::get_func_output_type(f, ctx);
            quote!(#var)
        };

        let serializations = Self::get_serializations_for_func(f, ctx);

        let inputs_sub = Self::get_inputs_for_func(&f, ctx)
            .iter()
            .map(|(name, ty)| quote!(#name:#ty))
            .collect::<Vec<_>>();

        let ccs = utils::cainome_cairo_serde();

        quote! {
            #[allow(clippy::ptr_arg)]
            #[allow(clippy::too_many_arguments)]
            pub fn #func_name_ident(
                &self,
                #(#inputs_sub),*
            ) -> #ccs::call::FCall<#type_param_ident, #out_type> {
                use #ccs::CairoSerde;

                let mut __calldata = vec![];
                #(#serializations)*

                let __call = starknet::core::types::FunctionCall {
                    contract_address: self.address,
                    entry_point_selector: starknet::macros::selector!(#func_name),
                    calldata: __calldata,
                };

                #ccs::call::FCall::new(
                    __call,
                    self.provider(),
                )
            }
        }
    }
}

impl Expandable for Contract {
    fn expand(&self, ctx: &super::ExpansionContext) -> Vec<ExpansionResult> {
        let contract_name = self.name.clone();
        let reader = utils::str_to_ident(format!("{}Reader", contract_name).as_str());

        let contract_name_ident = utils::str_to_ident(&contract_name);
        let snrs_types = utils::snrs_types();
        let snrs_accounts = utils::snrs_accounts();
        let snrs_providers = utils::snrs_providers();

        let internal_derives = self
            .derives
            .iter()
            .map(|d| utils::str_to_type(d))
            .collect::<Vec<_>>();

        let externals = self
            .mutating_methods
            .iter()
            .map(|f| Self::expand_mutable_method(f, &ctx))
            .collect::<Vec<_>>();

        let views = self
            .readonly_methods
            .iter()
            .map(|f| Self::expand_readonly_method(f, "A::Provider", &ctx))
            .collect::<Vec<_>>();

        let reader_views = self
            .readonly_methods
            .iter()
            .map(|f| Self::expand_readonly_method(f, "P", &ctx))
            .collect::<Vec<_>>();

        let derives = if internal_derives.len() > 0 {
            quote! {
                #[derive(#(#internal_derives,)*)]
            }
        } else {
            quote! {}
        };

        let q = quote! {

            #derives
            pub struct #contract_name_ident<A: #snrs_accounts::ConnectedAccount + Sync> {
                pub address: #snrs_types::Felt,
                pub account: A,
                pub block_id: #snrs_types::BlockId,
            }

            impl<A: #snrs_accounts::ConnectedAccount + Sync> #contract_name_ident<A> {
                pub fn new(address: #snrs_types::Felt, account: A) -> Self {
                    Self {
                        address,
                        account,
                        block_id:
                        #snrs_types::BlockId::Tag(#snrs_types::BlockTag::PreConfirmed)
                    }
                }

                pub fn set_contract_address(&mut self, address: #snrs_types::Felt) {
                    self.address = address;
                }

                pub fn provider(&self) -> &A::Provider {
                    self.account.provider()
                }

                pub fn set_block(&mut self, block_id: #snrs_types::BlockId) {
                    self.block_id = block_id;
                }

                pub fn with_block(self, block_id: #snrs_types::BlockId) -> Self {
                    Self { block_id, ..self }
                }

                #(#views)*
                #(#externals)*
            }

            #derives
            pub struct #reader<P: #snrs_providers::Provider + Sync> {
                pub address: #snrs_types::Felt,
                pub provider: P,
                pub block_id: #snrs_types::BlockId,
            }

            impl<P: #snrs_providers::Provider + Sync> #reader<P> {
                pub fn new(
                    address: #snrs_types::Felt,
                    provider: P,
                ) -> Self {
                    Self { address, provider, block_id: #snrs_types::BlockId::Tag(#snrs_types::BlockTag::PreConfirmed) }
                }

                pub fn set_contract_address(&mut self, address: #snrs_types::Felt) {
                    self.address = address;
                }

                pub fn provider(&self) -> &P {
                    &self.provider
                }

                pub fn set_block(&mut self, block_id: #snrs_types::BlockId) {
                    self.block_id = block_id;
                }

                pub fn with_block(self, block_id: #snrs_types::BlockId) -> Self {
                    Self { block_id, ..self }
                }

                #(#reader_views)*
            }
        };

        vec![ExpansionResult::new(ROOT_MODULE_NAME).with_item(&contract_name, q)]
    }
}
