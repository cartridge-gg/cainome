//! This module provides a parser for array types.
//!
//! Technically, a `Span` is different than an `Array` in cairo.
//! However, from a binding point of view, they are both collections,
//! and we can safely consider them as the same type.
use std::cell::RefCell;
use std::rc::Rc;

use super::constants::CAIRO_CORE_SPAN_ARRAY;
use super::genericity;

use crate::tokens::Token;
use crate::{CainomeResult, Error};

pub const CAIRO_0_ARRAY: &str = "*";

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayContainer {
    pub type_path: String,
    pub inner: Rc<RefCell<Token>>,
}

impl ArrayContainer {
    pub fn test_path(type_path: &str) -> bool {
        Self::is_span(type_path) || Self::is_cairo0(type_path)
    }

    pub fn is_span(type_path: &str) -> bool {
        let type_path = type_path.trim_start_matches("@");

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

        let [(_name, token_path)] = generic_args.as_slice() else {
            return Err(Error::TokenInitFailed(format!(
                "Array/Span are expected exactly one generic argument, found {} in `{}`.",
                generic_args.len(),
                type_path,
            )));
        };

        Ok(token_path.to_owned())
    }

    pub fn new(type_path: &str, inner: &Rc<RefCell<Token>>) -> Self {
        Self {
            type_path: type_path.to_string(),
            inner: Rc::clone(inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use starknet::core::types::contract::{AbiEntry, AbiNamedMember, AbiStruct};

    use super::*;
    use crate::{abi::registry::TypeRegistry, AbiParser, ParserContext};

    #[test]
    fn test_get_inner() {
        let inner_type =
            ArrayContainer::get_inner("core::array::Array::<core::felt252>").expect("Should work");

        assert_eq!(inner_type, "core::felt252")
    }

    fn execute_parsing_for(ttype: String) -> CainomeResult<TypeRegistry> {
        let data = vec![AbiEntry::Struct(AbiStruct {
            name: "test".to_string(),
            members: vec![AbiNamedMember {
                name: "f1".to_string(),
                r#type: ttype,
            }],
        })];

        AbiParser::build_registry(data, ParserContext::default())
    }

    #[test]
    fn test_ok_cases() {
        let cases = vec![
            "core::array::Array<felt>".to_string(),
            "@core::array::Array<core::felt252>".to_string(),
            "core::array::Span<felt>".to_string(),
            "@core::array::Span<core::option::Option<felt>>".to_string(),
        ];

        for case in cases {
            let res = execute_parsing_for(case.clone());

            assert!(res.is_ok())
        }
    }

    #[test]
    fn test_parse_no_inner_invalid1() {
        let res = execute_parsing_for("core::array::Array".to_string());

        let Result::Err(e) = res else {
            panic!("This should fail. Array is incorrect");
        };

        let Error::TokenInitFailed(_) = e else {
            panic!("This be Error::TokenInitFailed error");
        };
    }

    #[test]
    fn test_parse_wrong_path_invalid1() {
        let res = execute_parsing_for("array::Array::<core::felt252>".to_string());

        println!("res: {:?}", res);

        let Result::Err(e) = res else {
            panic!("This should fail. Array is incorrect");
        };

        let Error::ParsingFailed(_) = e else {
            panic!("This be Error::TokenInitFailed error");
        };
    }

    #[test]
    fn test_parse_wrong_path_invalid2() {
        let res = execute_parsing_for("module::module2::array::Array::<core::felt252>".to_string());

        let Result::Err(e) = res else {
            panic!("This should fail. Array is incorrect");
        };

        let Error::ParsingFailed(_) = e else {
            panic!("This be Error::TokenInitFailed error");
        };
    }

    #[test]
    fn test_parse_wrong_path_invalid3() {
        let res = execute_parsing_for("module::module2::MyStruct::<core::felt252>".to_string());

        let Result::Err(e) = res else {
            panic!("This should fail. Array is incorrect");
        };

        let Error::ParsingFailed(_) = e else {
            panic!("This be Error::TokenInitFailed error");
        };
    }
}
