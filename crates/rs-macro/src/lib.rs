use cainome_parser::AbiParser;
use cainome_rs::{self};
use proc_macro::TokenStream;
use quote::quote;

mod macro_inputs;
mod spanned;

use crate::macro_inputs::ContractAbi;

#[proc_macro]
pub fn abigen(input: TokenStream) -> TokenStream {
    abigen_internal(input)
}

fn abigen_internal(input: TokenStream) -> TokenStream {
    let contract_abi = syn::parse_macro_input!(input as ContractAbi);

    let contract_name = contract_abi.name;
    let abi_entries = contract_abi.abi;

    let abi_tokens = AbiParser::collect_tokens(&abi_entries, &contract_abi.type_aliases)
        .expect("failed tokens parsing");

    let expanded = cainome_rs::abi_to_tokenstream(&contract_name.to_string(), &abi_tokens);

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
