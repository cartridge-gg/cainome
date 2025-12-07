use starknet::core::types::contract::{AbiEntry, AbiEvent, SierraClass, TypedAbiEvent};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use crate::abi::extensions::{Named, TryTokenConvertable};
use crate::abi::registry::TypeRegistry;
use crate::tokens::Token;
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
                // TODO: this branch is most likely unreachable. Think on it.
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
            if let Ok(token) = registry.get(&entry.get_name()) {
                if token.borrow().is_basic() {
                    // If entry is of basic builtin type
                    // We just ignore that entry and drop it.
                    seen_since_last_removal = 0;
                    continue;
                }
            } else {
                // To support for indirect recursive reference resolution, we fill seen
                // types with Placeholder to later replace with resolved type.
                registry.set(&entry.get_name(), Token::Placeholder);
            }

            // Checking if registry has all the nested types to resolve
            // and contruct token.
            if let Some(unknown_field) = Self::has_unknown_dependencies(&entry, &registry)? {
                // We can't resolve that now, let's put to the end of the queue
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
