use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Once;

use crate::expand::for_tests::{
    assert_code_has, assert_code_has_enum, assert_code_has_impl_fn, assert_code_has_not,
    assert_code_has_not_struct, assert_code_has_struct,
};
use crate::expand::generic_resolver::GenericResolverFromMapping;
use crate::{abi_to_tokenstream, expand::ExpansionContextFactory, ExecutionVersion};
use cainome_parser::{tokens::Token, AbiParser, ParserContext};
use quote::quote;
use syn::parse_quote;
use tracing::Level;

static INIT: Once = Once::new();

fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_level(true)
            .with_max_level(Level::TRACE)
            .with_test_writer()
            .init();
    });
}

#[test]
fn test_generic_with_single_argument_expansion() {
    let abi = r#"[
        {
            "type": "struct",
            "name": "MyType::<NestedType>",
            "members": [
                {
                    "name": "alias",
                    "type": "NestedType"
                },
                {
                    "name": "data",
                    "type": "core::felt252"
                }
            ]
        },
        {
            "type": "struct",
            "name": "NestedType",
            "members": [
                {
                    "name": "data",
                    "type": "core::integer::u32"
                }
            ]
        },
        {
            "type": "function",
            "name": "very_cool_function",
            "inputs": [
                {
                    "name": "value",
                    "type": "MyType::<NestedType>"
                }
            ],
            "outputs": [],
            "state_mutability": "external"
        }
    ]"#;

    let ctx = ExpansionContextFactory::new("MyContract")
        .with_contract_derives(vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(vec!["Debug".to_string(), "PartialEq".to_string()])
        .build();

    let entries = AbiParser::parse_abi_string(abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx)).unwrap();

    let generated = abi_to_tokenstream(&registry, &ctx).unwrap();

    assert_code_has(
        &generated,
        &quote! {
            #[derive(Debug, PartialEq)]
            pub struct MyType<A> {
                pub alias: A,
                pub data: starknet::core::types::Felt,
            }
        },
        "No MyType struct found",
    );

    assert_code_has(
        &generated,
        &quote! {
            #[derive(Debug, PartialEq)]
            pub struct NestedType {
                pub data: u32,
            }
        },
        "No MyType struct found",
    );

    assert_code_has_impl_fn(
        &generated,
        &parse_quote! {
            #[allow(clippy::ptr_arg)]
            #[allow(clippy::too_many_arguments)]
            pub fn very_cool_function_getcall(
                &self,
                value: &self::MyType::<self::NestedType>
            ) -> starknet::core::types::Call {
                use cainome::cairo_serde::CairoSerde;
                let mut __calldata = vec![];
                __calldata.extend(self::MyType::<self::NestedType>::cairo_serialize(value));
                starknet::core::types::Call {
                    to: self.address,
                    selector: starknet::macros::selector!("very_cool_function"),
                    calldata: __calldata,
                }
            }
        },
        "No very_cool_function_getcall fn found",
    );

    assert_code_has_impl_fn(
        &generated,
        &parse_quote! {
            #[allow(clippy::ptr_arg)]
            #[allow(clippy::too_many_arguments)]
            pub fn very_cool_function(
                &self,
                value: &self::MyType::<self::NestedType>
            ) -> starknet::accounts::ExecutionV3<A> {
                let __call = self.very_cool_function_getcall(value);
                self.account.execute_v3(vec![__call])
            }
        },
        "No very_cool_function fn found",
    );
}

#[ignore = "Aliasing needs rework."]
#[test]
fn test_2_generic_variants_aliasing() {
    let abi = r#"[
        {
            "type": "struct",
            "name": "MyType::<Variant1>",
            "members": [
                {
                    "name": "alias",
                    "type": "Variant1"
                },
                {
                    "name": "data",
                    "type": "core::felt252"
                }
            ]
        },
        {
            "type": "struct",
            "name": "MyType::<Variant2>",
            "members": [
                {
                    "name": "alias",
                    "type": "Variant2"
                },
                {
                    "name": "data",
                    "type": "core::felt252"
                }
            ]
        },
        {
            "type": "struct",
            "name": "Variant1",
            "members": [
                {
                    "name": "data",
                    "type": "core::integer::u32"
                }
            ]
        },
        {
            "type": "struct",
            "name": "Variant2",
            "members": [
                {
                    "name": "data",
                    "type": "core::integer::u64"
                }
            ]
        },
        {
            "type": "function",
            "name": "set",
            "inputs": [
                {
                    "name": "v1",
                    "type": "MyType::<Variant1>"
                },
                {
                    "name": "v2",
                    "type": "MyType::<Variant2>"
                }

            ],
            "outputs": [],
            "state_mutability": "external"
        }
    ]"#;

    let ctx = ExpansionContextFactory::new("MyContract")
        .with_contract_derives(vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(vec!["Debug".to_string(), "PartialEq".to_string()])
        .build();

    let entries = AbiParser::parse_abi_string(abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx)).unwrap();

    let generated = abi_to_tokenstream(&registry, &ctx).unwrap();

    assert_code_has_struct(
        &generated,
        &parse_quote! {
            #[derive(Debug, PartialEq, )]
            pub struct MyTypeVariant1 {
                pub alias: self::Variant1,
                pub data: starknet::core::types::Felt
            }
        },
        "MyTypeVariant1 not found",
    );
    assert_code_has_struct(
        &generated,
        &parse_quote! {
            #[derive(Debug, PartialEq, )]
            pub struct MyTypeVariant2 {
                pub alias: self::Variant2,
                pub data: starknet::core::types::Felt
            }
        },
        "MyTypeVariant1 not found",
    );

    assert_code_has_impl_fn(
        &generated,
        &parse_quote! {
            #[allow(clippy::ptr_arg)]
            #[allow(clippy::too_many_arguments)]
            pub fn set_getcall(
                &self,
                v1: &self::MyTypeVariant1,
                v2: &self::MyTypeVariant2
            ) -> starknet::core::types::Call {
                use cainome::cairo_serde::CairoSerde;
                let mut __calldata = vec![];
                __calldata.extend(self::MyTypeVariant1::cairo_serialize(v1));
                __calldata.extend(self::MyTypeVariant2::cairo_serialize(v2));
                starknet::core::types::Call {
                    to: self.address,
                    selector: starknet::macros::selector!("set"),
                    calldata: __calldata,
                }
            }
        },
        "set_getcall not found",
    );
}

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
        .with_contract_derives(vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(vec!["Debug".to_string(), "PartialEq".to_string()])
        .with_execution(ExecutionVersion::V3)
        .with_aliases(aliases)
        .build();

    let entries = AbiParser::parse_abi_string(abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx));

    let generated = abi_to_tokenstream(&registry.unwrap(), &ctx).unwrap();

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
        .with_contract_derives(vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(vec!["Debug".to_string(), "PartialEq".to_string()])
        .build();

    let entries = AbiParser::parse_abi_string(abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx)).unwrap();

    let generated = abi_to_tokenstream(&registry, &ctx).unwrap();

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
        .with_contract_derives(vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(vec!["Debug".to_string(), "PartialEq".to_string()])
        .build();

    let entries = AbiParser::parse_abi_string(abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx)).unwrap();

    let generated = abi_to_tokenstream(&registry, &ctx).unwrap();

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
        .with_contract_derives(vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(vec!["Debug".to_string(), "PartialEq".to_string()])
        .with_aliases(aliases)
        .build();

    let entries = AbiParser::parse_abi_string(abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx)).unwrap();

    let generated = abi_to_tokenstream(&registry, &ctx).unwrap();

    let expected = quote! {};

    assert_code_has(&generated, &expected, "asd");
}

#[test]
fn test_tuple_with_generic_arg_with_2_parameters_resolves() {
    let abi_json = r#"[
            {
                "type": "struct",
                "name": "my::Generic::<core::felt252, core::felt252>", 
                "members": [
                    {
                        "name": "f1",
                        "type": "core::felt252"
                    },
                    {
                        "name": "f2",
                        "type": "core::felt252"
                    }
                ]
            },
            {
                "type": "struct",
                "name": "my::New", 
                "members": [
                    {
                        "name": "f1",
                        "type": "(my::Generic::<core::felt252, core::felt252>)"
                    }
                ]
            }

        ]"#;

    let ctx = ParserContext::new();

    let abi_entries = AbiParser::parse_abi_string(abi_json).unwrap();

    let mut registry = AbiParser::build_registry(abi_entries, ctx).unwrap();

    let ctx = ExpansionContextFactory::new("MyContract").build();

    registry.apply_substitutions(&ctx.substitutions);

    let token = registry
        .get("my::Generic::<core::felt252, core::felt252>")
        .unwrap();

    let Token::Struct(s) = &*token.borrow() else {
        unreachable!()
    };

    // s.to_rust_type_path(&ctx);

    assert_eq!(s.type_path, "my::Generic");
    assert!(s.is_generic());
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
        .with_contract_derives(vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(vec!["Debug".to_string(), "PartialEq".to_string()])
        .with_execution(ExecutionVersion::V3)
        .with_substitutions(substitutions)
        .with_add_declaration(false)
        .with_add_deployment(false)
        .with_root_module_path("some::module::path")
        .build();

    let entries = AbiParser::parse_abi_string(abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx));

    let generated = abi_to_tokenstream(&registry.unwrap(), &ctx).unwrap();

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
        .with_contract_derives(vec!["Debug".to_string(), "Clone".to_string()])
        .with_derives(vec!["Debug".to_string(), "PartialEq".to_string()])
        .with_execution(ExecutionVersion::V3)
        .with_substitutions(substitutions)
        .with_add_declaration(false)
        .with_add_deployment(false)
        .with_root_module_path("some::module::path")
        .build();

    let entries = AbiParser::parse_abi_string(abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx));

    let _generated = abi_to_tokenstream(&registry.unwrap(), &ctx);
}

#[test]
fn test_simple_generic_rendering_resolves_easy_struct() {
    init_tracing();

    let abi = r#"[
        {
            "type": "struct",
            "name": "my::GenericVar::<core::felt252>",
            "members": [
                {
                    "name": "a",
                    "type": "core::felt252"
                }
            ]
        },
        {
            "type": "struct",
            "name": "my::GenericVar::<core::integer::u32>",
            "members": [
                {
                    "name": "a",
                    "type": "core::integer::u32"
                }
            ]
        }
    ]"#;

    let ctx = ExpansionContextFactory::new("MyContract")
        .with_add_declaration(false)
        .with_add_deployment(false)
        .with_root_module_path("root")
        .build();

    let entries = AbiParser::parse_abi_string(abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx));

    let generated = abi_to_tokenstream(&registry.unwrap(), &ctx);

    assert_code_has_struct(
        &generated.unwrap(),
        &parse_quote! {
            pub struct GenericVar<A> {
                pub a: A
            }
        },
        "GenericVar<A> not found",
    );
}

#[test]
fn test_simple_generic_fucntion() {
    init_tracing();

    let abi = r#"[
        {
            "type": "struct",
            "name": "my::GenericVar::<core::felt252>",
            "members": [
                {
                    "name": "a",
                    "type": "core::felt252"
                }
            ]
        },
        {
            "type": "struct",
            "name": "my::GenericVar::<core::integer::u32>",
            "members": [
                {
                    "name": "a",
                    "type": "core::integer::u32"
                }
            ]
        },
        {
            "type": "function",
            "name": "my_func",
            "inputs": [
                {
                    "name": "arg",
                    "type": "my::GenericVar::<core::integer::u32>"
                }
            ],
            "outputs": [
                {
                    "type": "felt"
                }
            ],
            "state_mutability": "external"  
        }
    ]"#;

    let ctx = ExpansionContextFactory::new("MyContract")
        .with_add_declaration(false)
        .with_add_deployment(false)
        .with_root_module_path("root")
        .build();

    let entries = AbiParser::parse_abi_string(abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx));

    let generated = abi_to_tokenstream(&registry.unwrap(), &ctx);

    assert_code_has_impl_fn(
        &generated.unwrap(),
        &parse_quote! {
            #[allow(clippy::ptr_arg)]
            #[allow(clippy::too_many_arguments)]
            pub fn my_func(&self, arg: &root::my::GenericVar::<u32>) -> starknet::accounts::ExecutionV3<A> {
                let __call = self.my_func_getcall (arg);
                self.account.execute_v3(vec![__call])
            }
        },
        "GenericVar<A> not found",
    );
}

#[test]
fn test_nested_generic_field() {
    init_tracing();

    let abi = r#"[
        {
            "type": "struct",
            "name": "my::Nested::<core::felt252>",
            "members": [
                {
                    "name": "a",
                    "type": "core::felt252"
                }
            ]
        },
        {
            "type": "struct",
            "name": "my::GenericVar::<core::felt252>",
            "members": [
                {
                    "name": "a",
                    "type": "core::felt252"
                },
                {
                    "name": "nested",
                    "type": "my::Nested::<core::felt252>"
                }
            ]
        },
        {
            "type": "struct",
            "name": "my::GenericVar::<core::integer::u32>",
            "members": [
                {
                    "name": "a",
                    "type": "core::integer::u32"
                },
                {
                    "name": "nested",
                    "type": "my::Nested::<core::felt252>"
                }
            ]
        }
    ]"#;

    let ctx = ExpansionContextFactory::new("MyContract")
        .with_add_declaration(false)
        .with_add_deployment(false)
        .with_root_module_path("root")
        .build();

    let entries = AbiParser::parse_abi_string(abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx));

    let generated = abi_to_tokenstream(&registry.unwrap(), &ctx).unwrap();

    assert_code_has_struct(
        &generated.clone(),
        &parse_quote! {
            pub struct GenericVar<A> {
                pub a: A,
                pub nested: root::my::Nested::<starknet::core::types::Felt>
            }
        },
        "GenericVar<A> not found",
    );
}

#[test]
fn test_nested_generic_field_in_enumeration() {
    init_tracing();

    let abi = r#"[
        {
            "type": "enum",
            "name": "my::Nested::<core::felt252>",
            "variants": [
                {
                    "name": "One",
                    "type": "core::felt252"
                },
                {
                    "name": "Two",
                    "type": "(core::felt252, core::felt252)"
                }
            ]
        },
        {
            "type": "struct",
            "name": "my::Var",
            "members": [
                {
                    "name": "a",
                    "type": "core::felt252"
                },
                {
                    "name": "nested",
                    "type": "my::Nested::<core::felt252>"
                }
            ]
        }
    ]"#;

    let ctx = ExpansionContextFactory::new("MyContract")
        .with_add_declaration(false)
        .with_add_deployment(false)
        .with_root_module_path("root")
        .build();

    let entries = AbiParser::parse_abi_string(abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx));

    let generated = abi_to_tokenstream(&registry.unwrap(), &ctx).unwrap();

    assert_code_has_struct(
        &generated.clone(),
        &parse_quote! {
            pub struct Var {
                pub a: starknet::core::types::Felt,
                pub nested: root::my::Nested::<starknet::core::types::Felt>
            }
        },
        "Var not found",
    );

    assert_code_has_enum(
        &generated.clone(),
        &parse_quote! {
            pub enum Nested<A> {
                One(A),
                Two((starknet::core::types::Felt, starknet::core::types::Felt))
            }
        },
        "Nested<A> not found",
    )
}

#[test]
fn test_generic_resolver() {
    init_tracing();
    let abi = r#"[
        {
            "type": "struct",
            "name": "my::Var<core::felt252>",
            "members": [
                {
                    "name": "a",
                    "type": "core::felt252"
                },
                {
                    "name": "b",
                    "type": "core::felt252"
                }
            ]
        }
    ]"#;

    let resolver = GenericResolverFromMapping::new(vec![("my::Var", "b", "A")]);

    let ctx = ExpansionContextFactory::new("MyContract")
        .with_add_declaration(false)
        .with_add_deployment(false)
        .with_root_module_path("root")
        .with_generic_resolver(Rc::new(resolver))
        .build();

    let entries = AbiParser::parse_abi_string(abi).unwrap();

    let registry = AbiParser::build_registry(entries, ParserContext::from(&ctx));

    let generated = abi_to_tokenstream(&registry.unwrap(), &ctx).unwrap();

    assert_code_has_struct(
        &generated.clone(),
        &parse_quote! {
            pub struct Var<A> {
                pub a: starknet::core::types::Felt,
                pub b: A
            }
        },
        "Var not found",
    );
}
