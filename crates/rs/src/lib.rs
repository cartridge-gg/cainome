use cainome_parser::tokens::StateMutability;
use cainome_parser::AbiParser;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

mod expand;
mod macro_inputs;
mod spanned;
mod types;

use crate::expand::utils;
use crate::expand::{CairoContract, CairoEnum, CairoFunction, CairoStruct};
use crate::macro_inputs::ContractAbi;

#[proc_macro]
pub fn abigen(input: TokenStream) -> TokenStream {
    abigen_internal(input)
}

fn abigen_internal(input: TokenStream) -> TokenStream {
    let contract_abi = syn::parse_macro_input!(input as ContractAbi);

    let contract_name = contract_abi.name;
    let abi_entries = contract_abi.abi;

    let abi_tokens = AbiParser::collect_tokens(&abi_entries).expect("failed tokens parsing");
    let abi_tokens = AbiParser::organize_tokens(abi_tokens, &contract_abi.type_aliases);

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
        }
    }

    // TODO: events need to expand auto-deserialization based on selectors.

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

    if let Some(out_path) = contract_abi.output_path {
        let content: String = expanded.to_string();
        match std::fs::write(out_path, content) {
            Ok(_) => (),
            Err(e) => panic!("Failed to write to file: {}", e),
        }

        quote!().into()
    } else {
        expanded.into()
    }
}
