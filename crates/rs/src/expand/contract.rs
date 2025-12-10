use cainome_parser::{
    tokens::{Function, FunctionOutputKind, NamedToken, Token},
    TokenizedAbi,
};
use proc_macro2::TokenStream;

use crate::{
    expand::{types::CairoToRust, utils, Expandable, ExpansionContext},
    ExecutionVersion,
};
use quote::quote;

pub struct CairoContract {
    name: String,
    derives: Vec<String>,
    readonly_methods: Vec<Function>,
    mutating_methods: Vec<Function>,
}

impl CairoContract {
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

    fn get_inputs_for_func(f: &Function) -> Vec<TokenStream> {
        let mut out: Vec<TokenStream> = vec![];

        for NamedToken { name, token } in f.inputs.iter() {
            let name = utils::str_to_ident(name);
            let token = &*token.borrow();
            let ty = utils::str_to_type(&token.to_rust_type_path());
            out.push(quote!(#name:&#ty));
        }

        out
    }

    fn get_serializations_for_func(f: &Function) -> Vec<TokenStream> {
        let mut serializations: Vec<TokenStream> = vec![];

        for NamedToken { name, token } in f.inputs.iter() {
            let name = utils::str_to_ident(name);
            let token = &*token.borrow();
            let ty = utils::str_to_type(&token.to_rust_type_path());

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

    fn get_func_output_type(f: &Function) -> syn::Type {
        match f.get_output_kind() {
            FunctionOutputKind::NoOutput => utils::str_to_type("()"),
            FunctionOutputKind::Cairo1 => {
                let output_token = &*f.outputs[0].borrow();
                utils::str_to_type(&output_token.to_rust_type_path())
            }
            FunctionOutputKind::Cairo0 => utils::str_to_type(&f.get_cairo0_output_name()),
        }
    }

    fn expand_mutable_method(f: &Function, ctx: &ExpansionContext) -> TokenStream {
        let func_name = &f.name;
        let func_name_ident = utils::str_to_ident(func_name);

        let serializations = Self::get_serializations_for_func(f);

        let inputs = Self::get_inputs_for_func(&f);
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
                #(#inputs),*
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
                #(#inputs),*
            ) -> #exec_type {
                use #ccs::CairoSerde;

                let mut __calldata = vec![];
                #(#serializations)*

                let __call = starknet::core::types::Call {
                    to: self.address,
                    selector: starknet::macros::selector!(#func_name),
                    calldata: __calldata,
                };

                #exec_call
            }
        }
    }

    fn expand_readonly_method(
        f: &Function,
        type_param: &str,
        ctx: &ExpansionContext,
    ) -> TokenStream {
        let func_name = &f.name;
        let func_name_ident = utils::str_to_ident(func_name);

        let out_type = {
            let var = Self::get_func_output_type(f);
            quote!(#var)
        };

        let serializations = Self::get_serializations_for_func(f);
        let inputs = Self::get_inputs_for_func(&f);
        let ccs = utils::cainome_cairo_serde();

        quote! {
            #[allow(clippy::ptr_arg)]
            #[allow(clippy::too_many_arguments)]
            pub fn #func_name_ident(
                &self,
                #(#inputs),*
            ) -> #ccs::call::FCall<#type_param, #out_type> {
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

impl Expandable for CairoContract {
    fn expand(&self, expansion_context: &super::ExpansionContext) -> TokenStream {
        let contract_name = self.name.clone();
        let reader = utils::str_to_ident(format!("{}Reader", contract_name).as_str());

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
            .map(|f| Self::expand_mutable_method(f, &expansion_context))
            .collect::<Vec<_>>();

        let views = self
            .readonly_methods
            .iter()
            .map(|f| Self::expand_readonly_method(f, "A::Provider", &expansion_context))
            .collect::<Vec<_>>();

        let reader_views = self
            .readonly_methods
            .iter()
            .map(|f| Self::expand_readonly_method(f, "P", &expansion_context))
            .collect::<Vec<_>>();

        let q = quote! {

            #[derive(#(#internal_derives,)*)]
            pub struct #contract_name<A: #snrs_accounts::ConnectedAccount + Sync> {
                pub address: #snrs_types::Felt,
                pub account: A,
                pub block_id: #snrs_types::BlockId,
            }

            impl<A: #snrs_accounts::ConnectedAccount + Sync> #contract_name<A> {
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

            #[derive(#(#internal_derives,)*)]
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

        q
    }
}
