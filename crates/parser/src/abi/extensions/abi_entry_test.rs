use crate::{tokens::Token, AbiParser};
use std::collections::HashMap;

#[test]
fn test_enum_variant_composite_inner_kind() {
    // Test ABI with enum variants - some with data, some without
    let abi_json = r#"
        [
            {
                "type": "enum",
                "name": "test::TestEnum",
                "variants": [
                    {
                        "name": "VariantWithoutData",
                        "type": "()"
                    },
                    {
                        "name": "VariantWithFelt252",
                        "type": "core::felt252"
                    },
                    {
                        "name": "VariantWithTuple",
                        "type": "(core::felt252, core::integer::u32)"
                    }
                ]
            }
        ]   
        "#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();

    assert_eq!(result.enums.len(), 1);
    let Token::Enum(enum_token) = &*result.enums[0].borrow() else {
        panic!("Should be enum");
    };

    assert_eq!(enum_token.type_path, "test::TestEnum");
    assert_eq!(enum_token.variants.len(), 3);

    // Check that variant without data has NotUsed kind
    assert_eq!(enum_token.variants[0].name, "VariantWithoutData");
    let Token::Basic(f1t) = &*enum_token.variants[0].token.borrow() else {
        panic!("First field token should be basic");
    };
    assert_eq!(f1t.type_path, "()");

    // Check that variant with felt252 has Data kind
    assert_eq!(enum_token.variants[1].name, "VariantWithFelt252");
    let Token::Basic(f2t) = &*enum_token.variants[1].token.borrow() else {
        panic!("Second field token should be basic");
    };
    assert_eq!(f2t.type_path, "core::felt252");

    // Check that variant with tuple has Data kind
    assert_eq!(enum_token.variants[2].name, "VariantWithTuple");
    let Token::Tuple(f3t) = &*enum_token.variants[2].token.borrow() else {
        panic!("Third field token should be basic");
    };
    assert_eq!(f3t.type_path, "(core::felt252,core::integer::u32)");
}
