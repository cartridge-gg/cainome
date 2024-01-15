use cainome_parser::tokens::{StateMutability, Token};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::collections::HashMap;

mod expand;

use crate::expand::utils;
use crate::expand::{CairoContract, CairoEnum, CairoEnumEvent, CairoFunction, CairoStruct};

/// Converts the given ABI (in it's tokenize form) into rust bindings.
///
/// # Arguments
///
/// * `contract_name` - Name of the contract.
/// * `abi_tokens` - Tokenized ABI.
pub fn abi_to_tokenstream(
    contract_name: &str,
    abi_tokens: &HashMap<String, Vec<Token>>,
) -> TokenStream2 {
    let contract_name = utils::str_to_ident(contract_name);

    let mut tokens: Vec<TokenStream2> = vec![];

    tokens.push(CairoContract::expand(contract_name.clone()));

    if let Some(structs) = abi_tokens.get("structs") {
        for s in structs {
            let s_composite = s.to_composite().expect("composite expected");
            tokens.push(CairoStruct::expand_decl(s_composite));
            tokens.push(CairoStruct::expand_impl(s_composite));
        }
    }

    if let Some(enums) = abi_tokens.get("enums") {
        for e in enums {
            let e_composite = e.to_composite().expect("composite expected");
            tokens.push(CairoEnum::expand_decl(e_composite));
            tokens.push(CairoEnum::expand_impl(e_composite));

            tokens.push(CairoEnumEvent::expand(
                e.to_composite().expect("composite expected"),
                enums,
                abi_tokens
                    .get("structs")
                    .expect("at least one struct expected to expand events"),
            ));
        }
    }

    let mut reader_views = vec![];
    let mut views = vec![];
    let mut externals = vec![];

    if let Some(funcs) = abi_tokens.get("functions") {
        for f in funcs {
            let f = f.to_function().expect("function expected");
            match f.state_mutability {
                StateMutability::View => {
                    reader_views.push(CairoFunction::expand(f, true));
                    views.push(CairoFunction::expand(f, false));
                }
                StateMutability::External => externals.push(CairoFunction::expand(f, false)),
            }
        }
    }

    let reader = utils::str_to_ident(format!("{}Reader", contract_name).as_str());

    tokens.push(quote! {
        impl<A: starknet::accounts::ConnectedAccount + Sync> #contract_name<A> {
            #(#views)*
            #(#externals)*
        }

        impl<P: starknet::providers::Provider + Sync> #reader<P> {
            #(#reader_views)*
        }
    });

    let expanded = quote! {
        #(#tokens)*
    };

    expanded
}
