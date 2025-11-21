use starknet::core::types::contract::legacy::{
    RawLegacyAbiEntry, RawLegacyMember, RawLegacyStruct,
};
use starknet::core::types::contract::StateMutability;
use std::collections::HashMap;
use std::rc::Rc;

use crate::abi::abi_extensions::TokenConvertible;
use crate::abi::registry::TypeRegistry;
use crate::tokens::{Composite, CompositeType, CoreBasic, FuncInner, Function, Token};
use crate::{CainomeResult, Error, TokenizedAbi};

pub struct AbiParserLegacy {}

impl AbiParserLegacy {
    /// Generates the [`Token`]s from the given ABI string.
    ///
    /// # Arguments
    ///
    /// * `abi` - A string representing the ABI (a JSON array of `RawLegacyAbiEntry`).
    /// * `type_aliases` - Types to be renamed to avoid name clashing of generated types.
    pub fn tokens_from_abi_string(
        abi: &str,
        type_aliases: &HashMap<String, String>,
    ) -> CainomeResult<TokenizedAbi> {
        let abi_entries = Self::parse_abi_string(abi)?;
        let tokenized_abi =
            Self::collect_tokens(&abi_entries, type_aliases).expect("failed tokens parsing");

        Ok(tokenized_abi)
    }

    /// Parses an ABI string to output a `Vec<RawLegacyAbiEntry>`.
    ///
    /// # Arguments
    ///
    /// * `abi` - A string representing the ABI (a JSON array of `RawLegacyAbiEntry`).
    pub fn parse_abi_string(abi: &str) -> CainomeResult<Vec<RawLegacyAbiEntry>> {
        let entries =
            serde_json::from_str::<Vec<RawLegacyAbiEntry>>(abi).map_err(Error::SerdeJson)?;
        Ok(entries)
    }

    /// Parse all tokens in the ABI.
    pub fn collect_tokens(
        entries: &[RawLegacyAbiEntry],
        type_aliases: &HashMap<String, String>,
    ) -> CainomeResult<TokenizedAbi> {
        let mut tokens: HashMap<String, Rc<Token>> = HashMap::new();

        for entry in entries {
            Self::collect_entry_token(entry, &mut tokens)?;
        }

        let mut structs = vec![];
        let mut enums = vec![];
        // This is not memory efficient, but
        // currently the focus is on search speed.
        // To be optimized.
        let mut all_composites: HashMap<String, Composite> = HashMap::new();

        // Apply type aliases only on structs and enums.
        for (_, mut t) in tokens {
            // for (type_path, alias) in type_aliases {
            //     t.apply_alias(type_path, alias);
            // }

            if let Token::Composite(ref c) = t.as_ref() {
                all_composites.insert(c.type_path_no_generic(), c.clone());

                match c.r#type {
                    CompositeType::Struct => structs.push(t),
                    CompositeType::Enum => enums.push(t),
                    _ => (),
                }
            }
        }

        let mut functions = vec![];

        for entry in entries {
            Self::collect_entry_function(entry, &mut all_composites, &mut structs, &mut functions)?;
        }

        let interfaces: HashMap<String, Vec<Token>> = HashMap::new();

        Ok(TokenizedAbi {
            enums: enums.into_iter().map(|i| i.as_ref().clone()).collect(),
            structs: structs.into_iter().map(|i| i.as_ref().clone()).collect(),
            functions: functions.into_iter().map(|i| i.as_ref().clone()).collect(),
            interfaces,
        })
    }

    /// Collects the token from the ABI entry.
    ///
    /// # Arguments
    ///
    /// * `entry` - The ABI entry to collect tokens from.
    /// * `tokens` - The list of tokens already collected.
    fn collect_entry_token(
        entry: &RawLegacyAbiEntry,
        registry: &mut HashMap<String, Rc<Token>>,
    ) -> CainomeResult<()> {
        match entry {
            RawLegacyAbiEntry::Struct(s) => {
                // Some struct may be basics, we want to skip them.
                // if CoreBasic::parse(&s.name).is_ok() {
                //     return Ok(());
                // };

                let token: Token = s.to_token(registry)?;
                registry.insert(token.type_path(), Rc::new(token));
            }
            RawLegacyAbiEntry::Event(ev) => {
                let token: Token = ev.to_token(registry)?;
                registry.insert(token.type_path(), Rc::new(token));
            }
            _ => (),
        };

        Ok(())
    }

    /// Collects the function from the ABI entry.
    ///
    /// # Arguments
    ///
    /// * `entry` - The ABI entry to collect functions from.
    /// * `all_composites` - All known composites tokens.
    /// * `structs` - The list of structs already collected.
    /// * `functions` - The list of functions already collected.
    fn collect_entry_function(
        entry: &RawLegacyAbiEntry,
        all_composites: &mut HashMap<String, Composite>,
        structs: &mut Vec<Rc<Token>>,
        functions: &mut Vec<Rc<Token>>,
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
            if let Token::Composite(ref c) = parsed_token.as_ref() {
                match all_composites.get(&c.type_path_no_generic()) {
                    Some(e) => Ok(Token::Composite(e.clone())),
                    None => Ok(parsed_token.as_ref().clone()),
                }
            } else {
                Ok(parsed_token.as_ref().clone())
            }
        }

        // TODO: optimize the search and data structures.
        // HashMap would be more appropriate than vec.
        if let RawLegacyAbiEntry::Function(f) = entry {
            // Looks like in Cairo 0 ABI, if no mutability is given, it's an external.
            let mutability = match f.state_mutability {
                Some(_) => StateMutability::View,
                None => StateMutability::External,
            };

            let mut func = Function::new(&f.name, mutability.into());

            for i in &f.inputs {
                let token = get_existing_token_or_parsed(&i.r#type, all_composites)?;
                func.inputs.push(FuncInner {
                    name: i.name.clone(),
                    token: Rc::new(token),
                });
            }

            for o in &f.outputs {
                let token = get_existing_token_or_parsed(&o.r#type, all_composites)?;
                func.named_outputs.push(FuncInner {
                    name: o.name.clone(),
                    token: Rc::new(token),
                });
            }

            if !func.named_outputs.is_empty() {
                let mut members = vec![];

                for (offset, i) in func.named_outputs.iter().enumerate() {
                    members.push(RawLegacyMember {
                        name: i.name.clone(),
                        offset: offset.try_into().unwrap(),
                        r#type: i.token.type_path().clone(),
                    });
                }

                let s = &RawLegacyStruct {
                    members,
                    name: func.get_cairo0_output_name(),
                    size: func.named_outputs.len() as u64,
                };

                let mut registry = &mut TypeRegistry::new();
                let z = s.to_token(&mut registry)?;

                structs.push(Rc::new(z));
            }

            functions.push(Rc::new(Token::Function(func)));
        }

        Ok(())
    }
}
