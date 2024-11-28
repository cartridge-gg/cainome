use starknet::core::types::contract::{AbiEntry, AbiEvent, SierraClass, TypedAbiEvent};
use std::collections::HashMap;

use crate::tokens::{Array, Composite, CompositeType, CoreBasic, Function, Token};
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
            AbiParser::collect_tokens(&abi_entries, type_aliases).expect("failed tokens parsing");

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

    /// Parse all tokens in the ABI.
    pub fn collect_tokens(
        entries: &[AbiEntry],
        type_aliases: &HashMap<String, String>,
    ) -> CainomeResult<TokenizedAbi> {
        let mut token_candidates: HashMap<String, Vec<Token>> = HashMap::new();

        // Entry tokens are structs, enums and events (which are structs and enums).
        for entry in entries {
            Self::collect_entry_token(entry, &mut token_candidates)?;
        }

        let tokens = Self::filter_struct_enum_tokens(token_candidates);

        let mut structs = vec![];
        let mut enums = vec![];
        // This is not memory efficient, but
        // currently the focus is on search speed.
        // To be optimized.
        let mut all_composites: HashMap<String, Composite> = HashMap::new();

        // Apply type aliases only on structs and enums.
        for (_, mut t) in tokens {
            for (type_path, alias) in type_aliases {
                t.apply_alias(type_path, alias);
            }

            if let Token::Composite(ref c) = t {
                all_composites.insert(c.type_path_no_generic(), c.clone());

                match c.r#type {
                    CompositeType::Struct => structs.push(t),
                    CompositeType::Enum => enums.push(t),
                    _ => (),
                }
            }
        }

        let mut functions = vec![];
        let mut interfaces: HashMap<String, Vec<Token>> = HashMap::new();

        for entry in entries {
            Self::collect_entry_function(
                entry,
                &all_composites,
                &mut functions,
                &mut interfaces,
                None,
            )?;
        }

        Ok(TokenizedAbi {
            enums,
            structs,
            functions,
            interfaces,
        })
    }

    /// Collects the function from the ABI entry.
    ///
    /// # Arguments
    ///
    /// * `entry` - The ABI entry to collect functions from.
    /// * `all_composites` - All known composites tokens.
    /// * `functions` - The list of functions already collected.
    /// * `interfaces` - The list of interfaces already collected.
    /// * `interface_name` - The name of the interface (if any).
    fn collect_entry_function(
        entry: &AbiEntry,
        all_composites: &HashMap<String, Composite>,
        functions: &mut Vec<Token>,
        interfaces: &mut HashMap<String, Vec<Token>>,
        interface_name: Option<String>,
    ) -> CainomeResult<()> {
        /// Gets the existing token into known composite, if any.
        /// Otherwise, return the parsed token.
        fn get_existing_token_or_parsed(
            type_path: &str,
            all_composites: &HashMap<String, Composite>,
        ) -> CainomeResult<Token> {
            let parsed_token = Token::parse(type_path)?;

            // If the token is an known struct or enum, we look up
            // in existing one to get full info from there as the parsing
            // of composites is already done before functions.
            if let Token::Composite(ref c) = parsed_token {
                match all_composites.get(&c.type_path_no_generic()) {
                    Some(e) => Ok(Token::Composite(e.clone())),
                    None => Ok(parsed_token),
                }
            } else {
                Ok(parsed_token)
            }
        }

        // TODO: optimize the search and data structures.
        // HashMap would be more appropriate than vec.
        match entry {
            AbiEntry::Function(f) => {
                let mut func = Function::new(&f.name, f.state_mutability.clone().into());

                for i in &f.inputs {
                    let token = get_existing_token_or_parsed(&i.r#type, all_composites)?;
                    func.inputs.push((i.name.clone(), token));
                }

                for o in &f.outputs {
                    let token = get_existing_token_or_parsed(&o.r#type, all_composites)?;
                    func.outputs.push(token);
                }

                if let Some(name) = interface_name {
                    interfaces
                        .entry(name)
                        .or_default()
                        .push(Token::Function(func));
                } else {
                    functions.push(Token::Function(func));
                }
            }
            AbiEntry::Interface(interface) => {
                for entry in &interface.items {
                    Self::collect_entry_function(
                        entry,
                        all_composites,
                        functions,
                        interfaces,
                        Some(interface.name.clone()),
                    )?;
                }
            }
            _ => (),
        }

        Ok(())
    }

    /// Collects the token from the ABI entry.
    ///
    /// # Arguments
    ///
    /// * `entry` - The ABI entry to collect tokens from.
    /// * `tokens` - The list of tokens already collected.
    fn collect_entry_token(
        entry: &AbiEntry,
        tokens: &mut HashMap<String, Vec<Token>>,
    ) -> CainomeResult<()> {
        match entry {
            AbiEntry::Struct(s) => {
                if Array::parse(&s.name).is_ok() {
                    // Spans can be found as a struct entry in the ABI. We don't want
                    // them as Composite, they are considered as arrays.
                    return Ok(());
                };

                // Some struct may be basics, we want to skip them.
                if CoreBasic::parse(&s.name).is_ok() {
                    return Ok(());
                };

                let token: Token = s.try_into()?;
                let entry = tokens.entry(token.type_path()).or_default();
                entry.push(token);
            }
            AbiEntry::Enum(e) => {
                // Some enums may be basics, we want to skip them.
                if CoreBasic::parse(&e.name).is_ok() {
                    return Ok(());
                };

                let token: Token = e.try_into()?;
                let entry = tokens.entry(token.type_path()).or_default();
                entry.push(token);
            }
            AbiEntry::Event(ev) => {
                let mut token: Token;
                match ev {
                    AbiEvent::Typed(typed_e) => match typed_e {
                        TypedAbiEvent::Struct(s) => {
                            // Some enums may be basics, we want to skip them.
                            if CoreBasic::parse(&s.name).is_ok() {
                                return Ok(());
                            };

                            token = s.try_into()?;
                        }
                        TypedAbiEvent::Enum(e) => {
                            // Some enums may be basics, we want to skip them.
                            if CoreBasic::parse(&e.name).is_ok() {
                                return Ok(());
                            };

                            token = e.try_into()?;

                            // All types inside an event enum are also events.
                            // To ensure correctness of the tokens, we
                            // set the boolean is_event to true for each variant
                            // inner token (if any).

                            // An other solution would have been to looks for the type
                            // inside existing tokens, and clone it. More computation,
                            // but less logic.

                            // An enum if a composite, safe to expect here.
                            if let Token::Composite(ref mut c) = token {
                                for i in &mut c.inners {
                                    if let Token::Composite(ref mut ic) = i.token {
                                        ic.is_event = true;
                                    }
                                }
                            }
                        }
                    },
                    AbiEvent::Untyped(_) => {
                        // Cairo 0.
                        return Ok(());
                    }
                };

                let entry = tokens.entry(token.type_path()).or_default();
                entry.push(token);
            }
            AbiEntry::Interface(interface) => {
                for entry in &interface.items {
                    Self::collect_entry_token(entry, tokens)?;
                }
            }
            _ => (),
        };

        Ok(())
    }

    fn filter_struct_enum_tokens(
        token_candidates: HashMap<String, Vec<Token>>,
    ) -> HashMap<String, Token> {
        let tokens_filtered = Self::filter_token_candidates(token_candidates);

        // Can be a very huge copy here. Need an other way to do that in the loop
        // above here.
        let filtered = tokens_filtered.clone();

        // So now once it's filtered, we may actually iterate again on the tokens
        // to resolve all structs/enums inners that may reference existing types.
        Self::hydrate_composites(tokens_filtered, filtered)
    }

    /// ABI is a flat list of tokens that represents any types declared in cairo code.
    /// We need therefore to filter them out and resolve generic types.
    /// * `token_candidates` - A map of type name to a list of tokens that can be a type.
    ///
    fn filter_token_candidates(
        token_candidates: HashMap<String, Vec<Token>>,
    ) -> HashMap<String, Token> {
        token_candidates
            .into_iter()
            .filter_map(|(name, tokens)| {
                if tokens.is_empty() {
                    return None;
                }

                if tokens.len() == 1 {
                    // Only token with this type path -> we keep it without comparison.
                    return Some((name, tokens[0].clone()));
                }

                if let Token::Composite(composite_0) = &tokens[0] {
                    let unique_composite = composite_0.clone();
                    let inners = composite_0
                        .inners
                        .iter()
                        .map(|inner| {
                            let inner_tokens = tokens
                                .iter()
                                .filter_map(|__t| {
                                    __t.to_composite().ok().and_then(|comp| {
                                        comp.inners
                                            .iter()
                                            .find(|__t_inner| __t_inner.name == inner.name)
                                    })
                                })
                                .fold(HashMap::new(), |mut acc, __t_inner| {
                                    let type_path = __t_inner.token.type_path();
                                    let counter = acc
                                        .entry(type_path.clone())
                                        .or_insert((0, __t_inner.clone()));
                                    counter.0 += 1;
                                    acc
                                });

                            // Take the most abundant type path for each member, sorted by the usize counter in descending order.
                            inner_tokens
                                .into_iter()
                                .max_by_key(|(_, (count, _))| *count)
                                .map(|(_, (_, inner))| inner)
                                .unwrap()
                        })
                        .collect();

                    let mut unique_composite = unique_composite;
                    unique_composite.inners = inners;

                    return Some((name, Token::Composite(unique_composite)));
                }

                None
            })
            .collect()
    }

    fn hydrate_composites(
        tokens_filtered: HashMap<String, Token>,
        filtered: HashMap<String, Token>,
    ) -> HashMap<String, Token> {
        tokens_filtered
            .into_iter()
            .fold(HashMap::new(), |mut acc, (name, token)| {
                acc.insert(name, Token::hydrate(token, &filtered));
                acc
            })
    }
}

#[cfg(test)]
mod tests {
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
                        token: Token::CoreBasic(CoreBasic {
                            type_path: "()".to_owned(),
                        }),
                    },
                    CompositeInner {
                        index: 1,
                        name: "North".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Token::CoreBasic(CoreBasic {
                            type_path: "()".to_owned(),
                        }),
                    },
                    CompositeInner {
                        index: 2,
                        name: "South".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Token::CoreBasic(CoreBasic {
                            type_path: "()".to_owned(),
                        }),
                    },
                    CompositeInner {
                        index: 3,
                        name: "West".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Token::CoreBasic(CoreBasic {
                            type_path: "()".to_owned(),
                        }),
                    },
                    CompositeInner {
                        index: 4,
                        name: "East".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Token::CoreBasic(CoreBasic {
                            type_path: "()".to_owned(),
                        }),
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
                        token: Token::CoreBasic(CoreBasic {
                            type_path: "core::starknet::contract_address::ContractAddress"
                                .to_owned(),
                        }),
                    },
                    CompositeInner {
                        index: 1,
                        name: "directions".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Token::Array(Array {
                            is_legacy: false,
                            type_path: "core::array::Array::<dojo_starter::models::Direction>"
                                .to_owned(),
                            inner: Box::new(Token::Composite(Composite {
                                type_path: "dojo_starter::models::Direction".to_owned(),
                                inners: vec![],
                                generic_args: vec![],
                                r#type: CompositeType::Unknown,
                                is_event: false,
                                alias: None,
                            })),
                        }),
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
                            token: Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            }),
                        },
                        CompositeInner {
                            index: 1,
                            name: "Armor".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            }),
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
                            token: Token::CoreBasic(CoreBasic {
                                type_path: "core::integer::u8".to_owned(),
                            }),
                        },
                        CompositeInner {
                            index: 1,
                            name: "Armor".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Token::CoreBasic(CoreBasic {
                                type_path: "core::integer::u8".to_owned(),
                            }),
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
                            token: Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            }),
                        },
                        CompositeInner {
                            index: 1,
                            name: "Armor".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            }),
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
                            token: Token::CoreBasic(CoreBasic {
                                type_path: "core::integer::u64".to_owned(),
                            }),
                        },
                        CompositeInner {
                            index: 1,
                            name: "name".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            }),
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
                            token: Token::CoreBasic(CoreBasic {
                                type_path: "core::integer::u128".to_owned(),
                            }),
                        },
                        CompositeInner {
                            index: 1,
                            name: "name".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            }),
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
                            token: Token::CoreBasic(CoreBasic {
                                type_path: "core::integer::u64".to_owned(),
                            }),
                        },
                        CompositeInner {
                            index: 1,
                            name: "name".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Token::CoreBasic(CoreBasic {
                                type_path: "core::felt252".to_owned(),
                            }),
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
        if let Token::Array(a) = &s.inners[1].token {
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
        if let Token::Array(a) = &s.inners[1].token {
            if let Token::Tuple(t) = *a.inner.to_owned() {
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
                        token: Token::Composite(Composite {
                            type_path:
                                "tournament::ls15_components::models::tournament::EntryCriteria"
                                    .to_owned(),
                            inners: vec![
                                CompositeInner {
                                    index: 0,
                                    name: "token_id".to_owned(),
                                    kind: CompositeInnerKind::NotUsed,
                                    token: Token::CoreBasic(CoreBasic {
                                        type_path: "core::integer::u128".to_owned(),
                                    }),
                                },
                                CompositeInner {
                                    index: 1,
                                    name: "entry_count".to_owned(),
                                    kind: CompositeInnerKind::NotUsed,
                                    token: Token::CoreBasic(CoreBasic {
                                        type_path: "core::integer::u64".to_owned(),
                                    }),
                                },
                            ],
                            generic_args: vec![],
                            r#type: CompositeType::Struct,
                            is_event: false,
                            alias: None,
                        }),
                    },
                    CompositeInner {
                        index: 1,
                        name: "uniform".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Token::CoreBasic(CoreBasic {
                            type_path: "core::integer::u64".to_owned(),
                        }),
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
                        token: Token::CoreBasic(CoreBasic {
                            type_path: "core::starknet::contract_address::ContractAddress"
                                .to_owned(),
                        }),
                    },
                    CompositeInner {
                        index: 1,
                        name: "entry_type".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Token::Composite(Composite {
                            type_path:
                                "tournament::ls15_components::models::tournament::GatedEntryType"
                                    .to_owned(),
                            inners: vec![],
                            generic_args: vec![],
                            r#type: CompositeType::Unknown,
                            is_event: false,
                            alias: None,
                        }),
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
            token: Token::Composite(
                Composite {
                    type_path: "tournament::ls15_components::models::tournament::GatedToken".to_owned(),
                    inners: vec![
                        CompositeInner {
                            index: 0,
                            name: "token".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Token::CoreBasic(
                                CoreBasic {
                                    type_path: "core::starknet::contract_address::ContractAddress".to_owned(),
                                },
                            ),
                        },
                        CompositeInner {
                            index: 1,
                            name: "entry_type".to_owned(),
                            kind: CompositeInnerKind::NotUsed,
                            token: Token::Composite(
                                Composite {
                                    type_path: "tournament::ls15_components::models::tournament::GatedEntryType".to_owned(),
                                    inners: vec![],
                                    generic_args: vec![],
                                    r#type: CompositeType::Unknown,
                                    is_event: false,
                                    alias: None,
                                },
                            ),
                        },
                    ],
                    generic_args: vec![],
                    r#type: CompositeType::Struct,
                    is_event: false,
                    alias: None,
                },
            ),
        },
        CompositeInner {
            index: 1,
            name: "tournament".to_owned(),
            kind: CompositeInnerKind::NotUsed,
            token: Token::Array(
                Array {
                    type_path: "core::array::Span::<core::integer::u64>".to_owned(),
                    inner: Box::new(Token::CoreBasic(
                        CoreBasic {
                            type_path: "core::integer::u64".to_owned(),
                        },
                    )),
                    is_legacy: false,
                },
            ),
        },
        CompositeInner {
            index: 2,
            name: "address".to_owned(),
            kind: CompositeInnerKind::NotUsed,
            token: Token::Array(
                Array {
                    type_path: "core::array::Span::<core::starknet::contract_address::ContractAddress>".to_owned(),
                    inner: Box::new(
                        Token::CoreBasic(
                        CoreBasic {
                            type_path: "core::starknet::contract_address::ContractAddress".to_owned(),
                        },
                    )
                    ),
                    is_legacy: false,
                },
            ),
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
                    token: Token::Composite(Composite { type_path: "core::option::Option::<tournament::ls15_components::models::tournament::GatedType>".to_owned(), inners: vec![], generic_args: vec![
                ("A".to_owned(), Token::Composite(Composite { type_path: "tournament::ls15_components::models::tournament::GatedType".to_owned(), inners: vec![], generic_args: vec![], r#type: CompositeType::Unknown, is_event: false, alias: None })),
                    ], r#type: CompositeType::Unknown, is_event: false, alias: None }),
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
        if let Token::Composite(c) = &tmv.inners[0].token {
            if let Token::Composite(cc) = &c.generic_args[0].1 {
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
}
