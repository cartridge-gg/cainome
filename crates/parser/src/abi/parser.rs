use starknet::core::types::contract::{AbiEntry, AbiEvent, SierraClass, TypedAbiEvent};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use crate::abi::extensions::{Named, TokenConvertable};
use crate::abi::registry::TypeRegistry;
use crate::tokens::{constants, CoreBasic, Interface, Token};
use crate::{CainomeResult, Error};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TokenizedAbi {
    /// All enums found in the contract ABI.
    pub enums: Vec<Token>,
    /// All structs found in the contract ABI.
    pub structs: Vec<Token>,
    /// Standalone functions in the contract ABI.
    pub functions: Vec<Token>,
    /// Events.
    pub events: Vec<Token>,
    /// Fully qualified interface name mapped to all the defined functions in it.
    pub interfaces: HashMap<String, Vec<Token>>,
    pub interfaces_new: Vec<Token>,
}

pub struct AbiParser {}

impl AbiParser {
    /// Generates the [`Token`]s from the given ABI string.
    ///
    /// The `abi` can have two formats:
    /// 1. Entire [`SierraClass`] json representation.
    /// 2. The `abi` key from the [`SierraClass`], which is an array of [`AbiEntry`].
    ///
    /// # Arguments
    ///
    /// * `abi` - A string representing the ABI.
    /// * `type_aliases` - Types to be renamed to avoid name clashing of generated types.
    pub fn tokens_from_abi_string(
        abi: &str,
        type_aliases: &HashMap<String, String>,
    ) -> CainomeResult<TokenizedAbi> {
        let abi_entries = Self::parse_abi_string(abi)?;
        let tokenized_abi = AbiParser::collect_tokens(abi_entries).expect("failed tokens parsing");

        Ok(tokenized_abi)
    }

    /// Parses an ABI string to output a `Vec<AbiEntry>`.
    ///
    /// The `abi` can have two formats:
    /// 1. Entire [`SierraClass`] json representation.
    /// 2. The `abi` key from the [`SierraClass`], which is an array of AbiEntry.
    ///
    /// # Arguments
    ///
    /// * `abi` - A string representing the ABI.
    pub fn parse_abi_string(abi: &str) -> CainomeResult<Vec<AbiEntry>> {
        let entries = if let Ok(sierra) = serde_json::from_str::<SierraClass>(abi) {
            sierra.abi
        } else {
            serde_json::from_str::<Vec<AbiEntry>>(abi).map_err(Error::SerdeJson)?
        };

        Ok(entries)
    }

    // TODO: think on using visitor pattern (not only in that case).
    fn has_unknown_dependencies(
        entry: &AbiEntry,
        registry: &TypeRegistry,
    ) -> CainomeResult<Option<String>> {
        match entry {
            // move to abi extensions
            AbiEntry::Function(abi_function) => {
                for item in abi_function.inputs.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(Some(item.r#type.to_string()));
                    }
                }

                for item in abi_function.outputs.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(Some(item.r#type.to_string()));
                    }
                }

                Ok(None)
            }
            AbiEntry::Event(abi_event) => match abi_event {
                AbiEvent::Typed(typed_abi_event) => match typed_abi_event {
                    TypedAbiEvent::Struct(abi_event_struct) => {
                        for item in abi_event_struct.members.iter() {
                            if !registry.is_known_type(&item.r#type)? {
                                return Ok(Some(item.r#type.to_string()));
                            }
                        }

                        Ok(None)
                    }
                    TypedAbiEvent::Enum(abi_event_enum) => {
                        for item in abi_event_enum.variants.iter() {
                            if !registry.is_known_type(&item.r#type)? {
                                return Ok(Some(item.r#type.to_string()));
                            }
                        }
                        Ok(None)
                    }
                },
                AbiEvent::Untyped(event) => {
                    for item in event.inputs.iter() {
                        if !registry.is_known_type(&item.r#type)? {
                            return Ok(Some(item.r#type.to_string()));
                        }
                    }
                    Ok(None)
                }
            },

            AbiEntry::Struct(abi_struct) => {
                for item in abi_struct.members.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(Some(item.r#type.to_string()));
                    }
                }

                Ok(None)
            }

            AbiEntry::Enum(abi_enum) => {
                for item in abi_enum.variants.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(Some(item.r#type.to_string()));
                    }
                }

                Ok(None)
            }

            AbiEntry::Constructor(abi_constructor) => {
                for item in abi_constructor.inputs.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(Some(item.r#type.to_string()));
                    }
                }

                Ok(None)
            }

            AbiEntry::Interface(abi_interface) => {
                for item in abi_interface.items.iter() {
                    if let Some(unknown_field) = Self::has_unknown_dependencies(item, registry)? {
                        return Ok(Some(unknown_field));
                    }
                }

                Ok(None)
            }

            AbiEntry::L1Handler(abi_function) => {
                for item in abi_function.inputs.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(Some(item.r#type.to_string()));
                    }
                }

                for item in abi_function.outputs.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(Some(item.r#type.to_string()));
                    }
                }

                Ok(None)
            }
            AbiEntry::Impl(abi_impl) => {
                if !registry.is_known_type(&abi_impl.interface_name)? {
                    return Ok(Some(abi_impl.interface_name.clone()));
                }

                Ok(None)
            }
        }
    }

    pub fn build_registry(entries: Vec<AbiEntry>) -> CainomeResult<TypeRegistry> {
        let mut registry = TypeRegistry::new();

        let mut local_entries = VecDeque::from(entries.clone());
        let mut seen_since_last_removal = 0;
        let mut unknown_fields: HashSet<String> = HashSet::new();

        // We will be converting AbiEntries to tokens and drop them upon converting
        while local_entries.len() > 0 {
            // This branch means that we went through all the AbiEntry and could not
            // convert any. This means Abi is incorrect (well, we might have a bug though)
            if seen_since_last_removal > local_entries.len() {
                // TODO: sort out error types
                return Err(Error::ParsingFailed(format!(
                    "Can't resolve ABI types. Some type might be missing. Check: [{}]",
                    unknown_fields.into_iter().collect::<Vec<_>>().join(", ")
                )));
            }

            let entry = local_entries.pop_front().expect("Should always succeed");
            seen_since_last_removal += 1;

            // Workaround to skip parsing Composite CoreBasics (like core::boolean).
            // As get also strips all the containers, those will be dropped at this point.
            // Might also work for type duplicates, but I don't think those exist.
            // NOTE: this might move into extensions for AbiEntry
            if let Ok(_) = registry.get(&entry.get_name()) {
                seen_since_last_removal = 0;
                continue;
            }

            // Checking if registry has all the nested types to resolve
            // and contruct token.
            if let Some(unknown_field) = Self::has_unknown_dependencies(&entry, &registry)? {
                // We can't resolve that now, let's put to the end of the queue
                local_entries.push_back(entry);
                unknown_fields.insert(unknown_field);
            } else {
                // Let's resolve then.
                let token = entry.to_token(&mut registry)?;
                // TODO: registry can get type path itself
                registry.set(token.type_path(), token);
                seen_since_last_removal = 0;
                unknown_fields.clear();
            }
        }
        Ok(registry)
    }

    /// Parse all tokens in the ABI.
    pub fn collect_tokens(entries: Vec<AbiEntry>) -> CainomeResult<TokenizedAbi> {
        // This procedure will populate all the tokens
        let registry = Self::build_registry(entries)?;

        // So we'll need to just bucket them in correct fields.
        let tokens = registry.values();

        let mut structs = vec![];
        let mut enums = vec![];
        let mut events = vec![];
        let mut functions = vec![];
        let mut interfaces_new = vec![];
        let mut interfaces = HashMap::new();

        for token in tokens {
            match token.as_ref() {
                Token::Function(function) => {
                    functions.push(Token::Function(function.clone()));
                }
                Token::Interface(interface) => {
                    let mut new_function_tokens = vec![];

                    for function_token in interface.functions.iter() {
                        let Token::Function(function) = function_token.as_ref() else {
                            unreachable!("According to ABI specs only function can be there")
                        };

                        new_function_tokens.push(Token::Function(function.clone()));
                    }

                    interfaces_new.push(Token::Interface(interface.clone()));
                    interfaces.insert(interface.type_path.clone(), new_function_tokens);
                }
                Token::Event(event) => events.push(Token::Event(event.clone())),
                Token::Enum(enumeration) => enums.push(Token::Enum(enumeration.clone())),
                Token::Struct(structure) => structs.push(Token::Struct(structure.clone())),
                _ => (),
            }
        }

        Ok(TokenizedAbi {
            enums,
            events,
            structs,
            functions,
            interfaces,
            interfaces_new,
        })
    }
}

#[cfg(test)]
mod tests {

    use crate::tokens::StateMutability;

    use super::*;

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

        let result = AbiParser::tokens_from_abi_string(abi_json, &HashMap::new()).unwrap();

        assert_eq!(result.structs.len(), 1);
        assert_eq!(result.interfaces.len(), 0);
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.functions.len(), 0);
        assert_eq!(result.enums.len(), 0);
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

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();

        assert_eq!(result.structs.len(), 1);

        let Some(Token::Struct(s1)) = result.structs.iter().next() else {
            panic!("Only element parsed from ABI should be Token::Struct");
        };

        assert_eq!(s1.fields.len(), core_fields.len());

        for (idx, (name, ttype)) in core_fields.iter().enumerate() {
            assert_eq!(s1.fields[idx].name, name.to_string());
            let Token::CoreBasic(f1) = s1.fields[idx].token.as_ref() else {
                panic!("Only element parsed from ABI should be Token::Struct");
            };
            assert_eq!(ttype.to_string(), f1.type_path);
        }
    }

    #[test]
    fn check_array_container_is_parsed() {
        let abi_json = format!(
            r#"[
                {{
                    "type": "struct",
                    "name": "my::package::AllInOne",
                    "members": [
                        {{
                            "name": "f1",
                            "type": "core::array::Span::<felt>"
                        }},
                        {{
                            "name": "f2",
                            "type": "core::array::Array::<felt>"
                        }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
        assert_eq!(result.structs.len(), 1);

        let Some(Token::Struct(s1)) = result.structs.iter().next() else {
            panic!("Only element parsed from ABI should be Token::Struct");
        };

        assert_eq!(s1.fields.len(), 2);

        let f1_inner = s1.fields[0].clone();
        let f2_inner = s1.fields[1].clone();

        let Token::Array(a1) = f1_inner.token.as_ref() else {
            panic!("First field should be Array");
        };
        let Token::Array(a2) = f2_inner.token.as_ref() else {
            panic!("Second field should be Array");
        };

        assert_eq!(a1.inner.as_ref().type_path(), "felt");
        assert_eq!(a2.inner.as_ref().type_path(), "felt");
    }

    #[test]
    fn check_that_array_container_is_ignored_toplevel() {
        let abi_json = format!(
            r#"[
                {{
                  "type": "struct",
                  "name": "core::array::Span::<core::felt252>",
                  "members": [
                    {{
                      "name": "snapshot",
                      "type": "@core::array::Array::<core::felt252>"
                    }}
                  ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
        assert_eq!(result.enums.len(), 0);
        assert_eq!(result.structs.len(), 0);
        assert_eq!(result.interfaces.len(), 0);
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.functions.len(), 0);
    }

    #[test]
    fn check_tuple_container_is_parsed() {
        let abi_json = format!(
            r#"[
                {{
                    "type": "struct",
                    "name": "my::package::AllInOne",
                    "members": [
                        {{
                            "name": "f1",
                            "type": "(felt, core::integer::i32)"
                        }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
        assert_eq!(result.structs.len(), 1);

        let Some(Token::Struct(s1)) = result.structs.iter().next() else {
            panic!("Only element parsed from ABI should be Token::Struct");
        };

        assert_eq!(s1.fields.len(), 1);

        let f1_inner = s1.fields[0].clone();

        let Token::Tuple(a1) = f1_inner.token.as_ref() else {
            panic!("First field should be Tuple");
        };

        let Token::CoreBasic(t1) = a1.inners[0].as_ref() else {
            panic!("First tuple element should be CoreBasic");
        };
        let Token::CoreBasic(t2) = a1.inners[1].as_ref() else {
            panic!("Second tuple element should be CoreBasic");
        };

        assert_eq!(t1.type_path, "felt");
        assert_eq!(t2.type_path, "core::integer::i32");
    }

    #[test]
    fn check_nested_tuple_container_is_parsed() {
        let abi_json = format!(
            r#"[
                {{
                    "type": "struct",
                    "name": "my::package::AllInOne",
                    "members": [
                        {{
                            "name": "f1",
                            "type": "(felt, (felt, core::integer::i32))"
                        }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
        assert_eq!(result.structs.len(), 1);

        let Some(Token::Struct(s1)) = result.structs.iter().next() else {
            panic!("Only element parsed from ABI should be Token::Struct");
        };

        assert_eq!(s1.fields.len(), 1);

        let f1_inner = s1.fields[0].clone();

        let Token::Tuple(a1) = f1_inner.token.as_ref() else {
            panic!("First field should be Tuple");
        };

        let Token::CoreBasic(t1) = a1.inners[0].as_ref() else {
            panic!("First tuple element should be CoreBasic");
        };

        assert_eq!(t1.type_path, "felt");

        let Token::Tuple(t2) = a1.inners[1].as_ref() else {
            panic!("Second tuple element should be Tuple");
        };

        let Token::CoreBasic(nested_t1) = t2.inners[0].as_ref() else {
            panic!("First nested tuple element should be CoreBasic");
        };
        let Token::CoreBasic(nested_t2) = t2.inners[1].as_ref() else {
            panic!("Second nested tuple element should be CoreBasic");
        };

        assert_eq!(nested_t1.type_path, "felt");
        assert_eq!(nested_t2.type_path, "core::integer::i32");
    }

    #[test]
    fn check_bool_enum_is_ignored_on_parse() {
        let abi_json = format!(
            r#"[
                {{
                    "name": "core::bool",
                    "type": "enum",
                    "variants": [
                        {{
                            "name": "False",
                            "type": "()"
                        }},
                        {{
                            "name": "True",
                            "type": "()"
                        }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
        assert_eq!(result.structs.len(), 0);
        assert_eq!(result.enums.len(), 0);
    }

    #[test]
    fn check_basic_enum_is_parsed() {
        let abi_json = format!(
            r#"[
                {{
                    "name": "contracts::abicov::enums::MixedEnum",
                    "type": "enum",
                    "variants": [
                        {{
                            "name": "Variant1",
                            "type": "core::felt252"
                        }},
                        {{
                            "name": "Variant2",
                            "type": "()"
                        }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
        assert_eq!(result.enums.len(), 1);

        let Some(Token::Enum(s1)) = result.enums.iter().next() else {
            panic!("Only element parsed from ABI should be Token::Enum");
        };

        assert_eq!(s1.variants.len(), 2);

        let f1_inner = s1.variants[0].clone();
        let f2_inner = s1.variants[1].clone();

        let Token::CoreBasic(a1) = f1_inner.token.as_ref() else {
            panic!("First variant should be CoreBasic");
        };
        let Token::CoreBasic(a2) = f2_inner.token.as_ref() else {
            panic!("Second variant should be CoreBasic");
        };

        assert_eq!(a1.type_path, "core::felt252");
        assert_eq!(a2.type_path, "()");
    }

    #[test]
    fn check_complex_enum_is_parsed() {
        let abi_json = format!(
            r#"[
                {{
                    "type": "struct",
                    "name": "core::integer::u256",
                    "members": [
                        {{
                            "name": "low",
                            "type": "core::integer::u128"
                        }},
                        {{
                            "name": "high",
                            "type": "core::integer::u128"
                        }}
                    ]
                }},
                {{
                    "name": "contracts::abicov::enums::TypedEnum",
                    "type": "enum",
                    "variants": [
                        {{
                            "name": "Variant1",
                            "type": "core::felt252"
                        }},
                        {{
                            "name": "Variant2",
                            "type": "core::integer::u256"
                        }},
                        {{
                            "name": "Variant3",
                            "type": "(core::felt252, core::integer::u256)"
                        }},
                        {{
                            "name": "Variant4",
                            "type": "core::starknet::contract_address::ContractAddress"
                        }},
                        {{
                            "name": "Variant5",
                            "type": "contracts::abicov::enums::Simple"
                        }},
                        {{
                            "name": "Variant6",
                            "type": "contracts::abicov::enums::StructWithStruct"
                        }}
                    ]
                }},
                {{
                    "type": "struct",
                    "name": "contracts::abicov::enums::StructWithStruct",
                    "members": [
                        {{
                            "name": "simple",
                            "type": "contracts::abicov::enums::Simple"
                        }}
                    ]
                }},
                {{
                    "type": "struct",
                    "name": "contracts::abicov::enums::Simple",
                    "members": [
                        {{
                            "name": "span",
                            "type": "felt"
                        }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
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

        let result = AbiParser::tokens_from_abi_string(abi_json, &HashMap::new()).unwrap();

        assert_eq!(result.structs.len(), 2);
        assert_eq!(result.interfaces.len(), 0);
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.functions.len(), 0);
        assert_eq!(result.enums.len(), 0);
    }

    #[test]
    fn test_option_is_ignored_during_parsing() {
        let abi_json = format!(
            r#"[
                {{
                    "name": "core::option::Option::<core::felt252>",
                    "type": "enum",
                    "variants": [
                        {{
                            "name": "Some",
                            "type": "core::felt252"
                        }},
                        {{
                            "name": "None",
                            "type": "()"
                        }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
        assert_eq!(result.enums.len(), 0);
        assert_eq!(result.structs.len(), 0);
        assert_eq!(result.interfaces.len(), 0);
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.functions.len(), 0);
    }

    #[test]
    fn test_option_is_resolved_as_a_part_of_struct() {
        let abi_json = format!(
            r#"[
                {{
                    "name": "baitcode::TestStruct",
                    "type": "struct",
                    "members": [
                        {{
                            "name": "option",
                            "type": "core::option::Option::<core::felt252>"
                        }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
        assert_eq!(result.enums.len(), 0);
        assert_eq!(result.structs.len(), 1);
        assert_eq!(result.interfaces.len(), 0);
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.functions.len(), 0);

        let Some(Token::Struct(s1)) = result.structs.iter().next() else {
            panic!("Only element parsed from ABI should be Token::Struct");
        };

        assert_eq!(s1.fields.len(), 1);

        let f1_inner = s1.fields[0].clone();

        let Token::Option(a1) = f1_inner.token.as_ref() else {
            panic!("First field should be Option");
        };

        assert_eq!(a1.inner.as_ref().type_path(), "core::felt252");
    }

    #[test]
    fn test_result_is_ignored_during_parsing() {
        let abi_json = format!(
            r#"[
                {{
                    "name": "core::result::Result::<core::felt252, core::integer::u32>",
                    "type": "enum",
                    "variants": [
                        {{
                            "name": "Ok",
                            "type": "core::felt252"
                        }},
                        {{
                            "name": "Err",
                            "type": "core::integer::u32"
                        }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
        assert_eq!(result.enums.len(), 0);
        assert_eq!(result.structs.len(), 0);
        assert_eq!(result.interfaces.len(), 0);
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.functions.len(), 0);
    }

    #[test]
    fn test_result_is_resolved_as_a_part_of_struct() {
        let abi_json = format!(
            r#"[
                {{
                    "name": "baitcode::TestStruct",
                    "type": "struct",
                    "members": [
                        {{
                            "name": "result",
                            "type": "core::result::Result::<core::felt252, core::integer::u32>"
                        }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
        assert_eq!(result.enums.len(), 0);
        assert_eq!(result.structs.len(), 1);
        assert_eq!(result.interfaces.len(), 0);
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.functions.len(), 0);

        let Some(Token::Struct(s1)) = result.structs.iter().next() else {
            panic!("Only element parsed from ABI should be Token::Struct");
        };

        assert_eq!(s1.fields.len(), 1);

        let f1_inner = s1.fields[0].clone();

        let Token::Result(a1) = f1_inner.token.as_ref() else {
            panic!("First field should be Result");
        };

        assert_eq!(a1.inner.as_ref().type_path(), "core::felt252");
        assert_eq!(a1.error.as_ref().type_path(), "core::integer::u32");
    }

    #[test]
    fn test_non_zero_is_resolved_as_a_part_of_struct() {
        let abi_json = format!(
            r#"[
                {{
                    "name": "baitcode::TestStruct",
                    "type": "struct",
                    "members": [
                        {{
                            "name": "result",
                            "type": "core::zeroable::NonZero::<core::felt252>"
                        }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
        assert_eq!(result.enums.len(), 0);
        assert_eq!(result.structs.len(), 1);
        assert_eq!(result.interfaces.len(), 0);
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.functions.len(), 0);

        let Some(Token::Struct(s1)) = result.structs.iter().next() else {
            panic!("Only element parsed from ABI should be Token::Struct");
        };

        assert_eq!(s1.fields.len(), 1);

        let f1_inner = s1.fields[0].clone();

        let Token::NonZero(a1) = f1_inner.token.as_ref() else {
            panic!("First field should be NonZero");
        };

        assert_eq!(a1.inner.as_ref().type_path(), "core::felt252");
    }

    #[test]
    fn test_simple_event_struct_parsing() {
        let abi_json = format!(
            r#"[
                {{
                    "type": "event",
                    "name": "contracts::abicov::simple_events::simple_events::EventWithOtherName",
                    "kind": "struct",
                    "members": [
                      {{
                        "name": "value1",
                        "type": "core::felt252",
                        "kind": "data"
                      }},
                      {{
                        "name": "value2",
                        "type": "core::felt252",
                        "kind": "key"
                      }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
        assert_eq!(result.enums.len(), 0);
        assert_eq!(result.structs.len(), 0);
        assert_eq!(result.interfaces.len(), 0);
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.functions.len(), 0);

        let Some(Token::Event(e1)) = result.events.iter().next() else {
            panic!("Only element parsed from ABI should be Token::Event");
        };

        assert_eq!(e1.data.len(), 1);
        assert_eq!(e1.keys.len(), 1);

        let f1_inner = e1.data[0].clone();
        assert_eq!(f1_inner.token.as_ref().type_path(), "core::felt252");
        assert_eq!(f1_inner.name, "value1");

        let f2_inner = e1.keys[0].clone();
        assert_eq!(f2_inner.token.as_ref().type_path(), "core::felt252");
        assert_eq!(f2_inner.name, "value2");
    }

    #[test]
    fn test_nested_event_struct_parsing() {
        let abi_json = format!(
            r#"[
                {{
                    "type": "event",
                    "name": "contracts::Event",
                    "kind": "struct",
                    "members": [
                      {{
                        "name": "value1",
                        "type": "core::felt252",
                        "kind": "data"
                      }},
                      {{
                        "name": "value2",
                        "type": "core::felt252",
                        "kind": "key"
                      }}
                    ]
                }},
                {{
                    "type": "event",
                    "name": "contracts::EventComplex",
                    "kind": "enum",
                    "variants": [
                      {{
                        "name": "Event1",
                        "type": "contracts::Event",
                        "kind": "nested"
                      }},
                      {{
                        "name": "Event2",
                        "type": "contracts::Event",
                        "kind": "nested"
                      }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();
        assert_eq!(result.enums.len(), 0);
        assert_eq!(result.structs.len(), 0);
        assert_eq!(result.interfaces.len(), 0);
        assert_eq!(result.events.len(), 2);
        assert_eq!(result.functions.len(), 0);

        for event in result.events.into_iter() {
            let Token::Event(e) = event else {
                panic!("Only element parsed from ABI should be Token::Event");
            };

            if e.type_path == "contracts::EventComplex" {
                assert_eq!(e.data.len(), 0);
                assert_eq!(e.flat.len(), 0);
                assert_eq!(e.keys.len(), 0);
                assert_eq!(e.nested.len(), 2);

                let f1_inner = e.nested[0].clone();
                assert_eq!(f1_inner.token.as_ref().type_path(), "contracts::Event");
                assert_eq!(f1_inner.name, "Event1");

                let f2_inner = e.nested[1].clone();
                assert_eq!(f2_inner.token.as_ref().type_path(), "contracts::Event");
                assert_eq!(f2_inner.name, "Event2");
            }

            if e.type_path == "contracts::Event" {
                assert_eq!(e.data.len(), 1);
                assert_eq!(e.keys.len(), 1);

                let f1_inner = e.data[0].clone();
                assert_eq!(f1_inner.token.as_ref().type_path(), "core::felt252");
                assert_eq!(f1_inner.name, "value1");

                let f2_inner = e.keys[0].clone();
                assert_eq!(f2_inner.token.as_ref().type_path(), "core::felt252");
                assert_eq!(f2_inner.name, "value2");
            }
        }
    }

    #[test]
    fn test_function_parsing() {
        let abi_json = format!(
            r#"[
                {{
                    "type": "function",
                    "name": "procedure",
                    "inputs": [],
                    "outputs": [],
                    "state_mutability": "external"  
                }},
                {{
                    "type": "function",
                    "name": "procedure2",
                    "inputs": [],
                    "outputs": [],
                    "state_mutability": "view"  
                }},
                {{
                    "type": "function",
                    "name": "func",
                    "inputs": [
                        {{
                            "name": "arg",
                            "type": "felt"
                        }}
                    ],
                    "outputs": [
                        {{
                            "type": "felt"
                        }}
                    ],
                    "state_mutability": "external"  
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();

        assert_eq!(result.enums.len(), 0);
        assert_eq!(result.structs.len(), 0);
        assert_eq!(result.interfaces.len(), 0);
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.functions.len(), 3);

        for token in result.functions.into_iter() {
            let Token::Function(func) = token else {
                panic!("Only element parsed from ABI should be Token::Event");
            };

            if func.name == "func" {
                assert_eq!(func.inputs.len(), 1);

                let i1 = func.inputs[0].clone();
                assert_eq!(i1.name, "arg");
                let Token::CoreBasic(t1) = i1.token.as_ref() else {
                    panic!("funct first input is arg of CoreBasic type");
                };
                assert_eq!(t1.type_path, "felt");

                assert_eq!(func.outputs.len(), 1);
                let o1 = func.outputs[0].clone();
                let Token::CoreBasic(o1) = o1.as_ref() else {
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
        let abi_json = format!(
            r#"[
                {{
                    "type": "impl",
                    "name": "MyInterfaceImpl",
                    "interface_name": "contracts::abicov::simple_interface::MyInterface"
                }},
                {{
                    "type": "interface",
                    "name": "contracts::abicov::simple_interface::MyInterface",
                    "items": [
                        {{
                            "type": "function",
                            "name": "get_value",
                            "inputs": [],
                            "outputs": [
                                {{
                                    "type": "core::felt252"
                                }}
                            ],
                            "state_mutability": "view"
                        }},
                        {{
                            "type": "function",
                            "name": "set_value",
                            "inputs": [
                                {{
                                    "name": "value",
                                    "type": "core::felt252"
                                }}
                            ],
                            "outputs": [],
                            "state_mutability": "external"
                        }}
                    ]
                }}
            ]"#,
        );

        let result = AbiParser::tokens_from_abi_string(&abi_json, &HashMap::new()).unwrap();

        assert_eq!(result.enums.len(), 0);
        assert_eq!(result.structs.len(), 0);
        assert_eq!(result.interfaces.len(), 1);
        assert_eq!(result.interfaces_new.len(), 1);
        assert_eq!(result.events.len(), 0);
        assert_eq!(result.functions.len(), 0);

        let Some(Token::Interface(interface)) = result.interfaces_new.iter().next() else {
            panic!("interfaces should only store interfaces");
        };

        assert_eq!(interface.functions.len(), 2);
    }

    #[test]
    fn test_dojo_starter_direction_available_abi() {
        let abi = AbiParser::tokens_from_abi_string(
            include_str!("../../test_data/dojo_starter-directions_available.abi.json"),
            &HashMap::new(),
        )
        .unwrap();

        assert_eq!(abi.structs.len(), 1);
        assert_eq!(abi.enums.len(), 1);

        let Token::Enum(e) = abi.enums.into_iter().next().unwrap() else {
            panic!("Enums should only have Token::Enum")
        };

        let Token::Struct(s) = abi.structs.into_iter().next().unwrap() else {
            panic!("Structs should only have Token::Struct")
        };

        if let Token::Array(a) = &s.fields[1].token.as_ref() {
            let Token::Enum(array_inner) = a.inner.as_ref() else {
                panic!("Expect array of Direction Enums")
            };
            assert_eq!(5, array_inner.variants.len());
            // Check that copy was properly done

            assert_eq!(array_inner, &e);
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_nested_tuple() {
        let abi = AbiParser::tokens_from_abi_string(
            include_str!("../../test_data/struct_tuple.abi.json"),
            &HashMap::new(),
        )
        .unwrap();

        assert_eq!(abi.structs.len(), 1);
        assert_eq!(abi.enums.len(), 1);

        let Token::Enum(e) = abi.enums.into_iter().next().unwrap() else {
            panic!("Enums should only have Token::Enum")
        };

        let Token::Struct(s) = abi.structs.into_iter().next().unwrap() else {
            panic!("Structs should only have Token::Struct")
        };

        if let Token::Array(a) = &s.fields[1].token.as_ref() {
            let Token::Tuple(t) = a.inner.as_ref() else {
                panic!("Expect second field to hold Tuple")
            };

            let Token::Enum(tuple_f1) = t.inners[0].as_ref() else {
                panic!("Expect first tuple element to be Enum")
            };

            assert_eq!(5, tuple_f1.variants.len());
            // Check that copy was properly done
            assert_eq!(tuple_f1, &e);
        }
    }

    #[test]
    fn test_collect_tokens() {
        let sierra_abi = include_str!("../../test_data/cairo_ls_abi.json");
        let sierra = serde_json::from_str::<SierraClass>(sierra_abi).unwrap();
        let tokens = AbiParser::collect_tokens(sierra.abi).unwrap();
        assert_ne!(tokens.enums.len(), 0);
        assert_ne!(tokens.functions.len(), 0);
        assert_ne!(tokens.interfaces.len(), 0);
        assert_ne!(tokens.structs.len(), 0);
    }
}

// @core::array::Array::<dojo::meta::introspect::Ty>
// dojo::meta::layout::Layout
// dojo::meta::introspect::Ty
// core::array::Span::<dojo::meta::introspect::Member>
// @core::array::Array::<dojo::meta::layout::Layout>
// dojo::model::definition::ModelDef
// core::array::Span::<(core::felt252, dojo::meta::introspect::Ty)>
// @core::array::Array::<(core::felt252, dojo::meta::introspect::Ty)>
// dojo::meta::introspect::Struct
// @core::array::Array::<dojo::meta::layout::FieldLayout>
// core::array::Span::<dojo::meta::layout::FieldLayout>
// @core::array::Array::<dojo::meta::introspect::Member>
// dojo::meta::interface::IStoredResource
// dojo::model::interface::IModel
