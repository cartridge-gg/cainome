//! # Functions types expansion
//!
//! This module contains the auto-generated types
//! for the functions of a contract for which the bindings are requested.
//!
//! Starknet has two types of functions:
//!
//! * `Views` - Which are also named `FunctionCall` that don't modifying the state. Readonly operations.
//! * `Externals` - Where a transaction is involved and can alter the state. Write operations.
//!
//! For each of these functions, there is a struct that is dedicated for each function of the contract,
//! based on it's state mutability found in the ABI itself.
//!
//! * `FCall` - Struct for readonly functions.
//! * `Execution` - Struct from starknet-rs for transaction based functions.
//!
//! ## Examples
//!
//! ```ignore (pseudo-code)
//! // TODO
//! ```
use cainome_parser::tokens::{Function, StateMutability, Token};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::expand::types::CairoToRust;
use crate::expand::utils;

fn get_func_inputs(inputs: &[(String, Token)]) -> Vec<TokenStream2> {
    let mut out: Vec<TokenStream2> = vec![];

    for (name, token) in inputs {
        let name = utils::str_to_ident(name);
        let ty = utils::str_to_type(&token.to_rust_type_path());
        out.push(quote!(#name:&#ty));
    }

    out
}

pub struct CairoFunction;

impl CairoFunction {
    pub fn expand_trait_decl(func: &Function) -> TokenStream2 {
        let func_name = &func.name;
        let func_name_ident = utils::str_to_ident(func_name);

        let out_type = if func.outputs.is_empty() {
            quote!(())
        } else {
            // We consider only one type for Cairo 1, if any.
            // The outputs field is a list for historical reason from Cairo 0
            // were tuples were used as returned values.
            let out_type = utils::str_to_type(&func.outputs[0].to_rust_type_path());
            quote!(#out_type)
        };

        let inputs = get_func_inputs(&func.inputs);
        let func_name_call = utils::str_to_ident(&format!("{}_getcall", func_name));

        let ccs = utils::cainome_cairo_serde();

        match &func.state_mutability {
            StateMutability::View => quote! {
                #[allow(clippy::ptr_arg)]
                #[allow(clippy::too_many_arguments)]
                fn #func_name_ident(
                    &self,
                    #(#inputs),*
                ) -> #ccs::call::FCall<P, #out_type>;
            },
            StateMutability::External => {
                quote! {
                    #[allow(clippy::ptr_arg)]
                    #[allow(clippy::too_many_arguments)]
                    fn #func_name_call(
                        &self,
                        #(#inputs),*
                    ) -> starknet::accounts::Call;

                    #[allow(clippy::ptr_arg)]
                    fn #func_name_ident(
                        &self,
                        #(#inputs),*
                    ) -> starknet::accounts::Execution<A>;
                }
            }
        }
    }

    pub fn expand(func: &Function, provider_generic_arg: &str) -> TokenStream2 {
        let func_name = &func.name;
        let func_name_ident = utils::str_to_ident(func_name);

        let mut serializations: Vec<TokenStream2> = vec![];
        for (name, token) in &func.inputs {
            let name = utils::str_to_ident(name);
            let ty = utils::str_to_type(&token.to_rust_type_path());

            let ser = match token {
                Token::Tuple(_) => quote! {
                    __calldata.extend(<#ty>::cairo_serialize(#name));
                },
                _ => quote!(__calldata.extend(#ty::cairo_serialize(#name));),
            };

            serializations.push(ser);
        }

        let out_type = if func.outputs.is_empty() {
            quote!(())
        } else {
            // We consider only one type for Cairo 1, if any.
            // The outputs field is a list for historical reason from Cairo 0
            // were tuples were used as returned values.
            let out_type = utils::str_to_type(&func.outputs[0].to_rust_type_path());
            quote!(#out_type)
        };

        let inputs = get_func_inputs(&func.inputs);
        let func_name_call = utils::str_to_ident(&format!("{}_getcall", func_name));
        let type_param = utils::str_to_type(provider_generic_arg);

        let ccs = utils::cainome_cairo_serde();

        match &func.state_mutability {
            StateMutability::View => quote! {
                #[allow(clippy::ptr_arg)]
                #[allow(clippy::too_many_arguments)]
                fn #func_name_ident(
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
                        self.provider_ref(),
                    )
                }
            },
            StateMutability::External => {
                // For now, Execution can't return the list of calls.
                // This would be helpful to easily access the calls
                // without having to add `_getcall()` method.
                // If starknet-rs provides a way to get the calls,
                // we can remove `_getcall()` method.
                //
                // TODO: if it's possible to do it with lifetime,
                // this can be tried in an issue.
                quote! {
                    #[allow(clippy::ptr_arg)]
                    #[allow(clippy::too_many_arguments)]
                    fn #func_name_call(
                        &self,
                        #(#inputs),*
                    ) -> starknet::accounts::Call {
                        use #ccs::CairoSerde;

                        let mut __calldata = vec![];
                        #(#serializations)*

                        starknet::accounts::Call {
                            to: self.address,
                            selector: starknet::macros::selector!(#func_name),
                            calldata: __calldata,
                        }
                    }

                    #[allow(clippy::ptr_arg)]
                    fn #func_name_ident(
                        &self,
                        #(#inputs),*
                    ) -> starknet::accounts::Execution<A> {
                        use #ccs::CairoSerde;

                        let mut __calldata = vec![];
                        #(#serializations)*

                        let __call = starknet::accounts::Call {
                            to: self.address,
                            selector: starknet::macros::selector!(#func_name),
                            calldata: __calldata,
                        };

                        self.account.execute(vec![__call])
                    }
                }
            }
        }
    }
}
