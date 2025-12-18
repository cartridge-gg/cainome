use cainome_parser::{AbiParser, ParserContext};
use cainome_rs::expand::ExpansionContext;
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use quote::quote;

mod macro_inputs;
mod macro_inputs_legacy;
mod spanned;

use crate::macro_inputs::ContractAbi;
use crate::macro_inputs_legacy::ContractAbiLegacy;

#[proc_macro_error]
#[proc_macro]
pub fn abigen(input: TokenStream) -> TokenStream {
    abigen_internal(input)
}

#[proc_macro_error]
#[proc_macro]
pub fn abigen_legacy(input: TokenStream) -> TokenStream {
    abigen_internal_legacy(input)
}

fn abigen_internal(input: TokenStream) -> TokenStream {
    let contract_abi = syn::parse_macro_input!(input as ContractAbi);

    let abi_entries = contract_abi.abi;
    let contract_name = contract_abi.name;

    let ctx = ExpansionContext::new(&contract_name.to_string())
        .with_contract_derives(&contract_abi.contract_derives)
        .with_derives(&contract_abi.derives)
        .with_execution(contract_abi.execution_version)
        .with_type_skips(contract_abi.type_skips);

    let registry = AbiParser::build_registry(abi_entries, ParserContext::from(&ctx))
        .expect("failed tokens parsing");

    let expanded = cainome_rs::abi_to_tokenstream2(&registry, &ctx);

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

    let ctx = ExpansionContext::new(&contract_name.to_string())
        .with_contract_derives(&contract_abi.contract_derives)
        .with_derives(&contract_abi.derives)
        .with_execution(contract_abi.execution_version)
        .with_type_skips(contract_abi.type_skips);

    let registry = AbiParser::build_registry(abi_entries, ParserContext::from(&ctx))
        .expect("failed tokens parsing");

    let expanded = cainome_rs::abi_to_tokenstream2(&registry, &ctx);

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
