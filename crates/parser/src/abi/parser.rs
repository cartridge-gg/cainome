use starknet::core::types::contract::{AbiEntry, AbiEvent, SierraClass, TypedAbiEvent};
use std::collections::HashMap;

use crate::tokens::{Array, Composite, CompositeInner, CompositeType, CoreBasic, Function, Token};
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

    ///
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

    ///
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
        let mut tokens_filtered: HashMap<String, Token> = HashMap::new();
        for (name, tokens) in token_candidates.into_iter() {
            if tokens.len() == 1 {
                // Only token with this type path -> we keep it without comparison.
                tokens_filtered.insert(name, tokens[0].clone());
            } else if let Token::Composite(composite_0) = &tokens[0] {
                // Currently, it's hard to know the original generic arguments
                // for each struct/enum member types.
                // The following algorithm simply takes the most abundant
                // type for each member.

                let mut unique_composite = composite_0.clone();
                // Clear the inner list as they will be compared to select
                // the most accurate.
                unique_composite.inners.clear();

                for inner in &composite_0.inners {
                    let mut inner_tokens: HashMap<String, (usize, CompositeInner)> = HashMap::new();

                    for __t in &tokens {
                        for __t_inner in
                            &__t.to_composite().expect("only composite expected").inners
                        {
                            if __t_inner.name != inner.name {
                                continue;
                            }

                            let type_path = __t_inner.token.type_path();

                            let counter = if let Some(c) = inner_tokens.get(&type_path) {
                                (c.0 + 1, c.1.clone())
                            } else {
                                (1, __t_inner.clone())
                            };

                            inner_tokens.insert(type_path, counter);
                        }
                    }

                    // Take the most abundant type path for each members, sorted by
                    // the usize counter in descending order.
                    let mut entries: Vec<_> = inner_tokens.into_iter().collect();
                    entries.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

                    unique_composite.inners.push(entries[0].1 .1.clone());
                }

                tokens_filtered.insert(name, Token::Composite(unique_composite));
            }
        }

        tokens_filtered
    }
}
