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

    /// Generates the [`Token`]s from the given ABI string with file context for type aliases.
    ///
    /// The `abi` can have two formats:
    /// 1. Entire [`SierraClass`] json representation.
    /// 2. The `abi` key from the [`SierraClass`], which is an array of [`AbiEntry`].
    ///
    /// # Arguments
    ///
    /// * `abi` - A string representing the ABI.
    /// * `type_aliases` - Types to be renamed to avoid name clashing of generated types.
    /// * `file_name` - Optional file name for type alias disambiguation.
    pub fn tokens_from_abi_string_with_file_context(
        abi: &str,
        type_aliases: &HashMap<String, String>,
        file_name: std::option::Option<&str>,
    ) -> CainomeResult<TokenizedAbi> {
        let abi_entries = Self::parse_abi_string(abi)?;
        let tokenized_abi =
            AbiParser::collect_tokens_with_file_context(&abi_entries, type_aliases, file_name).expect("failed tokens parsing");

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
                type_aliases,
            )?;
        }

        Ok(TokenizedAbi {
            enums,
            structs,
            functions,
            interfaces,
        })
    }

    /// Parse all tokens in the ABI with file context for type aliases.
    pub fn collect_tokens_with_file_context(
        entries: &[AbiEntry],
        type_aliases: &HashMap<String, String>,
        file_name: std::option::Option<&str>,
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
            // Separate file-specific and general aliases
            let mut file_specific_aliases = Vec::new();
            let mut general_aliases = Vec::new();

            for (type_path, alias) in type_aliases {
                // Check if this is a file-specific alias
                if let Some((file_prefix, _)) = type_path.split_once("::") {
                    if let Some(current_file) = file_name {
                        if current_file == file_prefix {
                            file_specific_aliases.push((type_path, alias));
                            continue;
                        }
                    }
                }
                // If not file-specific, treat as general
                general_aliases.push((type_path, alias));
            }

            // Apply file-specific aliases first (highest priority)
            for (type_path, alias) in file_specific_aliases {
                t.apply_alias_with_file_context(type_path, alias, file_name);
            }

            // Then apply general aliases (lower priority, only if no file-specific alias was applied)
            for (type_path, alias) in general_aliases {
                t.apply_alias_with_file_context(type_path, alias, file_name);
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
            Self::collect_entry_function_with_file_context(
                entry,
                &all_composites,
                &mut functions,
                &mut interfaces,
                None,
                type_aliases,
                file_name,
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
        type_aliases: &HashMap<String, String>,
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
                    let mut token = get_existing_token_or_parsed(&i.r#type, all_composites)?;

                    for (alias_type_path, alias) in type_aliases {
                        token.apply_alias(alias_type_path, alias);
                    }

                    func.inputs.push((i.name.clone(), token));
                }

                for o in &f.outputs {
                    let mut token = get_existing_token_or_parsed(&o.r#type, all_composites)?;

                    for (alias_type_path, alias) in type_aliases {
                        token.apply_alias(alias_type_path, alias);
                    }

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
                        type_aliases,
                    )?;
                }
            }
            _ => (),
        }

        Ok(())
    }

    /// Collects the function from the ABI entry with file context for type aliases.
    ///
    /// # Arguments
    ///
    /// * `entry` - The ABI entry to collect functions from.
    /// * `all_composites` - All known composites tokens.
    /// * `functions` - The list of functions already collected.
    /// * `interfaces` - The list of interfaces already collected.
    /// * `interface_name` - The name of the interface (if any).
    /// * `type_aliases` - Types to be renamed to avoid name clashing of generated types.
    /// * `file_name` - Optional file name for type alias disambiguation.
    fn collect_entry_function_with_file_context(
        entry: &AbiEntry,
        all_composites: &HashMap<String, Composite>,
        functions: &mut Vec<Token>,
        interfaces: &mut HashMap<String, Vec<Token>>,
        interface_name: std::option::Option<String>,
        type_aliases: &HashMap<String, String>,
        file_name: std::option::Option<&str>,
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
                    let mut token = get_existing_token_or_parsed(&i.r#type, all_composites)?;

                    for (alias_type_path, alias) in type_aliases {
                        token.apply_alias_with_file_context(alias_type_path, alias, file_name);
                    }

                    func.inputs.push((i.name.clone(), token));
                }

                for o in &f.outputs {
                    let mut token = get_existing_token_or_parsed(&o.r#type, all_composites)?;

                    for (alias_type_path, alias) in type_aliases {
                        token.apply_alias_with_file_context(alias_type_path, alias, file_name);
                    }

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
                    Self::collect_entry_function_with_file_context(
                        entry,
                        all_composites,
                        functions,
                        interfaces,
                        Some(interface.name.clone()),
                        type_aliases,
                        file_name,
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

                let token: Token = s.try_into()?;
                let entry = tokens.entry(token.type_path()).or_default();
                entry.push(token);
            }
            AbiEntry::Enum(e) => {
                // `bool` is a core basic enum, we want to skip it.
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
                acc.insert(name, Token::hydrate(token, &filtered, 10, 0));
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
                        token: Token::CoreBasic(CoreBasic {
                            type_path: "core::integer::u8".to_owned(),
                        }),
                    },
                    CompositeInner {
                        index: 1,
                        name: "name".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Token::CoreBasic(CoreBasic {
                            type_path: "core::integer::u16".to_owned(),
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
            "tournament::ls15_components::models::loot_survivor::Equipment".to_owned(),
            vec![Token::Composite(Composite {
                type_path: "tournament::ls15_components::models::loot_survivor::Equipment"
                    .to_owned(),
                inners: vec![
                    CompositeInner {
                        index: 0,
                        name: "weapon".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Token::Composite(Composite {
                            type_path: "tournament::ls15_components::models::loot_survivor::Item"
                                .to_owned(),
                            inners: vec![],
                            generic_args: vec![],
                            r#type: CompositeType::Unknown,
                            is_event: false,
                            alias: None,
                        }),
                    },
                    CompositeInner {
                        index: 1,
                        name: "chest".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Token::Composite(Composite {
                            type_path: "tournament::ls15_components::models::loot_survivor::Item"
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
            "tournament::ls15_components::models::loot_survivor::Adventurer".to_owned(),
            vec![Token::Composite(Composite {
                type_path: "tournament::ls15_components::models::loot_survivor::Adventurer"
                    .to_owned(),
                inners: vec![CompositeInner {
                    index: 0,
                    name: "equipment".to_owned(),
                    kind: CompositeInnerKind::NotUsed,
                    token: Token::Composite(Composite {
                        type_path: "tournament::ls15_components::models::loot_survivor::Equipment"
                            .to_owned(),
                        inners: vec![],
                        generic_args: vec![],
                        r#type: CompositeType::Unknown,
                        is_event: false,
                        alias: None,
                    }),
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
                        token: Token::CoreBasic(CoreBasic {
                            type_path: "core::felt252".to_owned(),
                        }),
                    },
                    CompositeInner {
                        index: 1,
                        name: "adventurer".to_owned(),
                        kind: CompositeInnerKind::NotUsed,
                        token: Token::Composite(Composite {
                            type_path:
                                "tournament::ls15_components::models::loot_survivor::Adventurer"
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
        let abi = r#"
        [
            {
                "type": "struct",
                "name": "package::StructOne",
                "members": [
                    {
                        "name": "a",
                        "type": "core::integer::u64"
                    }
                ]
            }
        ]
        "#;

        let entries = AbiParser::parse_abi_string(abi).unwrap();
        let tokens = AbiParser::collect_tokens(&entries, &HashMap::new()).unwrap();

        assert_eq!(tokens.structs.len(), 1);
        assert_eq!(tokens.enums.len(), 0);
        assert_eq!(tokens.functions.len(), 0);
        assert_eq!(tokens.interfaces.len(), 0);
    }

    #[test]
    fn test_file_based_type_aliases_basic() {
        let abi = r#"
        [
            {
                "type": "struct",
                "name": "contracts::Token",
                "members": [
                    {
                        "name": "balance",
                        "type": "core::integer::u256"
                    }
                ]
            }
        ]
        "#;

        let mut type_aliases = HashMap::new();
        type_aliases.insert("erc20::contracts::Token".to_string(), "ERC20Token".to_string());
        type_aliases.insert("erc721::contracts::Token".to_string(), "ERC721Token".to_string());

        let entries = AbiParser::parse_abi_string(abi).unwrap();

        // Test with erc20 file context
        let tokens_erc20 = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, Some("erc20")).unwrap();
        assert_eq!(tokens_erc20.structs.len(), 1);
        if let Token::Composite(ref composite) = tokens_erc20.structs[0] {
            assert_eq!(composite.alias, Some("ERC20Token".to_string()));
            assert_eq!(composite.type_name_or_alias(), "ERC20Token");
        } else {
            panic!("Expected composite token");
        }

        // Test with erc721 file context
        let tokens_erc721 = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, Some("erc721")).unwrap();
        assert_eq!(tokens_erc721.structs.len(), 1);
        if let Token::Composite(ref composite) = tokens_erc721.structs[0] {
            assert_eq!(composite.alias, Some("ERC721Token".to_string()));
            assert_eq!(composite.type_name_or_alias(), "ERC721Token");
        } else {
            panic!("Expected composite token");
        }

        // Test with different file context (should not match)
        let tokens_other = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, Some("other")).unwrap();
        assert_eq!(tokens_other.structs.len(), 1);
        if let Token::Composite(ref composite) = tokens_other.structs[0] {
            assert_eq!(composite.alias, None);
            assert_eq!(composite.type_name_or_alias(), "Token");
        } else {
            panic!("Expected composite token");
        }
    }

    #[test]
    fn test_file_based_type_aliases_backward_compatibility() {
        let abi = r#"
        [
            {
                "type": "struct",
                "name": "contracts::Token",
                "members": [
                    {
                        "name": "balance",
                        "type": "core::integer::u256"
                    }
                ]
            }
        ]
        "#;

        let mut type_aliases = HashMap::new();
        // Traditional alias without file prefix
        type_aliases.insert("contracts::Token".to_string(), "GeneralToken".to_string());

        let entries = AbiParser::parse_abi_string(abi).unwrap();

        // Test with any file context - should still work
        let tokens = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, Some("any_file")).unwrap();
        assert_eq!(tokens.structs.len(), 1);
        if let Token::Composite(ref composite) = tokens.structs[0] {
            assert_eq!(composite.alias, Some("GeneralToken".to_string()));
            assert_eq!(composite.type_name_or_alias(), "GeneralToken");
        } else {
            panic!("Expected composite token");
        }

        // Test without file context - should still work
        let tokens_no_context = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, None).unwrap();
        assert_eq!(tokens_no_context.structs.len(), 1);
        if let Token::Composite(ref composite) = tokens_no_context.structs[0] {
            assert_eq!(composite.alias, Some("GeneralToken".to_string()));
            assert_eq!(composite.type_name_or_alias(), "GeneralToken");
        } else {
            panic!("Expected composite token");
        }
    }

    #[test]
    fn test_file_based_type_aliases_priority() {
        let abi = r#"
        [
            {
                "type": "struct",
                "name": "contracts::Token",
                "members": [
                    {
                        "name": "balance",
                        "type": "core::integer::u256"
                    }
                ]
            }
        ]
        "#;

        let mut type_aliases = HashMap::new();
        // Both file-specific and general aliases
        type_aliases.insert("erc20::contracts::Token".to_string(), "ERC20Token".to_string());
        type_aliases.insert("contracts::Token".to_string(), "GeneralToken".to_string());

        let entries = AbiParser::parse_abi_string(abi).unwrap();

        // Test with erc20 file context - should prefer file-specific alias
        let tokens_erc20 = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, Some("erc20")).unwrap();
        assert_eq!(tokens_erc20.structs.len(), 1);
        if let Token::Composite(ref composite) = tokens_erc20.structs[0] {
            assert_eq!(composite.alias, Some("ERC20Token".to_string()));
            assert_eq!(composite.type_name_or_alias(), "ERC20Token");
        } else {
            panic!("Expected composite token");
        }

        // Test with different file context - should fall back to general alias
        let tokens_other = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, Some("other")).unwrap();
        assert_eq!(tokens_other.structs.len(), 1);
        if let Token::Composite(ref composite) = tokens_other.structs[0] {
            assert_eq!(composite.alias, Some("GeneralToken".to_string()));
            assert_eq!(composite.type_name_or_alias(), "GeneralToken");
        } else {
            panic!("Expected composite token");
        }
    }

    #[test]
    fn test_file_based_type_aliases_enum() {
        let abi = r#"
        [
            {
                "type": "enum",
                "name": "contracts::Status",
                "variants": [
                    {
                        "name": "Active",
                        "type": "()"
                    },
                    {
                        "name": "Inactive",
                        "type": "()"
                    }
                ]
            }
        ]
        "#;

        let mut type_aliases = HashMap::new();
        type_aliases.insert("game::contracts::Status".to_string(), "GameStatus".to_string());
        type_aliases.insert("user::contracts::Status".to_string(), "UserStatus".to_string());

        let entries = AbiParser::parse_abi_string(abi).unwrap();

        // Test with game file context
        let tokens_game = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, Some("game")).unwrap();
        assert_eq!(tokens_game.enums.len(), 1);
        if let Token::Composite(ref composite) = tokens_game.enums[0] {
            assert_eq!(composite.alias, Some("GameStatus".to_string()));
            assert_eq!(composite.type_name_or_alias(), "GameStatus");
        } else {
            panic!("Expected composite token");
        }

        // Test with user file context
        let tokens_user = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, Some("user")).unwrap();
        assert_eq!(tokens_user.enums.len(), 1);
        if let Token::Composite(ref composite) = tokens_user.enums[0] {
            assert_eq!(composite.alias, Some("UserStatus".to_string()));
            assert_eq!(composite.type_name_or_alias(), "UserStatus");
        } else {
            panic!("Expected composite token");
        }
    }

    #[test]
    fn test_file_based_type_aliases_nested_structures() {
        let abi = r#"
        [
            {
                "type": "struct",
                "name": "contracts::Token",
                "members": [
                    {
                        "name": "balance",
                        "type": "core::integer::u256"
                    }
                ]
            },
            {
                "type": "struct",
                "name": "contracts::Wallet",
                "members": [
                    {
                        "name": "tokens",
                        "type": "core::array::Array::<contracts::Token>"
                    }
                ]
            }
        ]
        "#;

        let mut type_aliases = HashMap::new();
        type_aliases.insert("erc20::contracts::Token".to_string(), "ERC20Token".to_string());
        type_aliases.insert("erc20::contracts::Wallet".to_string(), "ERC20Wallet".to_string());

        let entries = AbiParser::parse_abi_string(abi).unwrap();

        // Test with erc20 file context
        let tokens_erc20 = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, Some("erc20")).unwrap();
        assert_eq!(tokens_erc20.structs.len(), 2);

        // Find Token struct
        let token_struct = tokens_erc20.structs.iter()
            .find(|t| t.type_path().contains("Token"))
            .expect("Token struct not found");
        if let Token::Composite(ref composite) = token_struct {
            assert_eq!(composite.alias, Some("ERC20Token".to_string()));
        } else {
            panic!("Expected composite token");
        }

        // Find Wallet struct
        let wallet_struct = tokens_erc20.structs.iter()
            .find(|t| t.type_path().contains("Wallet"))
            .expect("Wallet struct not found");
        if let Token::Composite(ref composite) = wallet_struct {
            assert_eq!(composite.alias, Some("ERC20Wallet".to_string()));
        } else {
            panic!("Expected composite token");
        }
    }

    #[test]
    fn test_file_based_type_aliases_functions() {
        let abi = r#"
        [
            {
                "type": "struct",
                "name": "contracts::Token",
                "members": [
                    {
                        "name": "balance",
                        "type": "core::integer::u256"
                    }
                ]
            },
            {
                "type": "function",
                "name": "transfer",
                "inputs": [
                    {
                        "name": "token",
                        "type": "contracts::Token"
                    }
                ],
                "outputs": [
                    {
                        "type": "core::bool"
                    }
                ],
                "state_mutability": "external"
            }
        ]
        "#;

        let mut type_aliases = HashMap::new();
        type_aliases.insert("erc20::contracts::Token".to_string(), "ERC20Token".to_string());

        let entries = AbiParser::parse_abi_string(abi).unwrap();

        // Test with erc20 file context
        let tokens_erc20 = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, Some("erc20")).unwrap();
        assert_eq!(tokens_erc20.structs.len(), 1);
        assert_eq!(tokens_erc20.functions.len(), 1);

        // Check that the Token struct has the alias
        if let Token::Composite(ref composite) = tokens_erc20.structs[0] {
            assert_eq!(composite.alias, Some("ERC20Token".to_string()));
        } else {
            panic!("Expected composite token");
        }

        // Check that the function uses the aliased type
        if let Token::Function(ref function) = tokens_erc20.functions[0] {
            assert_eq!(function.inputs.len(), 1);
            // The function input should contain the aliased token
            if let Token::Composite(ref input_composite) = function.inputs[0].1 {
                assert_eq!(input_composite.alias, Some("ERC20Token".to_string()));
            } else {
                panic!("Expected composite token in function input");
            }
        } else {
            panic!("Expected function token");
        }
    }

    #[test]
    fn test_tokens_from_abi_string_with_file_context() {
        let abi = r#"
        [
            {
                "type": "struct",
                "name": "contracts::Token",
                "members": [
                    {
                        "name": "balance",
                        "type": "core::integer::u256"
                    }
                ]
            }
        ]
        "#;

        let mut type_aliases = HashMap::new();
        type_aliases.insert("erc20::contracts::Token".to_string(), "ERC20Token".to_string());

        // Test the main API method
        let tokens = AbiParser::tokens_from_abi_string_with_file_context(abi, &type_aliases, Some("erc20")).unwrap();
        assert_eq!(tokens.structs.len(), 1);
        
        if let Token::Composite(ref composite) = tokens.structs[0] {
            assert_eq!(composite.alias, Some("ERC20Token".to_string()));
            assert_eq!(composite.type_name_or_alias(), "ERC20Token");
        } else {
            panic!("Expected composite token");
        }

        // Test without file context (should not match file-specific alias)
        let tokens_no_context = AbiParser::tokens_from_abi_string_with_file_context(abi, &type_aliases, None).unwrap();
        assert_eq!(tokens_no_context.structs.len(), 1);
        
        if let Token::Composite(ref composite) = tokens_no_context.structs[0] {
            assert_eq!(composite.alias, None);
            assert_eq!(composite.type_name_or_alias(), "Token");
        } else {
            panic!("Expected composite token");
        }
    }

    #[test]
    fn test_file_based_type_aliases_complex_paths() {
        let abi = r#"
        [
            {
                "type": "struct",
                "name": "my_project::contracts::utils::Token",
                "members": [
                    {
                        "name": "balance",
                        "type": "core::integer::u256"
                    }
                ]
            }
        ]
        "#;

        let mut type_aliases = HashMap::new();
        // Test with complex file name and type path
        type_aliases.insert("erc20_impl::my_project::contracts::utils::Token".to_string(), "ERC20TokenImpl".to_string());

        let entries = AbiParser::parse_abi_string(abi).unwrap();

        // Test with matching file context
        let tokens = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, Some("erc20_impl")).unwrap();
        assert_eq!(tokens.structs.len(), 1);
        
        if let Token::Composite(ref composite) = tokens.structs[0] {
            assert_eq!(composite.alias, Some("ERC20TokenImpl".to_string()));
            assert_eq!(composite.type_name_or_alias(), "ERC20TokenImpl");
        } else {
            panic!("Expected composite token");
        }

        // Test with non-matching file context
        let tokens_no_match = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, Some("other_impl")).unwrap();
        assert_eq!(tokens_no_match.structs.len(), 1);
        
        if let Token::Composite(ref composite) = tokens_no_match.structs[0] {
            assert_eq!(composite.alias, None);
            assert_eq!(composite.type_name_or_alias(), "Token");
        } else {
            panic!("Expected composite token");
        }
    }

    #[test]
    fn test_file_based_type_aliases_empty_file_context() {
        let abi = r#"
        [
            {
                "type": "struct",
                "name": "contracts::Token",
                "members": [
                    {
                        "name": "balance",
                        "type": "core::integer::u256"
                    }
                ]
            }
        ]
        "#;

        let mut type_aliases = HashMap::new();
        type_aliases.insert("erc20::contracts::Token".to_string(), "ERC20Token".to_string());
        type_aliases.insert("contracts::Token".to_string(), "GeneralToken".to_string());

        let entries = AbiParser::parse_abi_string(abi).unwrap();

        // Test with None file context - should use general alias
        let tokens = AbiParser::collect_tokens_with_file_context(&entries, &type_aliases, None).unwrap();
        assert_eq!(tokens.structs.len(), 1);
        
        if let Token::Composite(ref composite) = tokens.structs[0] {
            assert_eq!(composite.alias, Some("GeneralToken".to_string()));
            assert_eq!(composite.type_name_or_alias(), "GeneralToken");
        } else {
            panic!("Expected composite token");
        }
    }
}
