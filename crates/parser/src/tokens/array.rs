//! This module provides a parser for array types.
//!
//! Technically, a `Span` is different than an `Array` in cairo.
//! However, from a binding point of view, they are both collections,
//! and we can safely consider them as the same type.
use std::cell::RefCell;
use std::rc::Rc;

use super::constants::CAIRO_CORE_SPAN_ARRAY;
use super::genericity;

use crate::tokens::{Container, Token};
use crate::{CainomeResult, Error};

pub const CAIRO_0_ARRAY: &str = "*";

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayContainer {
    pub type_path: String,
    pub inner: Rc<RefCell<Token>>,
    pub is_legacy: bool,
}

impl ArrayContainer {
    pub fn test_path(type_path: &str) -> bool {
        Self::is_span(type_path) || Self::is_cairo0(type_path)
    }

    pub fn is_span(type_path: &str) -> bool {
        CAIRO_CORE_SPAN_ARRAY
            .iter()
            .any(|prefix| type_path.starts_with(prefix))
    }

    pub fn is_cairo0(type_path: &str) -> bool {
        type_path.strip_suffix(CAIRO_0_ARRAY).is_some()
    }

    pub fn get_inner(type_path: &str) -> CainomeResult<String, Error> {
        if Self::is_cairo0(type_path) {
            return Ok(type_path.strip_suffix(CAIRO_0_ARRAY).unwrap().to_string());
        }

        let type_path = type_path.trim_start_matches("@");

        let generic_args = genericity::extract_generics_args(type_path)?;

        if generic_args.len() != 1 {
            return Err(Error::TokenInitFailed(format!(
                "Array/Span are expected exactly one generic argument, found {} in `{}`.",
                generic_args.len(),
                type_path,
            )));
        }

        let generic_arg_token = generic_args.into_iter().next().map(|(_, token)| token);

        Ok(generic_arg_token.unwrap())
    }

    pub fn new(type_path: &str, inner: &Rc<RefCell<Token>>) -> Self {
        return Self {
            type_path: type_path.to_string(),
            inner: Rc::clone(inner),
            is_legacy: false,
        };
    }

    pub fn new_token(type_path: &str, inner: &Rc<RefCell<Token>>) -> Token {
        Token::Container(Container::Array(Self::new(type_path, inner)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::*;

    #[test]
    fn test_parse() {
        // assert_eq!(
        //     Array::parse("core::array::Array::<core::felt252>").unwrap(),
        //     Array {
        //         type_path: "core::array::Array::<core::felt252>".to_string(),
        //         inner: Rc::new(Token::CoreBasic(CoreBasic {
        //             type_path: "core::felt252".to_string()
        //         })),
        //         is_legacy: false,
        //     }
        // );
    }

    #[test]
    fn test_parse_no_inner_invalid() {
        // assert!(Array::parse("core::array::Array").is_err());
        // assert!(Array::parse("core::array::Array<>").is_err());
    }

    #[test]
    fn test_parse_wrong_path_invalid() {
        // assert!(Array::parse("array::Array::<core::felt252>").is_err());
    }

    #[test]
    fn test_parse_invalid_path_invalid() {
        // assert!(Array::parse("module::module2::array::Array::<core::felt252>").is_err());
        // assert!(Array::parse("module::module2::MyStruct::<core::felt252>").is_err());
    }
}
