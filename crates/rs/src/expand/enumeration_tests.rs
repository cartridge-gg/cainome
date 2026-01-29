use cainome_parser::{
    tokens::{Enum, NamedToken, Struct, Token},
    TypeRegistry,
};

use proc_macro2::TokenStream;
use syn::parse_quote;

use crate::expand::{
    for_tests::{assert_code_has, assert_code_has_statement},
    Expandable, ExpansionContextFactory, Module,
};

#[test]
fn test_enum_expand_empty() {
    let registry = TypeRegistry::new();

    let enumeration = Enum::new("my::Enum", &registry).unwrap();

    let ctx = ExpansionContextFactory::new("ContractName").build();

    let generated = Module::new()
        .with_includes(enumeration.expand(&ctx))
        .unwrap()
        .token_stream();

    let expected: TokenStream = parse_quote! {
        pub enum Enum {}
    };

    assert_code_has(&generated, &expected, "Struct not found");
}

#[test]
fn test_enum_expand_simple_variants() {
    let mut registry = TypeRegistry::new();

    let mut enumeration = Enum::new("my::Enum", &registry).unwrap();

    enumeration.variants.push(NamedToken {
        name: "variant1".to_string(),
        token: registry.get("felt").unwrap(),
    });

    let ctx = ExpansionContextFactory::new("ContractName").build();
    registry.apply_substitutions(&ctx.substitutions);

    let generated = Module::new()
        .with_includes(enumeration.expand(&ctx))
        .unwrap()
        .token_stream();

    let expected: TokenStream = parse_quote! {
        pub enum Enum {
            variant1(starknet::core::types::Felt),
        }
    };

    assert_code_has(&generated, &expected, "Struct not found");
}

#[test]
fn test_enum_expand_core_type_variants() {
    let mut registry = TypeRegistry::new();

    let enumeration = Enum::new("my::Enum", &registry)
        .unwrap()
        .with_variants(vec![
            NamedToken::new("variant0", registry.get("felt").unwrap()),
            NamedToken::new("variant1", registry.get("core::felt252").unwrap()),
            NamedToken::new("variant2", registry.get("core::bool").unwrap()),
            NamedToken::new("variant3", registry.get("core::integer::u8").unwrap()),
            NamedToken::new("variant4", registry.get("core::integer::u16").unwrap()),
            NamedToken::new("variant5", registry.get("core::integer::u32").unwrap()),
            NamedToken::new("variant6", registry.get("core::integer::u64").unwrap()),
            NamedToken::new("variant7", registry.get("core::integer::u128").unwrap()),
            NamedToken::new("variant8", registry.get("core::integer::usize").unwrap()),
            NamedToken::new("variant9", registry.get("core::integer::i8").unwrap()),
            NamedToken::new("variant10", registry.get("core::integer::i16").unwrap()),
            NamedToken::new("variant11", registry.get("core::integer::i32").unwrap()),
            NamedToken::new("variant12", registry.get("core::integer::i64").unwrap()),
            NamedToken::new("variant13", registry.get("core::integer::i128").unwrap()),
            NamedToken::new(
                "variant14",
                registry
                    .get("core::starknet::contract_address::ContractAddress")
                    .unwrap(),
            ),
            NamedToken::new(
                "variant15",
                registry
                    .get("core::starknet::class_hash::ClassHash")
                    .unwrap(),
            ),
            NamedToken::new(
                "variant16",
                registry.get("core::bytes_31::bytes31").unwrap(),
            ),
        ]);

    let ctx = ExpansionContextFactory::new("ContractName").build();
    registry.apply_substitutions(&ctx.substitutions);

    let generated = Module::new()
        .with_includes(enumeration.expand(&ctx))
        .unwrap()
        .token_stream();

    let expected = parse_quote! {
        pub enum Enum {
            variant0(starknet::core::types::Felt),
            variant1(starknet::core::types::Felt),
            variant2(bool),
            variant3(u8),
            variant4(u16),
            variant5(u32),
            #[serde(
                serialize_with = "cainome::cairo_serde::serialize_as_hex",
                deserialize_with = "cainome::cairo_serde::deserialize_from_hex"
            )]
            variant6(u64),
            #[serde(
                serialize_with = "cainome::cairo_serde::serialize_as_hex",
                deserialize_with = "cainome::cairo_serde::deserialize_from_hex"
            )]
            variant7(u128),
            variant8(usize),
            variant9(i8),
            variant10(i16),
            variant11(i32),
            #[serde(
                serialize_with = "cainome::cairo_serde::serialize_as_hex",
                deserialize_with = "cainome::cairo_serde::deserialize_from_hex"
            )]
            variant12(i64),
            #[serde(
                serialize_with = "cainome::cairo_serde::serialize_as_hex",
                deserialize_with = "cainome::cairo_serde::deserialize_from_hex"
            )]
            variant13(i128),
            variant14(cainome::cairo_serde::ContractAddress),
            variant15(cainome::cairo_serde::ClassHash),
            variant16(cainome::cairo_serde::Bytes31),
        }
    };

    assert_code_has(&generated, &expected, "Struct not found");
}

#[test]
fn test_enumeration_expand_with_containers_field() {
    let mut registry = TypeRegistry::new();

    let enumeration = Enum::new("my::Type", &registry)
        .unwrap()
        .with_variants(vec![
            NamedToken::new(
                "f1",
                registry
                    .get("core::zeroable::NonZero<core::felt252>")
                    .unwrap(),
            ),
            NamedToken::new("f2", registry.get("core::option::Option<felt>").unwrap()),
            NamedToken::new("f3", registry.get("core::array::Array<felt>").unwrap()),
            NamedToken::new(
                "f4",
                registry
                    .get("(core::felt252, core::option::Option<felt>)")
                    .unwrap(),
            ),
        ]);

    let ctx = ExpansionContextFactory::new("ContractName")
        .with_derives(vec!["Serde", "Clone"])
        .build();

    registry.apply_substitutions(&ctx.substitutions);

    let generated = Module::new()
        .with_includes(enumeration.expand(&ctx))
        .unwrap()
        .token_stream();

    let expected: TokenStream = parse_quote! {
        pub enum Type {
            f1(cainome::cairo_serde::NonZero<starknet::core::types::Felt>),
            f2(Option<starknet::core::types::Felt>),
            f3(Vec<starknet::core::types::Felt>),
            f4(
                (
                    starknet::core::types::Felt,
                    Option<starknet::core::types::Felt>
                )
            ),
        }
    };

    assert_code_has(&generated, &expected, "Struct not found");

    let expected = parse_quote! {
        temp.extend(<(starknet::core::types::Felt,Option::<starknet::core::types::Felt>)>::cairo_serialize(val));
    };

    assert_code_has_statement(&generated, &expected, "Tuple not found");
}

#[test]
fn test_enumeration_expand_with_structure_field() {
    let mut registry = TypeRegistry::new();

    let structure = Struct::new("my::Inner", &registry)
        .unwrap()
        .with_fields(vec![
            NamedToken::new("f1", registry.get("felt").unwrap()),
            NamedToken::new("f2", registry.get("core::bool").unwrap()),
        ]);

    registry.set("my::Inner", Token::Struct(structure.clone()));

    let enumeration = Enum::new("my::Type", &registry)
        .unwrap()
        .with_variants(vec![NamedToken::new(
            "variant",
            registry.get("my::Inner").unwrap(),
        )]);

    let ctx = ExpansionContextFactory::new("ContractName")
        .with_derives(vec!["Serde", "Clone"])
        .build();

    let generated = Module::new()
        .with_includes(structure.expand(&ctx))
        .unwrap()
        .with_includes(enumeration.expand(&ctx))
        .unwrap()
        .token_stream();

    let expected: TokenStream = parse_quote! {
        pub enum Type {
            variant(self::my::Inner)
        }
    };

    assert_code_has(&generated, &expected, "Struct not found");
}
