use cainome_parser::{
    tokens::{NamedToken, Struct, Token},
    TypeRegistry,
};

use proc_macro2::TokenStream;
use syn::parse_quote;

use crate::expand::{for_tests::assert_code_has, Expandable, ExpansionContextFactory, Module};

#[test]
fn test_structure_expand_empty() {
    let mut registry = TypeRegistry::new();

    let structure = Struct::new("my::Type", &registry).unwrap();

    let ctx = ExpansionContextFactory::new("ContractName").build();

    registry.apply_substitutions(&ctx.substitutions);

    let generated = Module::new()
        .with_includes(structure.expand(&ctx))
        .token_stream();

    let expected = parse_quote! {
        pub struct Type {}
    };

    assert_code_has(&generated, &expected, "Struct not found");
}

#[test]
fn test_structure_expand_basic_field() {
    let mut registry = TypeRegistry::new();

    let mut structure = Struct::new("my::Type", &registry).unwrap();

    structure.fields.push(NamedToken {
        name: "f1".to_string(),
        token: registry.get("felt").unwrap(),
    });

    let ctx = ExpansionContextFactory::new("ContractName").build();
    registry.apply_substitutions(&ctx.substitutions);

    let generated = Module::new()
        .with_includes(structure.expand(&ctx))
        .token_stream();

    let expected = parse_quote! {
        pub struct Type {
            pub f1: starknet::core::types::Felt
        }
    };

    assert_code_has(&generated, &expected, "Struct not found");
}

#[test]
fn test_structure_expand_with_derive() {
    let mut registry = TypeRegistry::new();

    let mut structure = Struct::new("my::Type", &registry).unwrap();

    structure.fields.push(NamedToken {
        name: "f1".to_string(),
        token: registry.get("felt").unwrap(),
    });

    let ctx = ExpansionContextFactory::new("ContractName")
        .with_derives(vec!["serde::Serialize", "serde::Deserialize", "Clone"])
        .build();
    registry.apply_substitutions(&ctx.substitutions);

    let generated = Module::new()
        .with_includes(structure.expand(&ctx))
        .token_stream();

    let expected = parse_quote! {
        #[derive(Clone, serde::Deserialize, serde::Serialize,)]
        pub struct Type {
            pub f1: starknet::core::types::Felt
        }
    };

    assert_code_has(&generated, &expected, "Struct not found");
}

#[test]
fn test_structure_expand_with_option_field() {
    let mut registry = TypeRegistry::new();

    let mut structure = Struct::new("my::Type", &registry).unwrap();

    structure.fields.push(NamedToken {
        name: "f1".to_string(),
        token: registry.get("core::option::Option<felt>").unwrap(),
    });

    let ctx = ExpansionContextFactory::new("ContractName")
        .with_derives(vec!["Serde", "Clone"])
        .build();
    registry.apply_substitutions(&ctx.substitutions);

    let generated = Module::new()
        .with_includes(structure.expand(&ctx))
        .token_stream();

    let expected = parse_quote! {
        #[derive(Clone, serde::Deserialize, serde::Serialize,)]
        pub struct Type {
            pub f1: Option<starknet::core::types::Felt>
        }
    };

    assert_code_has(&generated, &expected, "Struct not found");
}

#[test]
fn test_structure_expand_with_array_field() {
    let mut registry = TypeRegistry::new();

    let mut structure = Struct::new("my::Type", &registry).unwrap();

    structure.fields.push(NamedToken {
        name: "f1".to_string(),
        token: registry.get("core::array::Array<core::felt252>").unwrap(),
    });

    let ctx = ExpansionContextFactory::new("ContractName")
        .with_derives(vec!["serde::Serialize", "serde::Deserialize", "Clone"])
        .build();
    registry.apply_substitutions(&ctx.substitutions);

    let generated = Module::new()
        .with_includes(structure.expand(&ctx))
        .token_stream();

    let expected = parse_quote! {
        #[derive(Clone, serde::Deserialize, serde::Serialize,)]
        pub struct Type {
            pub f1: Vec<starknet::core::types::Felt>
        }
    };

    assert_code_has(&generated, &expected, "Struct not found");
}

#[test]
fn test_structure_expand_with_non_zero_field() {
    let mut registry = TypeRegistry::new();

    let mut structure = Struct::new("my::Type", &registry).unwrap();

    structure.fields.push(NamedToken {
        name: "f1".to_string(),
        token: registry
            .get("core::zeroable::NonZero<core::felt252>")
            .unwrap(),
    });

    let ctx = ExpansionContextFactory::new("ContractName")
        .with_derives(vec!["Serde", "Clone"])
        .build();
    registry.apply_substitutions(&ctx.substitutions);

    let generated = Module::new()
        .with_includes(structure.expand(&ctx))
        .token_stream();

    let expected: TokenStream = parse_quote! {
        #[derive(Clone, serde::Deserialize, serde::Serialize,)]
        pub struct Type {
            pub f1: cainome::cairo_serde::NonZero<starknet::core::types::Felt>
        }
    };

    assert_code_has(&generated, &expected, "Struct not found");
}

#[test]
fn test_structure_expand_with_tuple_field() {
    let mut registry = TypeRegistry::new();
    let ctx = ExpansionContextFactory::new("ContractName")
        .with_derives(vec!["Serde", "Clone"])
        .build();
    registry.apply_substitutions(&ctx.substitutions);

    let mut structure = Struct::new("my::Type", &registry).unwrap();

    structure.fields.push(NamedToken {
        name: "f1".to_string(),
        token: registry
            .get("(core::felt252, core::option::Option<felt>)")
            .unwrap(),
    });

    let generated = Module::new()
        .with_includes(structure.expand(&ctx))
        .token_stream();

    let expected = parse_quote! {
        #[derive(Clone, serde::Deserialize, serde::Serialize)]
        pub struct Type {
            pub f1: (starknet::core::types::Felt, Option<starknet::core::types::Felt>),
        }
    };

    assert_code_has(&generated, &expected, "Struct not found");
}

#[test]
fn test_structure_expand_with_self_reference() {
    let structure = {
        let mut registry = TypeRegistry::new();

        // Prepare placeholder
        registry.set("my::Type", Token::Placeholder);
        // Construct type
        let mut structure = Struct::new("my::Type", &registry).unwrap();
        structure.fields.push(NamedToken {
            name: "f1".to_string(),
            token: registry.get("my::Type").unwrap(),
        });
        // Update placeholder
        registry.set("my::Type", Token::Struct(structure.clone()));

        structure
    };

    let ctx = ExpansionContextFactory::new("ContractName")
        .with_root_module_path("crate")
        .with_derives(vec!["Serde", "Clone"])
        .build();

    let generated = Module::new()
        .with_includes(structure.expand(&ctx))
        .token_stream();

    // TODO(@baitcode): This is incorrect. Should be Box<> or something.
    // Discuss with @glihm

    let expected = parse_quote! {
        #[derive(Clone, serde::Deserialize, serde::Serialize,)]
        pub struct Type {
            pub f1: crate::my::Type
        }
    };

    assert_code_has(&generated, &expected, "Struct not found");
}

#[test]
fn test_structure_expand_all_core_types() {
    let mut registry = TypeRegistry::new();
    let ctx = ExpansionContextFactory::new("ContractName").build();
    registry.apply_substitutions(&ctx.substitutions);

    let structure = Struct::new("my::Struct", &registry)
        .unwrap()
        .with_fields(vec![
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

    let generated = Module::new()
        .with_includes(structure.expand(&ctx))
        .token_stream();

    let expected: TokenStream = parse_quote! {
        pub struct Struct {
            pub variant0: starknet::core::types::Felt,
            pub variant1: starknet::core::types::Felt,
            pub variant2: bool,
            pub variant3: u8,
            pub variant4: u16,
            pub variant5: u32,
            #[serde(
                serialize_with = "cainome::cairo_serde::serialize_as_hex",
                deserialize_with = "cainome::cairo_serde::deserialize_from_hex"
            )]
            pub variant6: u64,
            #[serde(
                serialize_with = "cainome::cairo_serde::serialize_as_hex",
                deserialize_with = "cainome::cairo_serde::deserialize_from_hex"
            )]
            pub variant7: u128,
            pub variant8: usize,
            pub variant9: i8,
            pub variant10: i16,
            pub variant11: i32,
            #[serde(
                serialize_with = "cainome::cairo_serde::serialize_as_hex",
                deserialize_with = "cainome::cairo_serde::deserialize_from_hex"
            )]
            pub variant12: i64,
            #[serde(
                serialize_with = "cainome::cairo_serde::serialize_as_hex",
                deserialize_with = "cainome::cairo_serde::deserialize_from_hex"
            )]
            pub variant13: i128,
            pub variant14: cainome::cairo_serde::ContractAddress,
            pub variant15: cainome::cairo_serde::ClassHash,
            pub variant16: cainome::cairo_serde::Bytes31
        }
    };

    assert_code_has(&generated, &expected, "Struct not found");
}

#[test]
fn structure_with_fields_conflicting_with_keywords() {
    let mut registry = TypeRegistry::new();

    let structure = Struct::new("my::Type", &registry)
        .unwrap()
        .with_fields(vec![
            NamedToken::new("type", registry.get("felt").unwrap()),
            NamedToken::new("match", registry.get("felt").unwrap()),
            NamedToken::new("move", registry.get("felt").unwrap()),
            NamedToken::new("final", registry.get("felt").unwrap()),
        ]);

    let ctx = ExpansionContextFactory::new("ContractName").build();
    registry.apply_substitutions(&ctx.substitutions);

    let generated = Module::new()
        .with_includes(structure.expand(&ctx))
        .token_stream();

    let expected = parse_quote! {
        pub struct Type {
            pub r#type: starknet::core::types::Felt,
            pub r#match: starknet::core::types::Felt,
            pub r#move: starknet::core::types::Felt,
            pub r#final: starknet::core::types::Felt
        }
    };

    assert_code_has(&generated, &expected, "Struct not found");
}
