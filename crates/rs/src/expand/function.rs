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
use cainome_parser::tokens::{Function, FunctionOutputKind, StateMutability, Token};
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
    pub fn expand(func: &Function, is_for_reader: bool) -> TokenStream2 {
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

        let out_type = match func.get_output_kind() {
            FunctionOutputKind::NoOutput => quote!(()),
            FunctionOutputKind::Cairo1 => {
                let out_type = utils::str_to_type(&func.outputs[0].to_rust_type_path());
                quote!(#out_type)
            }
            FunctionOutputKind::Cairo0 => {
                let out_type = utils::str_to_type(&func.get_cairo0_output_name());
                quote!(#out_type)
            }
        };

        let inputs = get_func_inputs(&func.inputs);
        let func_name_call = utils::str_to_ident(&format!("{}_getcall", func_name));
        let type_param = if is_for_reader {
            utils::str_to_type("P")
        } else {
            utils::str_to_type("A::Provider")
        };

        let ccs = utils::cainome_cairo_serde();

        match &func.state_mutability {
            StateMutability::View => quote! {
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
                    pub fn #func_name_call(
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
                    pub fn #func_name_ident(
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
