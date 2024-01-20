use cainome_parser::{AbiParser, AbiParserLegacy};
use cainome_rs::{self};
use proc_macro::TokenStream;
use quote::quote;

mod macro_inputs;
mod macro_inputs_legacy;
mod spanned;

use crate::macro_inputs::ContractAbi;
use crate::macro_inputs_legacy::ContractAbiLegacy;

#[proc_macro]
pub fn abigen(input: TokenStream) -> TokenStream {
    abigen_internal(input)
}

#[proc_macro]
pub fn abigen_legacy(input: TokenStream) -> TokenStream {
    abigen_internal_legacy(input)
}

fn abigen_internal(input: TokenStream) -> TokenStream {
    let contract_abi = syn::parse_macro_input!(input as ContractAbi);

    let abi_entries = contract_abi.abi;
    let contract_name = contract_abi.name;

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

fn abigen_internal_legacy(input: TokenStream) -> TokenStream {
    let contract_abi = syn::parse_macro_input!(input as ContractAbiLegacy);

    let abi_entries = contract_abi.abi;
    let contract_name = contract_abi.name;

    let abi_tokens = AbiParserLegacy::collect_tokens(&abi_entries, &contract_abi.type_aliases)
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
