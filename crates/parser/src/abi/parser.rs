use starknet::core::types::contract::{AbiEntry, AbiEvent, TypedAbiEvent};
use std::collections::HashMap;

use crate::tokens::{Array, CompositeInner, CompositeType, CoreBasic, Function, Token};
use crate::CainomeResult;

pub struct AbiParser {}

impl AbiParser {
    /// Organizes the tokens by cairo types.
    pub fn organize_tokens(
        tokens: HashMap<String, Token>,
        type_aliases: &HashMap<String, String>,
    ) -> HashMap<String, Vec<Token>> {
        let mut structs = vec![];
        let mut enums = vec![];
        let mut functions = vec![];

        for (_, mut t) in tokens {
            for (type_path, alias) in type_aliases {
                t.apply_alias(type_path, alias);
            }

            match t {
                Token::Composite(ref c) => {
                    match c.r#type {
                        CompositeType::Struct => structs.push(t),
                        CompositeType::Enum => enums.push(t),
                        _ => (), // TODO: warn?
                    }
                }
                Token::Function(_) => functions.push(t),
                _ => (), // TODO: warn?
            }
        }

        let mut out = HashMap::new();
        out.insert("structs".to_string(), structs);
        out.insert("enums".to_string(), enums);
        out.insert("functions".to_string(), functions);
        out
    }

    /// Parse all tokens in the ABI.
    pub fn collect_tokens(entries: &[AbiEntry]) -> CainomeResult<HashMap<String, Token>> {
        let mut token_candidates: HashMap<String, Vec<Token>> = HashMap::new();

        for entry in entries {
            Self::collect_entry_token(entry, &mut token_candidates)?;
        }

        let mut tokens = Self::filter_struct_enum_tokens(token_candidates);

        for entry in entries {
            Self::collect_entry_function(entry, &mut tokens)?;
        }

        Ok(tokens)
    }

    ///
    fn collect_entry_function(
        entry: &AbiEntry,
        tokens: &mut HashMap<String, Token>,
    ) -> CainomeResult<()> {
        match entry {
            AbiEntry::Function(f) => {
                let mut func = Function::new(&f.name, f.state_mutability.clone().into());

                // For functions, we don't need to deal with generics as we need
                // the flatten type. Parsing the token is enough.
                for i in &f.inputs {
                    func.inputs.push((i.name.clone(), Token::parse(&i.r#type)?));
                }

                for o in &f.outputs {
                    func.outputs.push(Token::parse(&o.r#type)?);
                }

                tokens.insert(f.name.clone(), Token::Function(func));
            }
            AbiEntry::Interface(interface) => {
                for entry in &interface.items {
                    Self::collect_entry_function(entry, tokens)?;
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
