use std::collections::HashMap;

use cainome_parser::{AbiParser, ParserContext};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

use crate::{
    abi_to_tokenstream,
    expand::{ExpansionContext, ExpansionContextFactory},
    Abigen, ExecutionVersion,
};

pub fn assert_code_has<T: ToTokens>(generated: &TokenStream, expected: &T, message: &str) {
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
fn test_complex_case() {
    let abi = r#"[
        {
            "type": "struct",
            "name": "core::integer::u256",
            "members": [
            {
                "name": "low",
                "type": "core::integer::u128"
            },
            {
                "name": "high",
                "type": "core::integer::u128"
            }
            ]
        },
        {
            "type": "struct",
            "name": "contracts::abicov::structs::ToAlias",
            "members": [
            {
                "name": "a",
                "type": "core::integer::u32"
            }
            ]
        },
        {
            "type": "struct",
            "name": "contracts::abicov::structs::GenericOne::<contracts::abicov::structs::ToAlias>",
            "members": [
            {
                "name": "a",
                "type": "contracts::abicov::structs::ToAlias"
            },
            {
                "name": "b",
                "type": "core::felt252"
            },
            {
                "name": "c",
                "type": "core::integer::u256"
            }
            ]
        },
        {
            "type": "struct",
            "name": "contracts::abicov::structs::GenericTwo::<core::felt252, core::integer::u64>",
            "members": [
            {
                "name": "a",
                "type": "core::felt252"
            },
            {
                "name": "b",
                "type": "core::integer::u64"
            },
            {
                "name": "c",
                "type": "core::felt252"
            },
            {
                "name": "d",
                "type": "contracts::abicov::structs::ToAlias"
            },
            {
                "name": "e",
                "type": "core::array::Span::<contracts::abicov::structs::ToAlias>"
            },
            {
                "name": "f",
                "type": "contracts::abicov::structs::GenericOne::<contracts::abicov::structs::ToAlias>"
            }
            ]
        },
        {
            "type": "struct",
            "name": "contracts::abicov::structs::GenericOne::<core::integer::u256>",
            "members": [
            {
                "name": "a",
                "type": "core::integer::u256"
            },
            {
                "name": "b",
                "type": "core::felt252"
            },
            {
                "name": "c",
                "type": "core::integer::u256"
            }
            ]
        },
        {
            "type": "function",
            "name": "set_tuple_generic",
            "inputs": [
            {
                "name": "value",
                "type": "(contracts::abicov::structs::GenericOne::<core::integer::u256>, contracts::abicov::structs::GenericTwo::<core::felt252, core::integer::u64>)"
            }
            ],
            "outputs": [],
            "state_mutability": "external"
        }
    ]"#;

    let mut aliases = HashMap::new();

    aliases.insert(
        String::from("contracts::abicov::structs::GenericOne"),
        String::from("GenericOneBis"),
    );
    aliases.insert(
        String::from("contracts::abicov::structs::GenericTwo"),
        String::from("GenericTwoBis"),
    );
    aliases.insert(
        String::from("contracts::abicov::structs::ToAlias"),
        String::from("MyDef"),
    );

    let ctx = ExpansionContextFactory::new("MyContract")
        .with_contract_derives(&vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(&vec!["Debug".to_string(), "PartialEq".to_string()])
        .with_execution(ExecutionVersion::V3)
        .with_aliases(aliases)
        .build();

    let entries = AbiParser::parse_abi_string(&abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx));

    let generated = abi_to_tokenstream(&registry.unwrap(), &ctx);

    let expected = quote! {};

    assert_code_has(&generated, &expected, "asd");
}

#[test]
fn test_tuple_with_custom_type_as_func_argument_case() {
    let abi = r#"[
        {
            "type": "struct",
            "name": "core::integer::u256",
            "members": [
            {
                "name": "low",
                "type": "core::integer::u128"
            },
            {
                "name": "high",
                "type": "core::integer::u128"
            }
            ]
        },
        {
            "type": "struct",
            "name": "contracts::abicov::structs::ToAlias",
            "members": [
            {
                "name": "a",
                "type": "core::integer::u32"
            }
            ]
        },
        {
            "type": "function",
            "name": "set_tuple_generic",
            "inputs": [
            {
                "name": "value",
                "type": "(core::integer::u256, contracts::abicov::structs::ToAlias)"
            }
            ],
            "outputs": [],
            "state_mutability": "external"
        }
    ]"#;

    let ctx = ExpansionContextFactory::new("MyContract")
        .with_contract_derives(&vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(&vec!["Debug".to_string(), "PartialEq".to_string()])
        .build();

    let entries = AbiParser::parse_abi_string(&abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx)).unwrap();

    let generated = abi_to_tokenstream(&registry, &ctx);

    let expected = quote! {};

    assert_code_has(&generated, &expected, "asd");
}

#[test]
fn test_tuple_with_custom_genetic_type_as_func_argument_case() {
    let abi = r#"[
        {
            "type": "struct",
            "name": "contracts::abicov::structs::ToAlias<core::integer::u32>",
            "members": [
            {
                "name": "a",
                "type": "core::integer::u32"
            }
            ]
        },
        {
            "type": "function",
            "name": "set_tuple_generic",
            "inputs": [
            {
                "name": "value",
                "type": "(core::integer::u32, contracts::abicov::structs::ToAlias<core::integer::u32>)"
            }
            ],
            "outputs": [],
            "state_mutability": "external"
        }
    ]"#;

    let ctx = ExpansionContextFactory::new("MyContract")
        .with_contract_derives(&vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(&vec!["Debug".to_string(), "PartialEq".to_string()])
        .build();

    let entries = AbiParser::parse_abi_string(&abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx)).unwrap();

    let generated = abi_to_tokenstream(&registry, &ctx);

    let expected = quote! {};

    assert_code_has(&generated, &expected, "asd");
}

#[test]
fn test_tuple_with_custom_genetic_type_as_func_argument_case_with_alias() {
    let abi = r#"[
        {
            "type": "struct",
            "name": "contracts::abicov::structs::ToAlias<core::integer::u32>",
            "members": [
            {
                "name": "a",
                "type": "core::integer::u32"
            }
            ]
        },
        {
            "type": "function",
            "name": "set_tuple_generic",
            "inputs": [
            {
                "name": "value",
                "type": "(core::integer::u32, contracts::abicov::structs::ToAlias<core::integer::u32>)"
            }
            ],
            "outputs": [],
            "state_mutability": "external"
        }
    ]"#;

    let mut aliases = HashMap::new();

    aliases.insert(
        String::from("contracts::abicov::structs::ToAlias"),
        String::from("ToAlias"),
    );

    let ctx = ExpansionContextFactory::new("MyContract")
        .with_contract_derives(&vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(&vec!["Debug".to_string(), "PartialEq".to_string()])
        .with_aliases(aliases)
        .build();

    let entries = AbiParser::parse_abi_string(&abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx)).unwrap();

    let generated = abi_to_tokenstream(&registry, &ctx);

    let expected = quote! {};

    assert_code_has(&generated, &expected, "asd");
}

#[test]
fn test_tuple_with_generic_arg_with_2_parameters_resolves() {
    let abi_json = format!(
        r#"[
            {{
                "type": "struct",
                "name": "my::Generic::<core::felt252, core::felt252>", 
                "members": [
                    {{
                        "name": "f1",
                        "type": "core::felt252"
                    }},
                    {{
                        "name": "f2",
                        "type": "core::felt252"
                    }}
                ]
            }},
            {{
                "type": "struct",
                "name": "my::New", 
                "members": [
                    {{
                        "name": "f1",
                        "type": "(my::Generic::<core::felt252, core::felt252>)"
                    }}
                ]
            }}

        ]"#
    );

    let ctx = ParserContext::new();

    let abi_entries = AbiParser::parse_abi_string(&abi_json).unwrap();

    let mut registry = AbiParser::build_registry(abi_entries, ctx).unwrap();

    let ctx = ExpansionContextFactory::new("MyContract").build();

    registry.apply_substitutions(&ctx.substitutions);

    let token = registry
        .get("my::Generic::<core::felt252, core::felt252>")
        .unwrap();

    // let Token::Struct(s) = &*token.borrow() else {
    //     unreachable!()
    // };

    // assert_eq!(s.type_path, "my::Generic::<core::felt252, core::felt252>");
}
