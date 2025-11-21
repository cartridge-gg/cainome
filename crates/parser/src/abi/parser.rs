use starknet::core::types::contract::{AbiEntry, AbiEvent, SierraClass, TypedAbiEvent};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use crate::abi::abi_extensions::TokenConvertible;
use crate::abi::registry::TypeRegistry;
use crate::tokens::{constants, CoreBasic, Token};
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

    pub fn has_unknown_dependencies(
        entry: &AbiEntry,
        registry: &TypeRegistry,
    ) -> CainomeResult<bool> {
        match entry {
            // move to abi extensions
            AbiEntry::Function(abi_function) => {
                for item in abi_function.inputs.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(true);
                    }
                }

                for item in abi_function.outputs.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }
            AbiEntry::Event(abi_event) => match abi_event {
                AbiEvent::Typed(typed_abi_event) => match typed_abi_event {
                    TypedAbiEvent::Struct(abi_event_struct) => {
                        for item in abi_event_struct.members.iter() {
                            if !registry.is_known_type(&item.r#type)? {
                                return Ok(true);
                            }
                        }

                        Ok(false)
                    }
                    TypedAbiEvent::Enum(abi_event_enum) => {
                        for item in abi_event_enum.variants.iter() {
                            if !registry.is_known_type(&item.r#type)? {
                                return Ok(true);
                            }
                        }
                        Ok(false)
                    }
                },
                AbiEvent::Untyped(event) => {
                    for item in event.inputs.iter() {
                        if !registry.is_known_type(&item.r#type)? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
            },

            AbiEntry::Struct(abi_struct) => {
                for item in abi_struct.members.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }

            AbiEntry::Enum(abi_enum) => {
                for item in abi_enum.variants.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }

            AbiEntry::Constructor(abi_constructor) => {
                for item in abi_constructor.inputs.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }

            AbiEntry::Interface(abi_interface) => {
                for item in abi_interface.items.iter() {
                    if Self::has_unknown_dependencies(item, registry)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }

            AbiEntry::L1Handler(abi_function) => {
                for item in abi_function.inputs.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(true);
                    }
                }

                for item in abi_function.outputs.iter() {
                    if !registry.is_known_type(&item.r#type)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }
            AbiEntry::Impl(abi_impl) => Ok(!registry.is_known_type(&abi_impl.interface_name)?),
        }
    }

    pub fn build_registry(entries: Vec<AbiEntry>) -> CainomeResult<TypeRegistry> {
        let mut registry = TypeRegistry::new();

        let mut local_entries = VecDeque::from(entries.clone());
        let mut seen_since_last_removal = 0;

        while local_entries.len() > 0 {
            if seen_since_last_removal > local_entries.len() {
                // TODO: sort out
                return Err(Error::ParsingFailed(
                    "Can't resolve ABI types. Some type might be missing.".to_string(),
                ));
            }

            let entry = local_entries.pop_front().expect("Should always succeed");
            seen_since_last_removal += 1;

            if Self::has_unknown_dependencies(&entry, &registry)? {
                local_entries.push_back(entry);
            } else {
                let token = entry.to_token(&mut registry)?;
                // TODO: registry can get type path itself
                registry.set(token.type_path(), token);
                seen_since_last_removal = 0;
            }
        }
        Ok(registry)
    }

    /// Parse all tokens in the ABI.
    pub fn collect_tokens(entries: Vec<AbiEntry>) -> CainomeResult<TokenizedAbi> {
        let registry = Self::build_registry(entries)?;

        let tokens = registry.values();

        let mut structs = vec![];
        let mut enums = vec![];
        let mut functions = vec![];
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
                            unreachable!("According to ABI only function can be there")
                        };

                        new_function_tokens.push(Token::Function(function.clone()));
                    }

                    interfaces.insert(interface.type_path.clone(), new_function_tokens);
                }
                Token::Enum(enumeration) => enums.push(Token::Enum(enumeration.clone())),
                Token::Struct(structure) => structs.push(Token::Struct(structure.clone())),
                _ => (),
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

    use super::*;
    use crate::tokens::CompositeType;

    #[test]
    fn test_parse_abi_struct() {
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
        assert_eq!(result.functions.len(), 0);
        assert_eq!(result.enums.len(), 0);

        // let s = result.structs[1].to_composite().unwrap();
        // assert_eq!(s.type_path, "package::StructOne");
        // assert_eq!(s.r#type, CompositeType::Struct);
        // assert_eq!(s.inners.len(), 3);
        // assert_eq!(s.inners[0].name, "a");
        // assert_eq!(s.inners[1].name, "b");
        // assert_eq!(s.inners[2].name, "c");
    }

    #[test]
    fn test_dojo_starter_direction_available_abi() {
        let abi = AbiParser::tokens_from_abi_string(
            include_str!("../../test_data/dojo_starter-directions_available.abi.json"),
            &HashMap::new(),
        )
        .unwrap();

        assert_eq!(abi.structs.len(), 1);
        // let s = abi.structs[0].to_composite().unwrap();
        // if let Token::Array(a) = &s.inners[1].token.as_ref() {
        //     let inner_array = a.inner.to_composite().unwrap();
        //     assert_eq!(5, inner_array.inners.len());
        //     // Check that copy was properly done
        //     let src_enum = abi.enums[0].to_composite().unwrap();
        //     assert_eq!(inner_array, src_enum);
        // } else {
        //     panic!("Expected array");
        // }
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
        // if let Token::Array(a) = &s.inners[1].token.as_ref() {
        //     if let Token::Tuple(t) = a.inner.as_ref() {
        //         let inner_array = t.inners[0].to_composite().unwrap();
        //         assert_eq!(5, inner_array.inners.len());
        //         // Check that copy was properly done
        //         let src_enum = abi.enums[0].to_composite().unwrap();
        //         assert_eq!(inner_array, src_enum);
        //     } else {
        //         panic!("Expected tuple");
        //     }
        // } else {
        //     panic!("Expected array");
        // }
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
