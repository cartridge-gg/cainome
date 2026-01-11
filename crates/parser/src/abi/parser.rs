use starknet::core::types::contract::{AbiEntry, SierraClass};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use crate::abi::extensions::TryTokenConvertable;
use crate::abi::parser_context::ParserContext;
use crate::abi::registry::TypeRegistry;
use crate::tokens::{Token, TypePath};
use crate::{CainomeResult, Error};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TokenizedAbi {
    /// All enums found in the contract ABI.
    pub enums: Vec<Rc<RefCell<Token>>>,
    /// All structs found in the contract ABI.
    pub structs: Vec<Rc<RefCell<Token>>>,
    /// Standalone functions in the contract ABI.
    pub functions: Vec<Rc<RefCell<Token>>>,
    /// Events.
    pub events: Vec<Rc<RefCell<Token>>>,
    /// Fully qualified interface name mapped to all the defined functions in it.
    pub interfaces: HashMap<String, Vec<Rc<RefCell<Token>>>>,
    pub interfaces_new: Vec<Rc<RefCell<Token>>>,
}

pub struct AbiParser {}

pub trait WithDependencies {
    fn get_dependencies(&self) -> Vec<String>;
}

pub trait Named {
    fn get_name(&self) -> String;
}

pub trait Parseable: TryTokenConvertable + Named + WithDependencies + Clone {}

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
        type_aliases: HashMap<&str, &str>,
    ) -> CainomeResult<TokenizedAbi> {
        let ctx = ParserContext::new().with_substitutions(type_aliases);

        let abi_entries = Self::parse_abi_string(abi)?;
        let tokenized_abi =
            AbiParser::collect_tokens(abi_entries, ctx).expect("failed tokens parsing");

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

    fn has_unknown_dependencies<T>(
        entry: &T,
        registry: &TypeRegistry,
        ctx: &ParserContext,
    ) -> CainomeResult<Option<String>>
    where
        T: WithDependencies,
    {
        for dep in entry.get_dependencies().iter() {
            // Skipped types are always unknown
            if ctx.is_type_skipped(dep) {
                return Ok(Some(dep.to_string()));
            }

            if !registry.is_known_type(dep)? {
                return Ok(Some(dep.to_string()));
            }
        }

        Ok(None)
    }

    pub fn build_registry<T>(entries: Vec<T>, ctx: ParserContext) -> CainomeResult<TypeRegistry>
    where
        T: Parseable,
    {
        let mut registry = TypeRegistry::new();

        let mut local_entries = VecDeque::from(entries.clone());
        let mut seen_since_last_removal = 0;
        let mut unknown_fields: HashSet<String> = HashSet::new();

        registry.apply_substitutions(&ctx.substitutions);
        // TODO: probably need to remove skipped types from registry right away

        // We will be converting AbiEntries to tokens and drop them upon converting
        while local_entries.len() > 0 {
            // This branch means that we went through all the AbiEntry and could not
            // convert any. This means Abi is incorrect (well, we might have a bug though)
            if seen_since_last_removal > local_entries.len() {
                // TODO: this branch is most likely unreachable. Think on it.
                // TODO: sort out error types
                return Err(Error::ParsingFailed(format!(
                    "Can't resolve ABI types. Some type might be missing. Check: [{}]",
                    unknown_fields.into_iter().collect::<Vec<_>>().join(", ")
                )));
            }

            let entry = local_entries.pop_front().expect("Should always succeed");
            seen_since_last_removal += 1;

            let type_path = entry.get_name();

            // Workaround to skip parsing Composite CoreBasics (like core::boolean).
            // As get also strips all the containers, those will be dropped at this point.
            // NOTE: this might move into extensions for AbiEntry
            if let Ok(token) = registry.get(&type_path) {
                if let Token::Substitute(_) = &*token.borrow() {
                    continue;
                }

                if let Token::Basic(_) = &*token.borrow() {
                    seen_since_last_removal = 0;
                    continue;
                }

                // If entry is skipped. Well, we just ignore it
                if let Token::Skip(_) = &*token.borrow() {
                    continue;
                }
            } else {
                // To support for indirect recursive reference resolution, we fill seen
                // types with Placeholder to later replace with resolved type.
                registry.set(&type_path, Token::Placeholder);
            }

            // Check if type should be skipped
            if ctx.is_type_skipped(&type_path) {
                registry.set(&type_path, Token::Skip(TypePath::new(&type_path)));
                continue;
            }

            // Checking if registry has all the nested types to resolve
            // and contruct token.
            if let Some(unknown_field) = Self::has_unknown_dependencies(&entry, &registry, &ctx)? {
                // We can't resolve that now, let's put to the end of the queue
                tracing::info!(
                    "Deferring type: {}. Field {} is unknown",
                    type_path,
                    unknown_field
                );

                local_entries.push_back(entry);
                unknown_fields.insert(unknown_field);
                continue;
            }

            // Ok, now we can resolve.
            if let Some(token) = entry.try_to_token(&mut registry)? {
                registry.set(&entry.get_name(), token);
            } else {
                // This means that entry resolved into token absence, let's remove placeholder.
                // (happens for implementation)
                registry.remove(&entry.get_name());
            }
            seen_since_last_removal = 0;
            unknown_fields.clear();
        }

        let uninitialised_placeholders = registry.get_uninitialised_placeholders();

        // Check for unresolved placeholders
        if uninitialised_placeholders.len() > 0 {
            return Err(Error::ParsingFailed(format!(
                "Can't resolve ABI types. Unresolved: [{}]",
                uninitialised_placeholders
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }

        Ok(registry)
    }

    pub fn create_tokenized_abi(tokens: Vec<Rc<RefCell<Token>>>) -> TokenizedAbi {
        let mut structs = vec![];
        let mut enums = vec![];
        let mut events = vec![];
        let mut functions = vec![];
        let mut interfaces_new = vec![];
        let mut interfaces = HashMap::new();

        for token in tokens {
            match &*token.borrow() {
                Token::Function(_) => {
                    functions.push(Rc::clone(&token));
                }
                Token::Interface(interface) => {
                    interfaces_new.push(Rc::clone(&token));

                    // Legacy
                    // TODO: remove
                    let mut new_function_tokens = vec![];

                    for function_token in interface.functions.iter() {
                        let Token::Function(_) = &*function_token.borrow() else {
                            unreachable!("According to ABI specs only function can be there")
                        };

                        new_function_tokens.push(function_token.clone());
                    }
                    interfaces.insert(interface.type_path.clone(), new_function_tokens);
                }
                Token::Event(_) => events.push(Rc::clone(&token)),
                Token::Enum(_) => enums.push(Rc::clone(&token)),
                Token::Struct(_) => structs.push(Rc::clone(&token)),
                _ => (),
            }
        }

        TokenizedAbi {
            enums,
            events,
            structs,
            functions,
            interfaces,
            interfaces_new,
        }
    }

    /// Parse all tokens in the ABI.
    pub fn collect_tokens<T>(entries: Vec<T>, ctx: ParserContext) -> CainomeResult<TokenizedAbi>
    where
        T: Parseable,
    {
        // This procedure will populate all the tokens
        let registry = Self::build_registry(entries, ctx)?;

        // So we'll need to just bucket them in correct fields.
        let tokens = registry.values();

        Ok(Self::create_tokenized_abi(tokens))
    }
}
