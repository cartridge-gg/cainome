use std::{collections::HashMap, rc::Rc};

use starknet::core::types::contract::SierraClass;

use crate::{
    tokens::{EventKind, StateMutability, Token},
    AbiParser, ParserContext,
};

#[test]
fn recursive_struct_parsing() {
    let abi_json = r#"[
            {
                "type": "struct",
                "name": "baitcode::TreeNode",
                "members": [
                    {
                        "name": "parent",
                        "type": "core::option::Option::<baitcode::TreeNode>"
                    },
                    {
                        "name": "children",
                        "type": "core::array::Array::<baitcode::TreeNode>"
                    }   
                ]
            }
        ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();

    assert_eq!(result.structs.len(), 1);
    assert_eq!(result.interfaces.len(), 0);
    assert_eq!(result.events.len(), 0);
    assert_eq!(result.functions.len(), 0);
    assert_eq!(result.enums.len(), 0);

    let Some(token) = result.structs.iter().next() else {
        panic!("At least one element should be present in structs");
    };

    let Token::Struct(s1) = &*token.borrow() else {
        panic!("Only element parsed from ABI should be Token::Struct");
    };

    for field in s1.fields.iter() {
        if field.name == "parent" {
            let Token::Option(parent) = &*field.token.borrow() else {
                panic!("Parent field is optional")
            };
            assert!(Rc::ptr_eq(token, &parent.inner));
        }

        if field.name == "children" {
            let Token::Array(children) = &*field.token.borrow() else {
                panic!("Children field is optional")
            };
            assert!(Rc::ptr_eq(token, &children.inner));
        }
    }
}

#[test]
fn indirect_recursion_struct_parsing() {
    let abi_json = r#"[
            {
                "type": "struct",
                "name": "baitcode::S1",
                "members": [
                    {
                        "name": "parent",
                        "type": "core::option::Option::<baitcode::S2>"
                    }
                ]
            },
            {
                "type": "struct",
                "name": "baitcode::S2",
                "members": [
                    {
                        "name": "parent",
                        "type": "core::option::Option::<baitcode::S1>"
                    }
                ]
            }
        ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();

    assert_eq!(result.structs.len(), 2);
    assert_eq!(result.interfaces.len(), 0);
    assert_eq!(result.events.len(), 0);
    assert_eq!(result.functions.len(), 0);
    assert_eq!(result.enums.len(), 0);
}

#[test]
fn recursive_enum_parsing() {
    let abi_json = r#"[
            {
                "type": "struct",
                "name": "baitcode::TreeNode",
                "members": [
                    {
                        "name": "parent",
                        "type": "core::option::Option::<baitcode::TreeNode>"
                    },
                    {
                        "name": "children",
                        "type": "core::array::Array::<baitcode::TreeNode>"
                    }   
                ]
            }
        ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();

    assert_eq!(result.structs.len(), 1);
    assert_eq!(result.interfaces.len(), 0);
    assert_eq!(result.events.len(), 0);
    assert_eq!(result.functions.len(), 0);
    assert_eq!(result.enums.len(), 0);

    let Some(token) = result.structs.iter().next() else {
        panic!("At least one element should be present in structs");
    };

    let Token::Struct(s1) = &*token.borrow() else {
        panic!("Only element parsed from ABI should be Token::Struct");
    };

    for field in s1.fields.iter() {
        if field.name == "parent" {
            let Token::Option(parent) = &*field.token.borrow() else {
                panic!("Parent field is optional")
            };
            assert!(Rc::ptr_eq(token, &parent.inner));
        }

        if field.name == "children" {
            let Token::Array(children) = &*field.token.borrow() else {
                panic!("Children field is optional")
            };
            assert!(Rc::ptr_eq(token, &children.inner));
        }
    }
}

#[test]
fn test_parsing_all_core_type_struct_fields() {
    let core_fields = vec![
        ("m1", "core::integer::u128"),
        ("m2", "felt"),
        ("m3", "core::felt252"),
        ("m4", "core::bool"),
        ("m5", "core::integer::u8"),
        ("m6", "core::integer::u16"),
        ("m7", "core::integer::u32"),
        ("m8", "core::integer::u64"),
        ("m9", "core::integer::u128"),
        ("m10", "core::integer::usize"),
        ("m11", "core::integer::i8"),
        ("m12", "core::integer::i16"),
        ("m13", "core::integer::i32"),
        ("m14", "core::integer::i64"),
        ("m15", "core::integer::i128"),
        ("m16", "core::starknet::contract_address::ContractAddress"),
        ("m17", "core::starknet::class_hash::ClassHash"),
        // TODO: Is this array?
        ("m18", "core::bytes_31::bytes31"),
    ];

    let mut members = vec![];
    for (name, ttype) in core_fields.iter() {
        members.push(format!(r#"{{"name": "{}", "type": "{}"}}"#, name, ttype));
    }

    let abi_json = format!(
        r#"[
                {{
                    "type": "struct",
                    "name": "my::package::AllInOne",
                    "members": [{}]
                }}
            ]"#,
        members.join(",")
    );

    let result = AbiParser::tokens_from_abi_string(&abi_json, HashMap::new()).unwrap();

    assert_eq!(result.structs.len(), 1);

    let Some(token) = result.structs.iter().next() else {
        panic!("At least one element should be present in structs");
    };

    let Token::Struct(s1) = &*token.borrow() else {
        panic!("Only element parsed from ABI should be Token::Struct");
    };

    assert_eq!(s1.fields.len(), core_fields.len());

    for (idx, (name, ttype)) in core_fields.iter().enumerate() {
        assert_eq!(s1.fields[idx].name, name.to_string());
        let Token::Basic(f1) = &*s1.fields[idx].token.as_ref().borrow() else {
            panic!("Only element parsed from ABI should be Token::Struct");
        };
        assert_eq!(ttype.to_string(), f1.type_path);
    }
}

#[test]
fn check_array_container_is_parsed() {
    let abi_json = r#"[
                {
                    "type": "struct",
                    "name": "my::package::AllInOne",
                    "members": [
                        {
                            "name": "f1",
                            "type": "core::array::Span::<felt>"
                        },
                        {
                            "name": "f2",
                            "type": "core::array::Array::<felt>"
                        }
                    ]
                }
            ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();
    assert_eq!(result.structs.len(), 1);

    let Some(token) = result.structs.iter().next() else {
        panic!("At least one element should be present in structs");
    };

    let Token::Struct(s1) = &*token.borrow() else {
        panic!("Only element parsed from ABI should be Token::Struct");
    };

    assert_eq!(s1.fields.len(), 2);

    let f1_inner = s1.fields[0].clone();
    let f2_inner = s1.fields[1].clone();

    let Token::Array(a1) = &*f1_inner.token.borrow() else {
        panic!("First field should be Array");
    };
    let Token::Array(a2) = &*f2_inner.token.borrow() else {
        panic!("Second field should be Array");
    };

    let Token::Basic(c1) = &*a1.inner.borrow() else {
        panic!("Array content should be CoreBasic");
    };
    assert_eq!(c1.type_path, "felt");
    let Token::Basic(c2) = &*a2.inner.borrow() else {
        panic!("Array content should be CoreBasic");
    };
    assert_eq!(c2.type_path, "felt");
}

#[test]
fn check_tuple_container_is_parsed() {
    let abi_json = r#"[
                {
                    "type": "struct",
                    "name": "my::package::AllInOne",
                    "members": [
                        {
                            "name": "f1",
                            "type": "(felt, core::integer::i32)"
                        }
                    ]
                }
            ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();
    assert_eq!(result.structs.len(), 1);

    let Some(token) = result.structs.iter().next() else {
        panic!("At least one element should be present in structs");
    };

    let Token::Struct(s1) = &*token.borrow() else {
        panic!("Only element parsed from ABI should be Token::Struct");
    };

    assert_eq!(s1.fields.len(), 1);

    let f1_inner = s1.fields[0].clone();

    let Token::Tuple(a1) = &*f1_inner.token.borrow() else {
        panic!("First field should be Tuple");
    };

    let Token::Basic(t1) = &*a1.inners[0].borrow() else {
        panic!("First tuple element should be CoreBasic");
    };
    let Token::Basic(t2) = &*a1.inners[1].borrow() else {
        panic!("Second tuple element should be CoreBasic");
    };

    assert_eq!(t1.type_path, "felt");
    assert_eq!(t2.type_path, "core::integer::i32");
}

#[test]
fn check_nested_tuple_container_is_parsed() {
    let abi_json = r#"[
                {
                    "type": "struct",
                    "name": "my::package::AllInOne",
                    "members": [
                        {
                            "name": "f1",
                            "type": "(felt, (felt, core::integer::i32))"
                        }
                    ]
                }
            ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();
    assert_eq!(result.structs.len(), 1);

    let Some(token) = result.structs.iter().next() else {
        panic!("At least one element should be present in structs");
    };

    let Token::Struct(s1) = &*token.borrow() else {
        panic!("Only element parsed from ABI should be Token::Struct");
    };

    assert_eq!(s1.fields.len(), 1);

    let f1_inner = s1.fields[0].clone();

    let Token::Tuple(a1) = &*f1_inner.token.as_ref().borrow() else {
        panic!("First field should be Tuple");
    };

    let Token::Basic(t1) = &*a1.inners[0].borrow() else {
        panic!("First tuple element should be CoreBasic");
    };

    assert_eq!(t1.type_path, "felt");

    let Token::Tuple(t2) = &*a1.inners[1].borrow() else {
        panic!("Second tuple element should be Tuple");
    };

    let Token::Basic(nested_t1) = &*t2.inners[0].borrow() else {
        panic!("First nested tuple element should be CoreBasic");
    };
    let Token::Basic(nested_t2) = &*t2.inners[1].borrow() else {
        panic!("Second nested tuple element should be CoreBasic");
    };

    assert_eq!(nested_t1.type_path, "felt");
    assert_eq!(nested_t2.type_path, "core::integer::i32");
}

#[test]
fn check_bool_enum_is_ignored_on_parse() {
    let abi_json = r#"[
                {
                    "name": "core::bool",
                    "type": "enum",
                    "variants": [
                        {
                            "name": "False",
                            "type": "()"
                        },
                        {
                            "name": "True",
                            "type": "()"
                        }
                    ]
                }
            ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();
    assert_eq!(result.structs.len(), 0);
    assert_eq!(result.enums.len(), 0);
}

#[test]
fn check_basic_enum_is_parsed() {
    let abi_json = r#"[
                {
                    "name": "contracts::abicov::enums::MixedEnum",
                    "type": "enum",
                    "variants": [
                        {
                            "name": "Variant1",
                            "type": "core::felt252"
                        },
                        {
                            "name": "Variant2",
                            "type": "()"
                        }
                    ]
                }
            ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();
    assert_eq!(result.enums.len(), 1);

    let Some(token) = result.enums.iter().next() else {
        panic!("At least one element should be present in enums");
    };

    let Token::Enum(s1) = &*token.borrow() else {
        panic!("Only element parsed from ABI should be Token::Enum");
    };

    assert_eq!(s1.variants.len(), 2);

    let f1_inner = s1.variants[0].clone();
    let f2_inner = s1.variants[1].clone();

    let Token::Basic(a1) = &*f1_inner.token.borrow() else {
        panic!("First variant should be CoreBasic");
    };
    let Token::Basic(a2) = &*f2_inner.token.borrow() else {
        panic!("Second variant should be CoreBasic");
    };

    assert_eq!(a1.type_path, "core::felt252");
    assert_eq!(a2.type_path, "()");
}

#[test]
fn check_complex_enum_is_parsed() {
    let abi_json = r#"[
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
                    "name": "contracts::abicov::enums::TypedEnum",
                    "type": "enum",
                    "variants": [
                        {
                            "name": "Variant1",
                            "type": "core::felt252"
                        },
                        {
                            "name": "Variant2",
                            "type": "core::integer::u256"
                        },
                        {
                            "name": "Variant3",
                            "type": "(core::felt252, core::integer::u256)"
                        },
                        {
                            "name": "Variant4",
                            "type": "core::starknet::contract_address::ContractAddress"
                        },
                        {
                            "name": "Variant5",
                            "type": "contracts::abicov::enums::Simple"
                        },
                        {
                            "name": "Variant6",
                            "type": "contracts::abicov::enums::StructWithStruct"
                        }
                    ]
                },
                {
                    "type": "struct",
                    "name": "contracts::abicov::enums::StructWithStruct",
                    "members": [
                        {
                            "name": "simple",
                            "type": "contracts::abicov::enums::Simple"
                        }
                    ]
                },
                {
                    "type": "struct",
                    "name": "contracts::abicov::enums::Simple",
                    "members": [
                        {
                            "name": "span",
                            "type": "felt"
                        }
                    ]
                }
            ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();
    assert_eq!(result.enums.len(), 1);
    assert_eq!(result.structs.len(), 3);
}

#[test]
fn test_abi_struct_with_link_to_other_struct_and_nonzero_container() {
    let abi_json = r#"
        [
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
                "name": "package::StructOne",
                "members": [
                    {
                        "name": "a",
                        "type": "core::integer::u64"
                    },
                    {
                        "name": "b",
                        "type": "core::zeroable::NonZero<core::felt252>"
                    },
                    {
                        "name": "c",
                        "type": "core::integer::u256"
                    }
                ]
            }
        ]
        "#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();

    assert_eq!(result.structs.len(), 2);
    assert_eq!(result.interfaces.len(), 0);
    assert_eq!(result.events.len(), 0);
    assert_eq!(result.functions.len(), 0);
    assert_eq!(result.enums.len(), 0);
}

#[test]
fn test_option_is_resolved_as_a_part_of_struct() {
    let abi_json = r#"[
                {
                    "name": "baitcode::TestStruct",
                    "type": "struct",
                    "members": [
                        {
                            "name": "option",
                            "type": "core::option::Option::<core::felt252>"
                        }
                    ]
                }
            ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();
    assert_eq!(result.enums.len(), 0);
    assert_eq!(result.structs.len(), 1);
    assert_eq!(result.interfaces.len(), 0);
    assert_eq!(result.events.len(), 0);
    assert_eq!(result.functions.len(), 0);

    let Some(token) = result.structs.iter().next() else {
        panic!("At least one element should be present in structs");
    };

    let Token::Struct(s1) = &*token.borrow() else {
        panic!("Only element parsed from ABI should be Token::Struct");
    };

    assert_eq!(s1.fields.len(), 1);

    let f1_inner = s1.fields[0].clone();

    let Token::Option(a1) = &*f1_inner.token.as_ref().borrow() else {
        panic!("First field should be Option");
    };

    let Token::Basic(c) = &*a1.inner.borrow() else {
        panic!("Array content should be CoreBasic");
    };
    assert_eq!(c.type_path, "core::felt252");
}

#[test]
fn test_result_is_resolved_as_a_part_of_struct() {
    let abi_json = r#"[
                {
                    "name": "baitcode::TestStruct",
                    "type": "struct",
                    "members": [
                        {
                            "name": "result",
                            "type": "core::result::Result::<core::felt252, core::integer::u32>"
                        }
                    ]
                }
            ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();
    assert_eq!(result.enums.len(), 0);
    assert_eq!(result.structs.len(), 1);
    assert_eq!(result.interfaces.len(), 0);
    assert_eq!(result.events.len(), 0);
    assert_eq!(result.functions.len(), 0);

    let Some(token) = result.structs.iter().next() else {
        panic!("At least one element should be present in structs");
    };

    let Token::Struct(s1) = &*token.borrow() else {
        panic!("Only element parsed from ABI should be Token::Struct");
    };

    assert_eq!(s1.fields.len(), 1);

    let f1_inner = s1.fields[0].clone();

    let Token::Result(a1) = &*f1_inner.token.as_ref().borrow() else {
        panic!("First field should be Result");
    };

    let Token::Basic(c1) = &*a1.inner.borrow() else {
        panic!("Result content should be CoreBasic");
    };
    assert_eq!(c1.type_path, "core::felt252");

    let Token::Basic(c2) = &*a1.error.borrow() else {
        panic!("Result error should be CoreBasic");
    };
    assert_eq!(c2.type_path, "core::integer::u32");
}

#[test]
fn test_non_zero_is_resolved_as_a_part_of_struct() {
    let abi_json = r#"[
                {
                    "name": "baitcode::TestStruct",
                    "type": "struct",
                    "members": [
                        {
                            "name": "result",
                            "type": "core::zeroable::NonZero::<core::felt252>"
                        }
                    ]
                }
            ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();
    assert_eq!(result.enums.len(), 0);
    assert_eq!(result.structs.len(), 1);
    assert_eq!(result.interfaces.len(), 0);
    assert_eq!(result.events.len(), 0);
    assert_eq!(result.functions.len(), 0);

    let Some(token) = result.structs.iter().next() else {
        panic!("At least one element should be present in structs");
    };

    let Token::Struct(s1) = &*token.borrow() else {
        panic!("Only element parsed from ABI should be Token::Struct");
    };

    assert_eq!(s1.fields.len(), 1);

    let f1_inner = s1.fields[0].clone();

    let Token::NonZero(a1) = &*f1_inner.token.as_ref().borrow() else {
        panic!("First field should be NonZero");
    };

    let Token::Basic(c1) = &*a1.inner.borrow() else {
        panic!("Result content should be CoreBasic");
    };
    assert_eq!(c1.type_path, "core::felt252");
}

#[test]
fn test_simple_event_struct_parsing() {
    let abi_json = r#"[
                {
                    "type": "event",
                    "name": "contracts::abicov::simple_events::simple_events::EventWithOtherName",
                    "kind": "struct",
                    "members": [
                      {
                        "name": "value1",
                        "type": "core::felt252",
                        "kind": "data"
                      },
                      {
                        "name": "value2",
                        "type": "core::felt252",
                        "kind": "key"
                      }
                    ]
                }
            ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();
    assert_eq!(result.enums.len(), 0);
    assert_eq!(result.structs.len(), 0);
    assert_eq!(result.interfaces.len(), 0);
    assert_eq!(result.events.len(), 1);
    assert_eq!(result.functions.len(), 0);

    let Some(token) = result.events.iter().next() else {
        panic!("At least one element should be present in events");
    };

    let Token::Event(e1) = &*token.borrow() else {
        panic!("Only element parsed from ABI should be Token::Event");
    };

    assert_eq!(e1.data.len(), 1);
    assert_eq!(e1.keys.len(), 1);

    let f1_inner = e1.data[0].clone();
    assert_eq!(f1_inner.name, "value1");
    let Token::Basic(c1) = &*f1_inner.token.borrow() else {
        panic!("Event field 1 content should be CoreBasic");
    };
    assert_eq!(c1.type_path, "core::felt252");

    let f2_inner = e1.keys[0].clone();
    assert_eq!(f2_inner.name, "value2");
    let Token::Basic(c2) = &*f2_inner.token.borrow() else {
        panic!("Event field 1 content should be CoreBasic");
    };
    assert_eq!(c2.type_path, "core::felt252");
}

#[test]
fn test_nested_event_struct_parsing() {
    let abi_json = r#"[
                {
                    "type": "event",
                    "name": "contracts::Event",
                    "kind": "struct",
                    "members": [
                      {
                        "name": "value1",
                        "type": "core::felt252",
                        "kind": "data"
                      },
                      {
                        "name": "value2",
                        "type": "core::felt252",
                        "kind": "key"
                      }
                    ]
                },
                {
                    "type": "event",
                    "name": "contracts::EventComplex",
                    "kind": "enum",
                    "variants": [
                      {
                        "name": "Event1",
                        "type": "contracts::Event",
                        "kind": "nested"
                      },
                      {
                        "name": "Event2",
                        "type": "contracts::Event",
                        "kind": "nested"
                      }
                    ]
                }
            ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();
    assert_eq!(result.enums.len(), 0);
    assert_eq!(result.structs.len(), 0);
    assert_eq!(result.interfaces.len(), 0);
    assert_eq!(result.events.len(), 2);
    assert_eq!(result.functions.len(), 0);

    for event in result.events.into_iter() {
        let Token::Event(e) = &*event.borrow() else {
            panic!("Only element parsed from ABI should be Token::Event");
        };

        if e.type_path == "contracts::EventComplex" {
            assert_eq!(e.data.len(), 0);
            assert_eq!(e.flat.len(), 0);
            assert_eq!(e.keys.len(), 0);
            assert_eq!(e.nested.len(), 2);

            let f1_inner = e.nested[0].clone();
            assert_eq!(f1_inner.name, "Event1");
            let Token::Event(c1) = &*f1_inner.token.borrow() else {
                panic!("Event field 1 content should be CoreBasic");
            };
            assert_eq!(c1.type_path, "contracts::Event");

            let f2_inner = e.nested[1].clone();
            assert_eq!(f2_inner.name, "Event2");
            let Token::Event(c2) = &*f2_inner.token.borrow() else {
                panic!("Event field 1 content should be CoreBasic");
            };
            assert_eq!(c2.type_path, "contracts::Event");
        }

        if e.type_path == "contracts::Event" {
            assert_eq!(e.data.len(), 1);
            assert_eq!(e.keys.len(), 1);

            let f1_inner = e.data[0].clone();
            assert_eq!(f1_inner.name, "value1");
            let Token::Basic(c1) = &*f1_inner.token.borrow() else {
                panic!("Event field 1 content should be CoreBasic");
            };
            assert_eq!(c1.type_path, "core::felt252");

            let f2_inner = e.keys[0].clone();
            let Token::Basic(c2) = &*f2_inner.token.borrow() else {
                panic!("Event field 1 content should be CoreBasic");
            };
            assert_eq!(c2.type_path, "core::felt252");

            assert_eq!(f2_inner.name, "value2");
        }
    }
}

#[test]
fn test_function_parsing() {
    let abi_json = r#"[
                {
                    "type": "function",
                    "name": "procedure",
                    "inputs": [],
                    "outputs": [],
                    "state_mutability": "external"  
                },
                {
                    "type": "function",
                    "name": "procedure2",
                    "inputs": [],
                    "outputs": [],
                    "state_mutability": "view"  
                },
                {
                    "type": "function",
                    "name": "func",
                    "inputs": [
                        {
                            "name": "arg",
                            "type": "felt"
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

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();

    assert_eq!(result.enums.len(), 0);
    assert_eq!(result.structs.len(), 0);
    assert_eq!(result.interfaces.len(), 0);
    assert_eq!(result.events.len(), 0);
    assert_eq!(result.functions.len(), 3);

    for token in result.functions.into_iter() {
        let Token::Function(func) = &*token.borrow() else {
            panic!("Only element parsed from ABI should be Token::Event");
        };

        if func.name == "func" {
            assert_eq!(func.inputs.len(), 1);

            let i1 = func.inputs[0].clone();
            assert_eq!(i1.name, "arg");
            let Token::Basic(t1) = &*i1.token.borrow() else {
                panic!("funct first input is arg of CoreBasic type");
            };
            assert_eq!(t1.type_path, "felt");

            assert_eq!(func.outputs.len(), 1);
            let o1 = func.outputs[0].clone();
            let Token::Basic(o1) = &*o1.borrow() else {
                panic!("funct first input is arg of CoreBasic type");
            };
            assert_eq!(o1.type_path, "felt");
            assert_eq!(func.state_mutability, StateMutability::External)
        }

        if func.name == "procedure" {
            assert_eq!(func.inputs.len(), 0);
            assert_eq!(func.outputs.len(), 0);
            assert_eq!(func.state_mutability, StateMutability::External)
        }

        if func.name == "procedure2" {
            assert_eq!(func.inputs.len(), 0);
            assert_eq!(func.outputs.len(), 0);
            assert_eq!(func.state_mutability, StateMutability::View)
        }
    }
}

#[test]
fn test_interface_parsing() {
    let abi_json = r#"[
                {
                    "type": "impl",
                    "name": "MyInterfaceImpl",
                    "interface_name": "contracts::abicov::simple_interface::MyInterface"
                },
                {
                    "type": "interface",
                    "name": "contracts::abicov::simple_interface::MyInterface",
                    "items": [
                        {
                            "type": "function",
                            "name": "get_value",
                            "inputs": [],
                            "outputs": [
                                {
                                    "type": "core::felt252"
                                }
                            ],
                            "state_mutability": "view"
                        },
                        {
                            "type": "function",
                            "name": "set_value",
                            "inputs": [
                                {
                                    "name": "value",
                                    "type": "core::felt252"
                                }
                            ],
                            "outputs": [],
                            "state_mutability": "external"
                        }
                    ]
                }
            ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();

    assert_eq!(result.enums.len(), 0);
    assert_eq!(result.structs.len(), 0);
    assert_eq!(result.interfaces.len(), 1);
    assert_eq!(result.interfaces_new.len(), 1);
    assert_eq!(result.events.len(), 0);
    assert_eq!(result.functions.len(), 0);

    let Some(token) = result.interfaces_new.first() else {
        panic!("At least one element should be present in interfaces_new");
    };

    let Token::Interface(interface) = &*token.borrow() else {
        panic!("interfaces should only store interfaces");
    };

    assert_eq!(interface.functions.len(), 2);
}

#[test]
fn test_dojo_starter_direction_available_abi() {
    let abi = AbiParser::tokens_from_abi_string(
        include_str!("../../test_data/dojo_starter-directions_available.abi.json"),
        HashMap::new(),
    )
    .unwrap();

    assert_eq!(abi.structs.len(), 1);
    assert_eq!(abi.enums.len(), 1);

    let Some(enum_token) = abi.enums.into_iter().next() else {
        panic!("Enums should have at least 1 item")
    };

    let Token::Enum(e) = &*enum_token.borrow() else {
        panic!("Enums should only have Token::Enum")
    };

    let Some(struct_token) = abi.structs.into_iter().next() else {
        panic!("Structs should have at least 1 item")
    };

    let Token::Struct(s) = &*struct_token.borrow() else {
        panic!("Structs should only have Token::Struct")
    };

    if let Token::Array(a) = &*s.fields[1].clone().token.as_ref().borrow() {
        let Token::Enum(array_inner) = &*a.inner.borrow() else {
            panic!("Expect array of Direction Enums")
        };
        assert_eq!(5, array_inner.variants.len());
        // Check that copy was properly done

        assert_eq!(array_inner, e);
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_nested_tuple() {
    let abi = AbiParser::tokens_from_abi_string(
        include_str!("../../test_data/struct_tuple.abi.json"),
        HashMap::new(),
    )
    .unwrap();

    assert_eq!(abi.structs.len(), 1);
    assert_eq!(abi.enums.len(), 1);

    let Some(enum_token) = abi.enums.into_iter().next() else {
        panic!("Enums should have at least 1 item")
    };

    let Token::Enum(e) = &*enum_token.borrow() else {
        panic!("Enums should only have Token::Enum")
    };

    let Some(struct_token) = abi.structs.into_iter().next() else {
        panic!("Structs should have at least 1 item")
    };

    let Token::Struct(s) = &*struct_token.borrow() else {
        panic!("Structs should only have Token::Struct")
    };

    if let Token::Array(a) = &*s.fields[1].clone().token.as_ref().borrow() {
        let Token::Tuple(t) = &*a.inner.borrow() else {
            panic!("Expect second field to hold Tuple")
        };

        let Token::Enum(tuple_f1) = &*t.inners[0].borrow() else {
            panic!("Expect first tuple element to be Enum")
        };

        assert_eq!(5, tuple_f1.variants.len());
        // Check that copy was properly done
        assert_eq!(tuple_f1, e);
    }
}

#[test]
fn test_collect_tokens() {
    let sierra_abi = include_str!("../../test_data/cairo_ls_abi.json");
    let sierra = serde_json::from_str::<SierraClass>(sierra_abi).unwrap();
    let tokens = AbiParser::collect_tokens(sierra.abi, ParserContext::default()).unwrap();
    assert_ne!(tokens.enums.len(), 0);
    assert_ne!(tokens.functions.len(), 0);
    assert_ne!(tokens.interfaces.len(), 0);
    assert_ne!(tokens.structs.len(), 0);
}

#[test]
fn events_parsing() {
    let abi_json = r#"[
            {
                "type": "event",
                "name": "zzz::HelloStarknet::BookAdded",
                "kind": "struct",
                "members": [
                    {
                        "name": "id",
                        "type": "core::integer::u32",
                        "kind": "data"
                    },
                    {
                        "name": "title",
                        "type": "core::felt252",
                        "kind": "data"
                    },
                    {
                        "name": "author",
                        "type": "core::felt252",
                        "kind": "key"
                    }
                ]
            },
            {
                "type": "event",
                "name": "zzz::HelloStarknet::UpdatedTitleData",
                "kind": "struct",
                "members": [
                    {
                        "name": "id",
                        "type": "core::integer::u32",
                        "kind": "key"
                    },
                    {
                        "name": "new_title",
                        "type": "core::felt252",
                        "kind": "data"
                    }
                ]
            },
            {
                "type": "event",
                "name": "zzz::HelloStarknet::UpdatedAuthorData",
                "kind": "struct",
                "members": [
                    {
                        "name": "id",
                        "type": "core::integer::u32",
                        "kind": "key"
                    },
                    {
                        "name": "new_author",
                        "type": "core::felt252",
                        "kind": "data"
                    }
                ]
            },
            {
                "type": "event",
                "name": "zzz::HelloStarknet::FieldUpdated",
                "kind": "enum",
                "variants": [
                    {
                        "name": "Title",
                        "type": "zzz::HelloStarknet::UpdatedTitleData",
                        "kind": "nested"
                    },
                    {
                        "name": "Author",
                        "type": "zzz::HelloStarknet::UpdatedAuthorData",
                        "kind": "nested"
                    }
                ]
            },
            {
                "type": "event",
                "name": "zzz::HelloStarknet::BookRemoved",
                "kind": "struct",
                "members": [
                    {
                        "name": "id",
                        "type": "core::integer::u32",
                        "kind": "data"
                    }
                ]
            },
            {
                "type": "event",
                "name": "zzz::HelloStarknet::Event",
                "kind": "enum",
                "variants": [
                    {
                        "name": "BookAdded",
                        "type": "zzz::HelloStarknet::BookAdded",
                        "kind": "nested"
                    },
                    {
                        "name": "FieldUpdated",
                        "type": "zzz::HelloStarknet::FieldUpdated",
                        "kind": "flat"
                    },
                    {
                        "name": "BookRemoved",
                        "type": "zzz::HelloStarknet::BookRemoved",
                        "kind": "nested"
                    }
                ]
            }
        ]"#;

    let result = AbiParser::tokens_from_abi_string(abi_json, HashMap::new()).unwrap();

    assert_eq!(result.enums.len(), 0);
    assert_eq!(result.structs.len(), 0);
    assert_eq!(result.interfaces.len(), 0);
    assert_eq!(result.events.len(), 6);
    assert_eq!(result.functions.len(), 0);

    for token in result.events.iter() {
        let Token::Event(event) = &*token.borrow() else {
            panic!("Only element parsed from ABI should be Token::Event");
        };

        match event.type_path.as_str() {
            "zzz::HelloStarknet::Event" => {
                assert_eq!(event.kind, EventKind::Enum);
                assert_eq!(event.flat.len(), 1);
                assert_eq!(event.nested.len(), 2);
                assert_eq!(event.data.len(), 0);
                assert_eq!(event.keys.len(), 0);
            }
            "zzz::HelloStarknet::FieldUpdated" => {
                assert_eq!(event.kind, EventKind::Enum);
                assert_eq!(event.flat.len(), 0);
                assert_eq!(event.nested.len(), 2);
                assert_eq!(event.data.len(), 0);
                assert_eq!(event.keys.len(), 0);
            }
            "zzz::HelloStarknet::BookAdded" => {
                assert_eq!(event.kind, EventKind::Struct);
                assert_eq!(event.flat.len(), 0);
                assert_eq!(event.nested.len(), 0);
                assert_eq!(event.data.len(), 2);
                assert_eq!(event.keys.len(), 1);
            }
            "zzz::HelloStarknet::UpdatedTitleData" => {
                assert_eq!(event.kind, EventKind::Struct);
                assert_eq!(event.flat.len(), 0);
                assert_eq!(event.nested.len(), 0);
                assert_eq!(event.data.len(), 1);
                assert_eq!(event.keys.len(), 1);
            }
            "zzz::HelloStarknet::UpdatedAuthorData" => {
                assert_eq!(event.kind, EventKind::Struct);
                assert_eq!(event.flat.len(), 0);
                assert_eq!(event.nested.len(), 0);
                assert_eq!(event.data.len(), 1);
                assert_eq!(event.keys.len(), 1);
            }
            "zzz::HelloStarknet::BookRemoved" => {
                assert_eq!(event.kind, EventKind::Struct);
                assert_eq!(event.flat.len(), 0);
                assert_eq!(event.nested.len(), 0);
                assert_eq!(event.data.len(), 1);
                assert_eq!(event.keys.len(), 0);
            }
            _ => (),
        }
    }
}

#[test]
fn test_unresolved_type_raiees_error() {
    let abi_json = r#"[
            {
                "type": "struct",
                "name": "contracts::abicov::structs::GenericOne::<core::felt252>",
                "members": [
                    {
                        "name": "a",
                        "type": "core::felt252"
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
            }
        ]"#;

    let ctx = ParserContext::new();

    let abi_entries = AbiParser::parse_abi_string(abi_json).unwrap();

    let result = AbiParser::build_registry(abi_entries, ctx);

    assert!(
        result.is_err(),
        "Type core::integer::u256 should be unknown"
    );
}

#[test]
fn test_skip_type_incomplete_type() {
    let abi_json = r#"[
            {
                "type": "struct",
                "name": "contracts::abicov::structs::GenericOne::<core::felt252>",
                "members": [
                    {
                        "name": "a",
                        "type": "core::felt252"
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
            }
        ]"#;

    let ctx = ParserContext::new().with_type_skips(vec!["core::integer::u256"]);

    let abi_entries = AbiParser::parse_abi_string(abi_json).unwrap();

    let result = AbiParser::build_registry(abi_entries, ctx);

    assert!(
        result.is_err(),
        "You can't construct type with skipped field"
    );
}

#[test]
fn test_skip_type() {
    let abi_json = r#"[
            {
                "type": "struct",
                "name": "contracts::abicov::structs::GenericOne::<core::felt252>",
                "members": [
                    {
                        "name": "a",
                        "type": "core::felt252"
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
            }
        ]"#;

    let ctx = ParserContext::new().with_type_skips(vec!["contracts::abicov::structs::GenericOne"]);

    let abi_entries = AbiParser::parse_abi_string(abi_json).unwrap();

    let result = AbiParser::build_registry(abi_entries, ctx);

    assert!(result.is_ok(), "All types should be skipped");
}

#[test]
fn test_substitute_type() {
    let abi_json = r#"[
            {
                "type": "struct",
                "name": "contracts::abicov::structs::GenericOne::<core::felt252>",
                "members": [
                    {
                        "name": "a",
                        "type": "core::felt252"
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
            }
        ]"#;

    let ctx = ParserContext::new().with_substitutions(HashMap::from([(
        "core::integer::u256",
        "cainome::cairo_serde::U256",
    )]));

    let abi_entries = AbiParser::parse_abi_string(abi_json).unwrap();

    let result = AbiParser::build_registry(abi_entries, ctx);

    assert!(result.is_ok(), "Type core::integer::u256 should be skipped");
}

#[test]
fn test_skip_generic_type() {
    let abi_json = r#"[
            {
                "type": "struct",
                "name": "contracts::abicov::structs::GenericOne::<core::felt252>",
                "members": [
                    {
                        "name": "a",
                        "type": "core::felt252"
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
            }
        ]"#;

    let ctx = ParserContext::new().with_type_skips(vec!["contracts::abicov::structs::GenericOne"]);

    let abi_entries = AbiParser::parse_abi_string(abi_json).unwrap();

    let result = AbiParser::build_registry(abi_entries, ctx);

    assert!(result.is_ok(), "Type all types should be skipped");
}
