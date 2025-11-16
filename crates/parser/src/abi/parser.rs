use starknet::core::types::contract::{AbiEntry, AbiEvent, SierraClass, TypedAbiEvent};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use crate::abi::conversions::TokenConvertible;
use crate::tokens::{Array, Composite, CompositeType, CoreBasic, FuncInner, Function, Token};
use crate::{CainomeResult, Error};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TokenizedAbi {
    /// All enums found in the contract ABI.
    pub enums: Vec<Token>,
    /// All structs found in the contract ABI.
    pub structs: Vec<Token>,
    /// Standalone functions in the contract ABI.
    pub functions: Vec<Token>,
    /// Fully qualified interface name mapped to all the defined functions in it.
    pub interfaces: HashMap<String, Vec<Token>>,
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
        let tokenized_abi =
            AbiParser::collect_tokens(abi_entries, type_aliases).expect("failed tokens parsing");

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
    
    pub fn has_unknown_dependencies(entry: &AbiEntry, registry: &HashMap<String, Rc<Token>>) -> bool {
        match entry {
            AbiEntry::Function(abi_function) => {
                let has_unknown_input = abi_function.inputs.iter().any(|m| !registry.contains_key(&m.r#type));
                let has_unknown_output = abi_function.outputs.iter().any(|m| !registry.contains_key(&m.r#type));
                has_unknown_input || has_unknown_output
            },
            AbiEntry::Event(abi_event) => {
                match abi_event {
                    AbiEvent::Typed(typed_abi_event) => {
                        match typed_abi_event {
                            TypedAbiEvent::Struct(abi_event_struct) => {
                                abi_event_struct.members.iter().any(|m| !registry.contains_key(&m.r#type))
                            },
                            TypedAbiEvent::Enum(abi_event_enum) => {
                                abi_event_enum.variants.iter().any(|m| !registry.contains_key(&m.r#type))
                            },
                        }
                    },
                    AbiEvent::Untyped(untyped_abi_event) => {
                        untyped_abi_event.inputs.iter().any(|m| !registry.contains_key(&m.r#type))
                    },
                }
            },
            AbiEntry::Struct(abi_struct) => {
                abi_struct.members.iter().any(|m| !registry.contains_key(&m.r#type))
            },
            AbiEntry::Enum(abi_enum) => {
                abi_enum.variants.iter().any(|m| !registry.contains_key(&m.r#type))
            },
            AbiEntry::Constructor(abi_constructor) => {
                abi_constructor.inputs.iter().any(|m| !registry.contains_key(&m.r#type))
            },
            AbiEntry::Interface(abi_interface) => {
                abi_interface.items.iter().any(|m| Self::has_unknown_dependencies(m, registry))
            },
            AbiEntry::L1Handler(abi_function) => {
                let has_unknown_input = abi_function.inputs.iter().any(|m| !registry.contains_key(&m.r#type));
                let has_unknown_output = abi_function.outputs.iter().any(|m| !registry.contains_key(&m.r#type));
                has_unknown_input || has_unknown_output
            },
            AbiEntry::Impl(abi_impl) => {
                !registry.contains_key(&abi_impl.interface_name)
            },
        }
    }


    /// Parse all tokens in the ABI.
    pub fn collect_tokens(
        entries: Vec<AbiEntry>,
        type_aliases: &HashMap<String, String>,
    ) -> CainomeResult<TokenizedAbi> {
        let mut registry: HashMap<String, Rc<Token>> = HashMap::new();
        let mut local_entries = VecDeque::from(entries.clone());
        let mut seen_since_last_removal = 0;

        while local_entries.len() > 0 {
            if seen_since_last_removal > local_entries.len() {
                // TODO: sort out
                return Err(Error::ParsingFailed("Something went wromg. Indirect recursion most likely.".to_string()))
            }
            
            let entry = local_entries.pop_front().expect("Should always succeed");
            seen_since_last_removal += 1;
            
            if Self::has_unknown_dependencies(&entry, &registry) {
                local_entries.push_back(entry);
            } else {
                let token = entry.to_token(&mut registry)?;
                registry.insert(token.type_name(), Rc::new(token));
                seen_since_last_removal = 0;
            }
        }

        let tokens = registry.values();

        let mut structs = vec![];
        let mut enums = vec![];
        let mut functions = vec![];
        let mut interfaces: HashMap<String, Vec<Token>> = HashMap::new();

        for token in tokens {
            match token.as_ref() {
                Token::Function(function) => {
                    functions.push(Token::Function(function.clone()));
                },
                _ => ()
            }
        }

        Ok(TokenizedAbi {
            enums,
            structs,
            functions,
            interfaces,
        })
    }

}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::tokens::{CompositeInner, CompositeInnerKind, CompositeType};

    #[test]
    fn test_filter_token_candidates_single_inner() {
        let mut input: HashMap<String, Vec<Token>> = HashMap::new();
        input.insert(
            "dojo_starter::models::Direction".to_owned(),
            vec![Token::Composite(Composite {
                type_path: "dojo_starter::models::Direction".to_owned(),
                inners: vec![
                    CompositeInner {
                        index: 0,
                        name: "None".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::CoreBasic(CoreBasic {
                            type_path: "()".to_owned(),
                        })),
                    },
                    CompositeInner {
                        index: 1,
                        name: "North".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::CoreBasic(CoreBasic {
                            type_path: "()".to_owned(),
                        })),
                    },
                    CompositeInner {
                        index: 2,
                        name: "South".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::CoreBasic(CoreBasic {
                            type_path: "()".to_owned(),
                        })),
                    },
                    CompositeInner {
                        index: 3,
                        name: "West".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::CoreBasic(CoreBasic {
                            type_path: "()".to_owned(),
                        })),
                    },
                    CompositeInner {
                        index: 4,
                        name: "East".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::CoreBasic(CoreBasic {
                            type_path: "()".to_owned(),
                        })),
                    },
                ],
                generic_args: vec![],
                r#type: CompositeType::Enum,
                is_event: false,
                alias: None,
            })],
        );
        input.insert(
            "dojo_starter::models::DirectionsAvailable".to_owned(),
            vec![Token::Composite(Composite {
                type_path: "dojo_starter::models::DirectionsAvailable".to_owned(),
                inners: vec![
                    CompositeInner {
                        index: 0,
                        name: "player".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::CoreBasic(CoreBasic {
                            type_path: "core::starknet::contract_address::ContractAddress"
                                .to_owned(),
                        })),
                    },
                    CompositeInner {
                        index: 1,
                        name: "directions".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::Array(Array {
                            is_legacy: false,
                            type_path: "core::array::Array::<dojo_starter::models::Direction>"
                                .to_owned(),
                            inner: Rc::new(Token::Composite(Composite {
                                type_path: "dojo_starter::models::Direction".to_owned(),
                                inners: vec![],
                                generic_args: vec![],
                                r#type: CompositeType::Unknown,
                                is_event: false,
                                alias: None,
                            })),
                        })),
                    },
                ],
                generic_args: vec![],
                r#type: CompositeType::Struct,
                is_event: false,
                alias: None,
            })],
        );
        let filtered = AbiParser::filter_token_candidates(input);
        assert_eq!(2, filtered.len());
        assert!(filtered.contains_key("dojo_starter::models::Direction"));
        assert!(filtered.contains_key("dojo_starter::models::DirectionsAvailable"));
    }

    #[test]
    fn test_filter_token_candidates_multiple_composites() {
        let mut input = HashMap::new();

        // First composite: Enum with multiple variants
        input.insert(
            "game::models::ItemType".to_owned(),
            vec![
                Token::Composite(Composite {
                    type_path: "game::models::ItemType".to_owned(),
                    inners: vec![
                        CompositeInner {
                            index: 0,
                            name: "Weapon".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            })),
                        },
                        CompositeInner {
                            index: 1,
                            name: "Armor".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            })),
                        },
                    ],
                    generic_args: vec![],
                    r#type: CompositeType::Enum,
                    is_event: false,
                    alias: None,
                }),
                Token::Composite(Composite {
                    type_path: "game::models::ItemType".to_owned(),
                    inners: vec![
                        CompositeInner {
                            index: 0,
                            name: "Weapon".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::CoreBasic(CoreBasic {
                                type_path: "core::integer::u8".to_owned(),
                            })),
                        },
                        CompositeInner {
                            index: 1,
                            name: "Armor".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::CoreBasic(CoreBasic {
                                type_path: "core::integer::u8".to_owned(),
                            })),
                        },
                    ],
                    generic_args: vec![],
                    r#type: CompositeType::Enum,
                    is_event: false,
                    alias: None,
                }),
                Token::Composite(Composite {
                    type_path: "game::models::ItemType".to_owned(),
                    inners: vec![
                        CompositeInner {
                            index: 0,
                            name: "Weapon".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            })),
                        },
                        CompositeInner {
                            index: 1,
                            name: "Armor".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            })),
                        },
                    ],
                    generic_args: vec![],
                    r#type: CompositeType::Enum,
                    is_event: false,
                    alias: None,
                }),
            ],
        );

        // Second composite: Struct with different types for a member
        input.insert(
            "game::models::Player".to_owned(),
            vec![
                Token::Composite(Composite {
                    type_path: "game::models::Player".to_owned(),
                    inners: vec![
                        CompositeInner {
                            index: 0,
                            name: "id".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::CoreBasic(CoreBasic {
                                type_path: "core::integer::u64".to_owned(),
                            })),
                        },
                        CompositeInner {
                            index: 1,
                            name: "name".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            })),
                        },
                    ],
                    generic_args: vec![],
                    r#type: CompositeType::Struct,
                    is_event: false,
                    alias: None,
                }),
                Token::Composite(Composite {
                    type_path: "game::models::Player".to_owned(),
                    inners: vec![
                        CompositeInner {
                            index: 0,
                            name: "id".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::CoreBasic(CoreBasic {
                                type_path: "core::integer::u128".to_owned(),
                            })),
                        },
                        CompositeInner {
                            index: 1,
                            name: "name".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            })),
                        },
                    ],
                    generic_args: vec![],
                    r#type: CompositeType::Struct,
                    is_event: false,
                    alias: None,
                }),
                Token::Composite(Composite {
                    type_path: "game::models::Player".to_owned(),
                    inners: vec![
                        CompositeInner {
                            index: 0,
                            name: "id".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::CoreBasic(CoreBasic {
                                type_path: "core::integer::u64".to_owned(),
                            })),
                        },
                        CompositeInner {
                            index: 1,
                            name: "name".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            })),
                        },
                    ],
                    generic_args: vec![],
                    r#type: CompositeType::Struct,
                    is_event: false,
                    alias: None,
                }),
            ],
        );

        let filtered = AbiParser::filter_token_candidates(input);

        assert_eq!(2, filtered.len());
        assert!(filtered.contains_key("game::models::ItemType"));
        assert!(filtered.contains_key("game::models::Player"));

        // Check ItemType
        let item_type = filtered
            .get("game::models::ItemType")
            .unwrap()
            .to_composite()
            .unwrap();
        assert_eq!(item_type.inners.len(), 2);
        assert_eq!(item_type.inners[0].name, "Weapon");
        assert_eq!(item_type.inners[1].name, "Armor");
        // The most abundant type should be chosen (felt252 in this case)
        assert_eq!(item_type.inners[0].token.type_path(), "core::felt252");
        assert_eq!(item_type.inners[1].token.type_path(), "core::felt252");

        // Check Player
        let player = filtered
            .get("game::models::Player")
            .unwrap()
            .to_composite()
            .unwrap();
        assert_eq!(player.inners.len(), 2);
        assert_eq!(player.inners[0].name, "id");
        assert_eq!(player.inners[1].name, "name");
        // The most abundant type should be chosen (u64 for id, felt252 for name)
        assert_eq!(player.inners[0].token.type_path(), "core::integer::u64");
        assert_eq!(player.inners[1].token.type_path(), "core::felt252");
    }

    #[test]
    fn test_parse_abi_struct() {
        let abi_json = r#"
        [
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
                        "type": "core::zeroable::NonZero"
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

        assert_eq!(result.structs.len(), 1);
        assert_eq!(result.interfaces.len(), 0);
        assert_eq!(result.functions.len(), 0);
        assert_eq!(result.enums.len(), 0);

        let s = result.structs[0].to_composite().unwrap();
        assert_eq!(s.type_path, "package::StructOne");
        assert_eq!(s.r#type, CompositeType::Struct);
        assert_eq!(s.inners.len(), 3);
        assert_eq!(s.inners[0].name, "a");
        assert_eq!(s.inners[1].name, "b");
        assert_eq!(s.inners[2].name, "c");
    }

    #[test]
    fn test_dojo_starter_direction_available_abi() {
        let abi = AbiParser::tokens_from_abi_string(
            include_str!("../../test_data/dojo_starter-directions_available.abi.json"),
            &HashMap::new(),
        )
        .unwrap();

        assert_eq!(abi.structs.len(), 1);
        let s = abi.structs[0].to_composite().unwrap();
        if let Token::Array(a) = &s.inners[1].token.as_ref() {
            let inner_array = a.inner.to_composite().unwrap();
            assert_eq!(5, inner_array.inners.len());
            // Check that copy was properly done
            let src_enum = abi.enums[0].to_composite().unwrap();
            assert_eq!(inner_array, src_enum);
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
        let s = abi.structs[0].to_composite().unwrap();
        if let Token::Array(a) = &s.inners[1].token.as_ref() {
            if let Token::Tuple(t) = a.inner.as_ref() {
                let inner_array = t.inners[0].to_composite().unwrap();
                assert_eq!(5, inner_array.inners.len());
                // Check that copy was properly done
                let src_enum = abi.enums[0].to_composite().unwrap();
                assert_eq!(inner_array, src_enum);
            } else {
                panic!("Expected tuple");
            }
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_composite_generic_args_hydratation() {
        let mut input: HashMap<String, Vec<Token>> = HashMap::new();
        input.insert(
            "tournament::ls15_components::models::tournament::GatedEntryType".to_owned(),
            vec![Token::Composite(Composite {
                type_path: "tournament::ls15_components::models::tournament::GatedEntryType"
                    .to_owned(),
                inners: vec![
                    CompositeInner {
                        index: 0,
                        name: "criteria".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::Composite(Composite {
                            type_path:
                                "tournament::ls15_components::models::tournament::EntryCriteria"
                                    .to_owned(),
                            inners: vec![
                                CompositeInner {
                                    index: 0,
                                    name: "token_id".to_owned(),
                                    kind: CompositeInnerKind::NotUsed,
                                    token: Rc::new(Token::CoreBasic(CoreBasic {
                                        type_path: "core::integer::u128".to_owned(),
                                    })),
                                },
                                CompositeInner {
                                    index: 1,
                                    name: "entry_count".to_owned(),
                                    kind: CompositeInnerKind::NotUsed,
                                    token: Rc::new(Token::CoreBasic(CoreBasic {
                                        type_path: "core::integer::u64".to_owned(),
                                    })),
                                },
                            ],
                            generic_args: vec![],
                            r#type: CompositeType::Struct,
                            is_event: false,
                            alias: None,
                        })),
                    },
                    CompositeInner {
                        index: 1,
                        name: "uniform".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::CoreBasic(CoreBasic {
                            type_path: "core::integer::u64".to_owned(),
                        })),
                    },
                ],
                generic_args: vec![],
                r#type: CompositeType::Enum,
                is_event: false,
                alias: None,
            })],
        );

        input.insert(
            "tournament::ls15_components::models::tournament::GatedToken".to_owned(),
            vec![Token::Composite(Composite {
                type_path: "tournament::ls15_components::models::tournament::GatedToken".to_owned(),
                inners: vec![
                    CompositeInner {
                        index: 0,
                        name: "token".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::CoreBasic(CoreBasic {
                            type_path: "core::starknet::contract_address::ContractAddress"
                                .to_owned(),
                        })),
                    },
                    CompositeInner {
                        index: 1,
                        name: "entry_type".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::Composite(Composite {
                            type_path:
                                "tournament::ls15_components::models::tournament::GatedEntryType"
                                    .to_owned(),
                            inners: vec![],
                            generic_args: vec![],
                            r#type: CompositeType::Unknown,
                            is_event: false,
                            alias: None,
                        })),
                    },
                ],
                generic_args: vec![],
                r#type: CompositeType::Struct,
                is_event: false,
                alias: None,
            })],
        );
        input.insert(
            "tournament::ls15_components::models::tournament::GatedType".to_owned(),
            vec![Token::Composite(
Composite {
    type_path: "tournament::ls15_components::models::tournament::GatedType".to_owned(),
    inners: vec![
        CompositeInner {
            index: 0,
            name: "token".to_owned(),
            kind: CompositeInnerKind::NotUsed,
            token: Rc::new(Token::Composite(
                Composite {
                    type_path: "tournament::ls15_components::models::tournament::GatedToken".to_owned(),
                    inners: vec![
                        CompositeInner {
                            index: 0,
                            name: "token".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::CoreBasic(
                                CoreBasic {
                                    type_path: "core::starknet::contract_address::ContractAddress".to_owned(),
                                },
                            )),
                        },
                        CompositeInner {
                            index: 1,
                            name: "entry_type".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Rc::new(Token::Composite(
                                Composite {
                                    type_path: "tournament::ls15_components::models::tournament::GatedEntryType".to_owned(),
                                    inners: vec![],
                                    generic_args: vec![],
                                    r#type: CompositeType::Unknown,
                                    is_event: false,
                                    alias: None,
                                },
                            )),
                        },
                    ],
                    generic_args: vec![],
                    r#type: CompositeType::Struct,
                    is_event: false,
                    alias: None,
                },
            )),
        },
        CompositeInner {
            index: 1,
            name: "tournament".to_owned(),
            kind: CompositeInnerKind::NotUsed,
            token: Rc::new(Token::Array(
                Array {
                    type_path: "core::array::Span::<core::integer::u64>".to_owned(),
                    inner: Rc::new(Token::CoreBasic(
                        CoreBasic {
                            type_path: "core::integer::u64".to_owned(),
                        },
                    )),
                    is_legacy: false,
                },
            )),
        },
        CompositeInner {
            index: 2,
            name: "address".to_owned(),
            kind: CompositeInnerKind::NotUsed,
            token: Rc::new(Token::Array(
                Array {
                    type_path: "core::array::Span::<core::starknet::contract_address::ContractAddress>".to_owned(),
                    inner: Rc::new(
                        Token::CoreBasic(
                        CoreBasic {
                            type_path: "core::starknet::contract_address::ContractAddress".to_owned(),
                        },
                    )
                    ),
                    is_legacy: false,
                },
            )),
        },
    ],
    generic_args: vec![],
    r#type: CompositeType::Enum,
    is_event: false,
    alias: None,
}            )],
        );
        input.insert(
            "tournament::ls15_components::models::tournament::TournamentModelValue".to_owned(),
            vec![Token::Composite(Composite {
                type_path: "tournament::ls15_components::models::tournament::TournamentModelValue"
                    .to_owned(),
                inners: vec![CompositeInner {
                    index: 0,
                    name: "gated_type".to_owned(),
                    kind: CompositeInnerKind::NotUsed,
                    token: Rc::new(Token::Composite(Composite { 
                        type_path: "core::option::Option::<tournament::ls15_components::models::tournament::GatedType>".to_owned(), 
                        inners: vec![], 
                        generic_args: vec![
                            ("A".to_owned(), Rc::new(Token::Composite(
                                Composite { 
                                    type_path: "tournament::ls15_components::models::tournament::GatedType".to_owned(), 
                                    inners: vec![], 
                                    generic_args: vec![], 
                                    r#type: CompositeType::Unknown, 
                                    is_event: false, 
                                    alias: None 
                                }
                            ))),
                        ], 
                        r#type: CompositeType::Unknown, 
                        is_event: false, 
                        alias: None 
                    }))
                }],
                generic_args: vec![],
                r#type: CompositeType::Struct,
                is_event: false,
                alias: None,
            })],
        );

        let filtered = AbiParser::filter_struct_enum_tokens(input);
        let tmv = filtered
            .get("tournament::ls15_components::models::tournament::TournamentModelValue")
            .unwrap()
            .to_composite()
            .unwrap();
        if let Token::Composite(c) = &tmv.inners[0].token.as_ref() {
            if let Token::Composite(cc) = &c.generic_args[0].1.as_ref() {
                // Checking that inners are not empty ensures us that hydration was done, even for
                // `generic_args`.
                assert_ne!(0, cc.inners.len());
            } else {
                panic!("Expected composite");
            }
        } else {
            panic!("Expected composite");
        }
    }
    #[test]
    fn test_deep_nested_hydration() {
        let mut input: HashMap<String, Vec<Token>> = HashMap::new();
        input.insert(
            "tournament::ls15_components::models::loot_survivor::Item".to_owned(),
            vec![Token::Composite(Composite {
                type_path: "tournament::ls15_components::models::loot_survivor::Item".to_owned(),
                inners: vec![
                    CompositeInner {
                        index: 0,
                        name: "id".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::CoreBasic(CoreBasic {
                            type_path: "core::integer::u8".to_owned(),
                        })),
                    },
                    CompositeInner {
                        index: 1,
                        name: "name".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::CoreBasic(CoreBasic {
                            type_path: "core::integer::u16".to_owned(),
                        })),
                    },
                ],
                generic_args: vec![],
                r#type: CompositeType::Struct,
                is_event: false,
                alias: None,
            })],
        );
        input.insert(
            "tournament::ls15_components::models::loot_survivor::Equipment".to_owned(),
            vec![Token::Composite(Composite {
                type_path: "tournament::ls15_components::models::loot_survivor::Equipment"
                    .to_owned(),
                inners: vec![
                    CompositeInner {
                        index: 0,
                        name: "weapon".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::Composite(Composite {
                            type_path: "tournament::ls15_components::models::loot_survivor::Item"
                                .to_owned(),
                            inners: vec![],
                            generic_args: vec![],
                            r#type: CompositeType::Unknown,
                            is_event: false,
                            alias: None,
                        })),
                    },
                    CompositeInner {
                        index: 1,
                        name: "chest".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::Composite(Composite {
                            type_path: "tournament::ls15_components::models::loot_survivor::Item"
                                .to_owned(),
                            inners: vec![],
                            generic_args: vec![],
                            r#type: CompositeType::Unknown,
                            is_event: false,
                            alias: None,
                        })),
                    },
                ],
                generic_args: vec![],
                r#type: CompositeType::Struct,
                is_event: false,
                alias: None,
            })],
        );
        input.insert(
            "tournament::ls15_components::models::loot_survivor::Adventurer".to_owned(),
            vec![Token::Composite(Composite {
                type_path: "tournament::ls15_components::models::loot_survivor::Adventurer"
                    .to_owned(),
                inners: vec![CompositeInner {
                    index: 0,
                    name: "equipment".to_owned(),
                    kind: CompositeInnerKind::NotUsed,
                    token: Rc::new(Token::Composite(Composite {
                        type_path: "tournament::ls15_components::models::loot_survivor::Equipment"
                            .to_owned(),
                        inners: vec![],
                        generic_args: vec![],
                        r#type: CompositeType::Unknown,
                        is_event: false,
                        alias: None,
                    })),
                }],
                generic_args: vec![],
                r#type: CompositeType::Struct,
                is_event: false,
                alias: None,
            })],
        );
        input.insert(
            "tournament::ls15_components::models::loot_survivor::AdventurerModel".to_owned(),
            vec![Token::Composite(Composite {
                type_path: "tournament::ls15_components::models::loot_survivor::AdventurerModel"
                    .to_owned(),
                inners: vec![
                    CompositeInner {
                        index: 0,
                        name: "adventurer_id".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::CoreBasic(CoreBasic {
                            type_path: "core::felt252".to_owned(),
                        })),
                    },
                    CompositeInner {
                        index: 1,
                        name: "adventurer".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Rc::new(Token::Composite(Composite {
                            type_path:
                                "tournament::ls15_components::models::loot_survivor::Adventurer"
                                    .to_owned(),
                            inners: vec![],
                            generic_args: vec![],
                            r#type: CompositeType::Unknown,
                            is_event: false,
                            alias: None,
                        })),
                    },
                ],
                generic_args: vec![],
                r#type: CompositeType::Struct,
                is_event: false,
                alias: None,
            })],
        );

        let filtered = AbiParser::filter_struct_enum_tokens(input);
        fn check_token_inners(token: &Token) {
            // end of recursion, if token is composite and inners are empty, this means hydration
            // was not properly done.
            if let Token::Composite(c) = token {
                assert_ne!(0, c.inners.len());
                // deep dive into compsite,
                c.inners.iter().for_each(|i| check_token_inners(&i.token));
            }
        }
        filtered.iter().for_each(|(_, t)| check_token_inners(t));
    }

    #[test]
    fn test_collect_tokens() {
        let sierra_abi = include_str!("../../test_data/cairo_ls_abi.json");
        let sierra = serde_json::from_str::<SierraClass>(sierra_abi).unwrap();
        let tokens = AbiParser::collect_tokens(sierra.abi, &HashMap::new()).unwrap();
        assert_ne!(tokens.enums.len(), 0);
        assert_ne!(tokens.functions.len(), 0);
        assert_ne!(tokens.interfaces.len(), 0);
        assert_ne!(tokens.structs.len(), 0);
    }
}
