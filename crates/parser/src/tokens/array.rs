//! This module provides a parser for array types.
//!
//! Technically, a `Span` is different than an `Array` in cairo.
//! However, from a binding point of view, they are both collections,
//! and we can safely consider them as the same type.
use super::constants::CAIRO_CORE_SPAN_ARRAY;
use super::genericity;

use crate::tokens::Token;
use crate::{CainomeResult, Error};

pub const CAIRO_0_ARRAY: &str = "*";

#[derive(Debug, Clone, PartialEq)]
pub struct Array {
    pub type_path: String,
    pub inner: Box<Token>,
    pub is_legacy: bool,
}

impl Array {
    pub fn parse(type_path: &str) -> CainomeResult<Self> {
        for a in CAIRO_CORE_SPAN_ARRAY {
            if type_path.starts_with(a) {
                let generic_args = genericity::extract_generics_args(type_path)?;

                if generic_args.len() != 1 {
                    return Err(Error::TokenInitFailed(format!(
                        "Array/Span are expected exactly one generic argument, found {} in `{}`.",
                        generic_args.len(),
                        type_path,
                    )));
                }

                let (_, generic_arg_token) = &generic_args[0];

                return Ok(Self {
                    type_path: type_path.to_string(),
                    inner: Box::new(generic_arg_token.clone()),
                    is_legacy: false,
                });
            }
        }

        if let Some(inner_type) = type_path.strip_suffix(CAIRO_0_ARRAY) {
            return Ok(Self {
                type_path: type_path.to_string(),
                inner: Box::new(Token::parse(inner_type)?),
                is_legacy: true,
            });
        }

        Err(Error::TokenInitFailed(format!(
            "Array/Span couldn't be initialized from `{}`.",
            type_path,
        )))
    }

    pub fn apply_alias(&mut self, type_path: &str, alias: &str) {
        self.inner.apply_alias(type_path, alias);
    }

    pub fn apply_alias_with_file_context(&mut self, type_path: &str, alias: &str, file_name: std::option::Option<&str>) {
        self.inner.apply_alias_with_file_context(type_path, alias, file_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::*;

    #[test]
    fn test_parse() {
        assert_eq!(
            Array::parse("core::array::Array::<core::felt252>").unwrap(),
            Array {
                type_path: "core::array::Array::<core::felt252>".to_string(),
                inner: Box::new(Token::CoreBasic(CoreBasic {
                    type_path: "core::felt252".to_string()
                })),
                is_legacy: false,
            }
        );
    }

    #[test]
    fn test_parse_no_inner_invalid() {
        assert!(Array::parse("core::array::Array").is_err());
        assert!(Array::parse("core::array::Array<>").is_err());
    }

    #[test]
    fn test_parse_wrong_path_invalid() {
        assert!(Array::parse("array::Array::<core::felt252>").is_err());
    }

    #[test]
    fn test_parse_invalid_path_invalid() {
        assert!(Array::parse("module::module2::array::Array::<core::felt252>").is_err());
        assert!(Array::parse("module::module2::MyStruct::<core::felt252>").is_err());
    }

    #[test]
    fn test_apply_alias_with_file_context() {
        // Create an array with a composite inner type
        let inner_composite = Token::Composite(Composite {
            type_path: "contracts::Token".to_string(),
            inners: vec![],
            generic_args: vec![],
            r#type: CompositeType::Struct,
            is_event: false,
            alias: None,
        });

        let mut array = Array {
            type_path: "core::array::Array::<contracts::Token>".to_string(),
            inner: Box::new(inner_composite),
            is_legacy: false,
        };

        // Apply alias with file context - should affect the inner composite
        array.apply_alias_with_file_context("erc20::contracts::Token", "ERC20Token", Some("erc20"));

        // Check that the inner composite got the alias
        if let Token::Composite(ref inner) = *array.inner {
            assert_eq!(inner.alias, Some("ERC20Token".to_string()));
        } else {
            panic!("Expected composite token in array inner");
        }
    }

    #[test]
    fn test_apply_alias_with_file_context_no_match() {
        // Create an array with a composite inner type
        let inner_composite = Token::Composite(Composite {
            type_path: "contracts::Token".to_string(),
            inners: vec![],
            generic_args: vec![],
            r#type: CompositeType::Struct,
            is_event: false,
            alias: None,
        });

        let mut array = Array {
            type_path: "core::array::Array::<contracts::Token>".to_string(),
            inner: Box::new(inner_composite),
            is_legacy: false,
        };

        // Apply alias with non-matching file context
        array.apply_alias_with_file_context("erc721::contracts::Token", "ERC721Token", Some("erc20"));

        // Check that the inner composite did not get the alias
        if let Token::Composite(ref inner) = *array.inner {
            assert_eq!(inner.alias, None);
        } else {
            panic!("Expected composite token in array inner");
        }
    }
}
