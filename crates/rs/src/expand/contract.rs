use cainome_parser::{
    tokens::{Function, FunctionOutputKind, NamedToken, Token},
    TokenizedAbi,
};
use proc_macro2::TokenStream;

use crate::{
    expand::{types::CairoToRust, utils, Expandable, ExpansionContext, Module, ROOT_MODULE_NAME},
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

    fn get_inputs_for_func(f: &Function) -> Vec<(TokenStream, TokenStream)> {
        let mut out = vec![];

        for NamedToken { name, token } in f.inputs.iter() {
            let name = utils::str_to_ident(name);
            let token = &*token.borrow();
            let ty = utils::str_to_type(&token.to_rust_type_path());
            out.push((quote!(#name), quote!(&#ty)));
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
        _ctx: &ExpansionContext,
    ) -> TokenStream {
        let type_param_ident = utils::str_to_type(type_param);
        let func_name = &f.name;
        let func_name_ident = utils::str_to_ident(func_name);

        let out_type = {
            let var = Self::get_func_output_type(f);
            quote!(#var)
        };

        let serializations = Self::get_serializations_for_func(f);

        let inputs_sub = Self::get_inputs_for_func(&f)
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

impl Expandable for CairoContract {
    fn expand(&self, ctx: &super::ExpansionContext) -> Vec<Module> {
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

        vec![Module::new(ROOT_MODULE_NAME).add_item(&contract_name, q)]
    }
}

#[cfg(test)]
mod tests {
    use cainome_parser::{
        tokens::{Function, NamedToken, Token},
        AbiParser, TypeRegistry,
    };
    use proc_macro2::TokenStream;
    use quote::ToTokens;
    use syn::parse_quote;

    use crate::expand::{contract::CairoContract, render, Expandable, ExpansionContext};

    fn assert_code_has<T: ToTokens>(generated: &TokenStream, expected: &T, message: &str) {
        let file: syn::File = syn::parse2(generated.clone()).expect("expected file-like tokens");

        let expected_str = expected.to_token_stream().to_string();

        let has_match = file.items.iter().any(|item| {
            let item = item.to_token_stream().to_string();
            item.contains(&expected_str)
        });

        assert!(
            has_match,
            "{}. Expected: {} In: {}",
            message,
            expected_str,
            generated.to_string()
        );
    }

    #[test]
    fn test_naive_contract_expansion() {
        let ctx = ExpansionContext::new("ContractName");

        let contract = CairoContract {
            name: "ContractName".to_string(),
            derives: vec![],
            readonly_methods: vec![],
            mutating_methods: vec![],
        };

        let generated = render(contract.expand(&ctx));

        let expected: TokenStream = parse_quote! {
            pub struct ContractName<A: starknet::accounts::ConnectedAccount + Sync> {
                pub address: starknet::core::types::Felt,
                pub account: A,
                pub block_id: starknet::core::types::BlockId,
            }
        };

        assert_code_has(&generated, &expected, "Contract struct not found");

        let expected: TokenStream = parse_quote! {
            pub struct ContractNameReader<P: starknet::providers::Provider + Sync> {
                pub address: starknet::core::types::Felt,
                pub provider: P,
                pub block_id: starknet::core::types::BlockId,
            }
        };

        assert_code_has(&generated, &expected, "Contract reader not found");
    }

    #[test]
    fn test_naive_contract_with_view_function() {
        let mut registry = TypeRegistry::new();

        let function = Token::Function(Function {
            name: "get_value".to_string(),
            inputs: vec![NamedToken {
                name: "in1".to_string(),
                token: registry.get("felt").unwrap(),
            }],
            outputs: vec![registry.get("felt").unwrap()],
            state_mutability: cainome_parser::tokens::StateMutability::View,
            named_outputs: vec![],
        });

        registry.set("some_unique_path", function);

        let ctx = ExpansionContext::new("ContractName");

        let abi = AbiParser::create_tokenized_abi(registry.values()).unwrap();

        let contract = CairoContract::new("ContractName", vec![], &abi);

        let generated = render(contract.expand(&ctx));

        let expected: TokenStream = parse_quote! {
            #[allow(clippy::ptr_arg)]
            #[allow(clippy::too_many_arguments)]
            pub fn get_value(
                &self,
                in1: &starknet::core::types::Felt
            ) -> cainome::cairo_serde::call::FCall<A::Provider, starknet::core::types::Felt> {
                use cainome::cairo_serde::CairoSerde;
                let mut __calldata = vec![];
                __calldata.extend(starknet::core::types::Felt::cairo_serialize(in1));
                let __call = starknet::core::types::FunctionCall {
                    contract_address: self.address,
                    entry_point_selector: starknet::macros::selector!("get_value"),
                    calldata: __calldata,
                };
                cainome::cairo_serde::call::FCall::new(__call, self.provider(), )
            }
        };

        assert_code_has(&generated, &expected, "Contract class not found");

        let expected: TokenStream = parse_quote! {
            #[allow(clippy::ptr_arg)]
            #[allow(clippy::too_many_arguments)]
            pub fn get_value(
                &self,
                in1: &starknet::core::types::Felt
            ) -> cainome::cairo_serde::call::FCall<P, starknet::core::types::Felt> {
                use cainome::cairo_serde::CairoSerde;
                let mut __calldata = vec![];
                __calldata.extend(starknet::core::types::Felt::cairo_serialize(in1));
                let __call = starknet::core::types::FunctionCall {
                    contract_address: self.address,
                    entry_point_selector: starknet::macros::selector!("get_value"),
                    calldata: __calldata,
                };
                cainome::cairo_serde::call::FCall::new(__call, self.provider(), )
            }
        };

        assert_code_has(&generated, &expected, "Contract reader not found");
    }

    #[test]
    fn test_naive_contract_with_view_mutating_function() {
        let mut registry = TypeRegistry::new();

        let function = Token::Function(Function {
            name: "get_value".to_string(),
            inputs: vec![NamedToken {
                name: "in1".to_string(),
                token: registry.get("felt").unwrap(),
            }],
            outputs: vec![registry.get("felt").unwrap()],
            state_mutability: cainome_parser::tokens::StateMutability::External,
            named_outputs: vec![],
        });

        registry.set("some_unique_path", function);

        let ctx = ExpansionContext::new("ContractName");

        let abi = AbiParser::create_tokenized_abi(registry.values()).unwrap();

        let contract = CairoContract::new("ContractName", vec![], &abi);

        let generated = render(contract.expand(&ctx));

        let expected: TokenStream = parse_quote! {
            #[allow(clippy::ptr_arg)]
            #[allow(clippy::too_many_arguments)]
            pub fn get_value_getcall(
                &self,
                in1: &starknet::core::types::Felt
            ) -> starknet::core::types::Call {
                use cainome::cairo_serde::CairoSerde;
                let mut __calldata = vec![];
                __calldata.extend(starknet::core::types::Felt::cairo_serialize(in1));
                starknet::core::types::Call {
                    to: self.address,
                    selector: starknet::macros::selector!("get_value"),
                    calldata: __calldata,
                }
            }
        };

        assert_code_has(&generated, &expected, "Contract method not found");

        let expected: TokenStream = parse_quote! {
            #[allow(clippy::ptr_arg)]
            #[allow(clippy::too_many_arguments)]
            pub fn get_value(
                &self,
                in1: &starknet::core::types::Felt
            ) -> starknet::accounts::ExecutionV3<A> {
                let __call = self.get_value_getcall(in1);
                self.account.execute_v3(vec![__call])
            }
        };

        assert_code_has(&generated, &expected, "Contract method not found");
    }
}
