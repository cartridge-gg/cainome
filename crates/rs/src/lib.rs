use cainome_parser::tokens::StateMutability;
use cainome_parser::TokenizedAbi;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

mod expand;

use crate::expand::utils;
use crate::expand::{CairoContract, CairoEnum, CairoEnumEvent, CairoFunction, CairoStruct};

/// Expands the given ABI into rust bindings.
///
/// # Arguments
///
/// * `contract_name` - Name of the contract.
/// * `types_aliases` - Types aliases to avoid name conflicts.
/// * `file_path` - The sierra artifact / abi file path.
pub fn generate(contract_name: &str, types_aliases: &Vec<(String, String)>, abi_tokens: &TokenizedAbi) -> TokenStream2 {
    let contract_name = utils::str_to_ident(contract_name);
}

/// Converts the given ABI (in it's tokenize form) into rust bindings.
///
/// # Arguments
///
/// * `contract_name` - Name of the contract.
/// * `abi_tokens` - Tokenized ABI.
pub fn abi_to_tokenstream(contract_name: &str, abi_tokens: &TokenizedAbi) -> TokenStream2 {
    let contract_name = utils::str_to_ident(contract_name);

    let mut tokens: Vec<TokenStream2> = vec![];

    tokens.push(CairoContract::expand(contract_name.clone()));

    for s in &abi_tokens.structs {
        let s_composite = s.to_composite().expect("composite expected");
        tokens.push(CairoStruct::expand_decl(s_composite));
        tokens.push(CairoStruct::expand_impl(s_composite));
    }

    for e in &abi_tokens.enums {
        let e_composite = e.to_composite().expect("composite expected");
        tokens.push(CairoEnum::expand_decl(e_composite));
        tokens.push(CairoEnum::expand_impl(e_composite));

        tokens.push(CairoEnumEvent::expand(
            e.to_composite().expect("composite expected"),
            &abi_tokens.enums,
            &abi_tokens.structs,
        ));
    }

    let mut reader_views = vec![];
    let mut views = vec![];
    let mut externals = vec![];

    // Interfaces are not yet reflected in the generated contract.
    // Then, the standalone functions and functions from interfaces are put together.
    let mut functions = abi_tokens.functions.clone();
    for funcs in abi_tokens.interfaces.values() {
        functions.extend(funcs.clone());
    }

    for f in functions {
        let f = f.to_function().expect("function expected");
        match f.state_mutability {
            StateMutability::View => {
                reader_views.push(CairoFunction::expand(f, true));
                views.push(CairoFunction::expand(f, false));
            }
            StateMutability::External => externals.push(CairoFunction::expand(f, false)),
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
