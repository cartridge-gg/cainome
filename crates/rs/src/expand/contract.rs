use std::collections::HashMap;

use cainome_parser::{
    tokens::{Constructor, Function, FunctionOutputKind, NamedToken, Token},
    TypeRegistry,
};
use proc_macro2::TokenStream;

use crate::{
    expand::{
        structure::{struct_declaration, struct_implementation},
        types::CairoToRust,
        utils, Expandable, ExpansionContext, ExpansionResult, ROOT_MODULE_NAME,
    },
    ExecutionVersion,
};

use quote::quote;

pub struct Contract {
    pub name: String,
    pub derives: Vec<String>,
    pub readonly_methods_sorted: Vec<Function>,
    pub mutating_methods: Vec<Function>,
    pub constructor: Option<Constructor>,
}

impl Contract {
    pub fn new(name: &str, derives: Vec<String>, abi: &TypeRegistry) -> Self {
        let mut readonly_methods = vec![];
        let mut mutating_methods = vec![];

        for token in abi.get_functions() {
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

        readonly_methods.sort_by_key(|f| f.name.to_string());

        let constructor = abi.get_constructor();

        Self {
            name: name.to_string(),
            derives,
            mutating_methods,
            readonly_methods_sorted: readonly_methods,
            constructor,
        }
    }

    fn get_inputs_for_func(
        f: &Function,
        ctx: &ExpansionContext,
    ) -> Vec<(TokenStream, TokenStream)> {
        let mut out = vec![];

        for NamedToken { name, token } in f.inputs.iter() {
            println!("Getting input for func: {} || {:?}", name, &*token.borrow());
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

            let ser = if token.is_tuple() {
                quote! {
                    __calldata.extend(<#ty>::cairo_serialize(#name));
                }
            } else {
                quote! {
                    __calldata.extend(#ty::cairo_serialize(#name));
                }
            };

            serializations.push(ser);
        }
        serializations
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

        let inputs = Self::get_inputs_for_func(f, ctx);

        let input_names = inputs
            .iter()
            .map(|(name, _ty)| name.clone())
            .collect::<Vec<_>>();

        let inputs_sub = inputs
            .iter()
            .map(|(name, ty)| quote!(#name:#ty))
            .collect::<Vec<_>>();

        let func_name_call = utils::str_to_ident(&format!("{func_name}_getcall"));

        let ccs = utils::str_to_type(&ctx.cainome_serde_path);

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

        let inputs_sub = Self::get_inputs_for_func(f, ctx)
            .iter()
            .map(|(name, ty)| quote!(#name:#ty))
            .collect::<Vec<_>>();

        let ccs = utils::str_to_type(&ctx.cainome_serde_path);

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
        let constructor_calldata_name = format!("{contract_name}Calldata");
        let reader = utils::str_to_ident(format!("{contract_name}Reader").as_str());

        let contract_name_ident = utils::str_to_ident(&contract_name);
        let constructor_calldata_name_ident = utils::str_to_ident(&constructor_calldata_name);
        let snrs_types = utils::snrs_types();
        let snrs_accounts = utils::snrs_accounts();
        let snrs_providers = utils::snrs_providers();
        let snrs_contract = utils::snrs_contract();
        let cairo_serde = utils::str_to_type(&ctx.cainome_serde_path);
        let cairo_lang = utils::cairo_lang();

        let internal_derives = self
            .derives
            .iter()
            .map(|d| utils::str_to_type(d))
            .collect::<Vec<_>>();

        let externals_with_name = self
            .mutating_methods
            .iter()
            .map(|f| (f.name.to_owned(), Self::expand_mutable_method(f, ctx)))
            .collect::<Vec<_>>();

        let views_with_name = self
            .readonly_methods_sorted
            .iter()
            .map(|f| {
                (
                    f.name.to_owned(),
                    Self::expand_readonly_method(f, "A::Provider", ctx),
                )
            })
            .collect::<Vec<_>>();

        let mut externals_and_views_with_name = [externals_with_name, views_with_name].concat();
        externals_and_views_with_name.sort_by_key(|(name, _)| name.to_string());

        let externals_and_views = externals_and_views_with_name
            .iter()
            .map(|(_, f)| f)
            .collect::<Vec<_>>();

        let reader_views = self
            .readonly_methods_sorted
            .iter()
            .map(|f| Self::expand_readonly_method(f, "P", ctx))
            .collect::<Vec<_>>();

        let derives = if !internal_derives.is_empty() {
            quote! {
                #[derive(#(#internal_derives,)*)]
            }
        } else {
            quote! {}
        };

        let declaration = if ctx.add_declaration {
            let max_bytecode_size = ctx.sierra_max_bytecode_size;
            let add_pythonic_hints = ctx.sierra_add_pythonic_hints;

            quote! {
                pub async fn declare(
                    path: &std::path::Path,
                    account: &A,
                    use_blake2s_class_hash: bool,
                ) -> Result<#snrs_types::Felt, Box<dyn std::error::Error>>
                where
                    A::SignError: 'static,
                {
                    use starknet_api::contract_class::compiled_class_hash::HashableCompiledClass;
                    let sierra_class: #cairo_lang::contract_class::ContractClass =
                        serde_json::from_slice::<#cairo_lang::contract_class::ContractClass>(
                            std::fs::read(path)?.as_slice(),
                        )?;

                    let casm_class = #cairo_lang::casm_contract_class::CasmContractClass::from_contract_class(
                        sierra_class,
                        #add_pythonic_hints,
                        #max_bytecode_size,
                    )?;

                    let hash_version = if use_blake2s_class_hash {
                        starknet_api::contract_class::compiled_class_hash::HashVersion::V2
                    } else {
                        starknet_api::contract_class::compiled_class_hash::HashVersion::V1
                    };

                    let class_hash = #snrs_types::Felt::from_bytes_be(
                        &casm_class.hash(&hash_version).0.to_bytes_be(),
                    );

                    let contract_artifact: #snrs_types::contract::SierraClass =
                        serde_json::from_reader(std::fs::File::open(path)?)?;

                    let declaration = account.declare_v3(
                        std::sync::Arc::new(contract_artifact.flatten()?),
                        class_hash,
                    );

                    let declaration_result = declaration.send().await?;

                    Ok(declaration_result.class_hash)
                }
            }
        } else {
            quote! {}
        };

        let generate_salt = ctx.deployer_generate_salt;
        let is_unique = ctx.deployer_is_unique;

        let mut constructor_calldata = quote! {};
        let mut calldata_arg = quote! {};
        let mut calldata_serialise = quote! {vec![]};

        if let Some(constructor) = &self.constructor {
            if !constructor.inputs.is_empty() {
                let declaration = struct_declaration(
                    &constructor_calldata_name,
                    &constructor.inputs,
                    &vec![],
                    &HashMap::new(),
                    &ctx,
                );

                let implementation = struct_implementation(
                    &constructor_calldata_name,
                    &constructor.inputs,
                    &vec![],
                    &HashMap::new(),
                    ctx,
                );

                constructor_calldata = quote! {
                    #declaration

                    #implementation
                };
                calldata_arg = quote! {calldata: &#constructor_calldata_name_ident, };
                calldata_serialise = quote! {
                    #constructor_calldata_name_ident::cairo_serialize(&calldata)
                };
            }
        }

        let deployment = if ctx.add_deployment {
            quote! {
                pub async fn deploy(
                    udc_selector: #snrs_contract::UdcSelector,
                    account: A,
                    class_hash: #snrs_types::Felt,
                    #calldata_arg
                ) -> Result<Self, Box<dyn std::error::Error>>
                where
                    A::SignError: 'static,
                {
                    use #cairo_serde::CairoSerde;

                    let generate_salt = #generate_salt;
                    let is_unique = #is_unique;

                    let salt: #snrs_types::Felt = if generate_salt {
                        use rand::Rng;
                        rand::rng().random::<u128>().into()
                    } else {
                        #snrs_types::Felt::ZERO
                    };

                    let factory =
                        #snrs_contract::ContractFactory::new_with_udc(class_hash, &account, udc_selector);

                    let deployment = factory.deploy_v3(
                        #calldata_serialise,
                        salt,
                        is_unique
                    );

                    let deployed_address = deployment.deployed_address();

                    let tx = deployment.send().await?;

                    Ok(Self::new(deployed_address, account))
                }
            }
        } else {
            quote! {}
        };

        let expanded_contract = quote! {

            #derives
            pub struct #contract_name_ident<A: #snrs_accounts::ConnectedAccount + Sync> {
                pub address: #snrs_types::Felt,
                pub account: A,
                pub block_id: #snrs_types::BlockId,
            }

            #constructor_calldata

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

                #(#externals_and_views)*

                #declaration

                #deployment
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

        vec![ExpansionResult::new(ROOT_MODULE_NAME).with_item(&contract_name, expanded_contract)]
    }
}
