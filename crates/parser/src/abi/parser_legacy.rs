use starknet::core::types::contract::legacy::{
    RawLegacyAbiEntry, RawLegacyMember, RawLegacyStruct,
};
use starknet::core::types::contract::StateMutability;
use std::collections::HashMap;

use crate::tokens::{Composite, CompositeType, CoreBasic, Function, Token};
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
        let mut tokens: HashMap<String, Token> = HashMap::new();

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

        for entry in entries {
            Self::collect_entry_function(entry, &mut all_composites, &mut structs, &mut functions)?;
        }

        let interfaces: HashMap<String, Vec<Token>> = HashMap::new();

        Ok(TokenizedAbi {
            enums,
            structs,
            functions,
            interfaces,
        })
    }

    ///
    fn collect_entry_token(
        entry: &RawLegacyAbiEntry,
        tokens: &mut HashMap<String, Token>,
    ) -> CainomeResult<()> {
        match entry {
            RawLegacyAbiEntry::Struct(s) => {
                // Some struct may be basics, we want to skip them.
                if CoreBasic::parse(&s.name).is_ok() {
                    return Ok(());
                };

                let token: Token = s.try_into()?;
                tokens.insert(token.type_path(), token);
            }
            RawLegacyAbiEntry::Event(ev) => {
                let token: Token = ev.try_into()?;
                tokens.insert(token.type_path(), token);
            }
            _ => (),
        };

        Ok(())
    }

    ///
    fn collect_entry_function(
        entry: &RawLegacyAbiEntry,
        all_composites: &mut HashMap<String, Composite>,
        structs: &mut Vec<Token>,
        functions: &mut Vec<Token>,
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
        if let RawLegacyAbiEntry::Function(f) = entry {
            // Looks like in Cairo 0 ABI, if no mutability is given, it's an external.
            let mutability = match f.state_mutability {
                Some(_) => StateMutability::View,
                None => StateMutability::External,
            };

            let mut func = Function::new(&f.name, mutability.into());

            for i in &f.inputs {
                let token = get_existing_token_or_parsed(&i.r#type, all_composites)?;
                func.inputs.push((i.name.clone(), token));
            }

            for o in &f.outputs {
                let token = get_existing_token_or_parsed(&o.r#type, all_composites)?;
                func.named_outputs.push((o.name.clone(), token));
            }

            if !func.named_outputs.is_empty() {
                let mut members = vec![];

                for (offset, (n, t)) in func.named_outputs.iter().enumerate() {
                    members.push(RawLegacyMember {
                        name: n.clone(),
                        offset: offset.try_into().unwrap(),
                        r#type: t.type_path().clone(),
                    });
                }

                let s = RawLegacyStruct {
                    members,
                    name: func.get_cairo0_output_name(),
                    size: func.named_outputs.len() as u64,
                };

                structs.push((&s).try_into()?);
            }

            functions.push(Token::Function(func));
        }

        Ok(())
    }
}
