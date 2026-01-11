use std::collections::HashMap;

use crate::expand::for_tests::{
    assert_code_has, assert_code_has_not, assert_code_has_not_struct, assert_code_has_struct,
};
use crate::{abi_to_tokenstream, expand::ExpansionContextFactory, ExecutionVersion};
use cainome_parser::{tokens::Token, AbiParser, ParserContext};
use quote::quote;
use syn::parse_quote;

#[ignore = "Generics are not yet handled properly"]
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

    assert_code_has(
        &generated,
        &quote! {
            #[derive(Debug, PartialEq)]
            pub struct MyDef {
                pub a: u32
            }
        },
        "MyDef not found",
    );

    assert_code_has(
        &generated,
        &quote! {
            #[derive(Debug, PartialEq, serde :: Deserialize, serde :: Serialize,)]
            pub struct GenericTwoBis {
                pub a: starknet::core::types::Felt,
                #[serde(
                    serialize_with = "cainome::cairo_serde::serialize_as_hex",
                    deserialize_with = "cainome::cairo_serde::deserialize_from_hex"
                )]
                pub b: u64,
                pub c: starknet::core::types::Felt,
                pub d: self::MyDef,
                pub e: Vec<self::MyDef>,
                pub f: self::GenericOneBis
            }
        },
        "GenericTwoBis not found",
    );

    assert_code_has_not(
        &generated,
        &quote! {
            #[derive(Debug, PartialEq, serde :: Deserialize, serde :: Serialize,)]
            pub struct GenericTwo {
                pub a: starknet::core::types::Felt,
                #[serde(
                    serialize_with = "cainome::cairo_serde::serialize_as_hex",
                    deserialize_with = "cainome::cairo_serde::deserialize_from_hex"
                )]
                pub b: u64,
                pub c: starknet::core::types::Felt,
                pub d: self::MyDef,
                pub e: Vec<self::MyDef>,
                pub f: self::GenericOneBis
            }
        },
        "GenericTwo should have been replaced by alias ",
    );

    assert_code_has(
        &generated,
        &quote! {
            #[derive(Debug, PartialEq,)]
            pub struct GenericOneBis {
                pub a: self::MyDef,
                pub b: starknet::core::types::Felt,
                pub c: cainome::cairo_serde::U256
            }
        },
        "asd",
    );
    assert_code_has(
        &generated,
        &quote! {
            #[derive(Debug, PartialEq,)]
            pub struct GenericOneBis {
                pub a: cainome::cairo_serde::U256,
                pub b: starknet::core::types::Felt,
                pub c: cainome::cairo_serde::U256
            }
        },
        "asd",
    );
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

    let expected = quote! {
        #[allow(clippy::ptr_arg)]
        #[allow(clippy::too_many_arguments)]
        pub fn set_tuple_generic(
            &self,
            value: &(cainome::cairo_serde::U256, self::contracts::abicov::structs::ToAlias),
        ) -> starknet::accounts::ExecutionV3<A> {
            let __call = self.set_tuple_generic_getcall(value);
            self.account.execute_v3(vec![__call])
        }
    };

    assert_code_has(&generated, &expected, "No set_tuple_generic function found");
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

    let Token::Struct(s) = &*token.borrow() else {
        unreachable!()
    };

    assert_eq!(s.type_path, "my::Generic::<core::felt252, core::felt252>");
}

#[test]
fn test_substitution_simple_case() {
    let abi = r#"[
        {
            "type": "struct",
            "name": "contracts::abicov::structs::HasSubstitute",
            "members": [
            {
                "name": "a",
                "type": "contracts::abicov::structs::ToAlias"
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
        }
    ]"#;

    let mut substitutions = HashMap::new();

    substitutions.insert(
        String::from("contracts::abicov::structs::ToAlias"),
        String::from("some::module::path::MyDef"),
    );

    let ctx = ExpansionContextFactory::new("MyContract")
        .with_contract_derives(&vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(&vec!["Debug".to_string(), "PartialEq".to_string()])
        .with_execution(ExecutionVersion::V3)
        .with_substitutions(substitutions)
        .with_add_declaration(false)
        .with_add_deployment(false)
        .with_root_module_path("some::module::path")
        .build();

    let entries = AbiParser::parse_abi_string(&abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx));

    let generated = abi_to_tokenstream(&registry.unwrap(), &ctx);

    assert_code_has_not_struct(
        &generated,
        &parse_quote! {
            #[derive(Debug, PartialEq, )]
            pub struct MyDef {
                pub a: u32
            }
        },
        "MyDef found (should not)",
    );

    assert_code_has_struct(
        &generated,
        &parse_quote! {
            #[derive(Debug, PartialEq, )]
            pub struct HasSubstitute {
                pub a: some::module::path::MyDef
            }
        },
        "HasSubstitute not found",
    );
}

#[ignore = "Discuss with glihm, old cainome bahaviour seems problematic"]
#[test]
fn test_substitution_for_generic_case() {
    let abi = r#"[
        {
            "type": "struct",
            "name": "contracts::abicov::structs::HasSubstitute",
            "members": [
                {
                    "name": "a",
                    "type": "contracts::abicov::structs::ToAlias<core::integer::u32>"
                }
            ]
        },
        {
            "type": "struct",
            "name": "contracts::abicov::structs::ToAlias<core::integer::u32>",
            "members": [
                {
                    "name": "a",
                    "type": "core::integer::u32"
                }
            ]
        }
    ]"#;

    let mut substitutions = HashMap::new();

    substitutions.insert(
        String::from("contracts::abicov::structs::ToAlias"),
        String::from("MyDef"),
    );

    let ctx = ExpansionContextFactory::new("MyContract")
        .with_contract_derives(&vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(&vec!["Debug".to_string(), "PartialEq".to_string()])
        .with_execution(ExecutionVersion::V3)
        .with_substitutions(substitutions)
        .with_add_declaration(false)
        .with_add_deployment(false)
        .with_root_module_path("some::module::path")
        .build();

    let entries = AbiParser::parse_abi_string(&abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx));

    let _generated = abi_to_tokenstream(&registry.unwrap(), &ctx);
}
