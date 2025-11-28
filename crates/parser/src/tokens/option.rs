//! This module provides a token type for the `Option` type.
//!
//! <https://github.com/starkware-libs/cairo/blob/main/corelib/src/option.cairo>
use std::cell::RefCell;
use std::rc::Rc;

use crate::tokens::{Container, Token};
use crate::{CainomeResult, Error};

use super::genericity;

#[derive(Debug, Clone, PartialEq)]
pub struct OptionContainer {
    pub type_path: String,
    pub inner: Rc<RefCell<Token>>,
}

impl OptionContainer {
    pub fn test_path(type_path: &str) -> bool {
        type_path.starts_with("core::option::Option")
    }

    pub fn new(type_path: &str, inner: &Rc<RefCell<Token>>) -> Self {
        Self {
            type_path: type_path.to_string(),
            inner: Rc::clone(inner),
        }
    }

    pub fn new_token(type_path: &str, inner: &Rc<RefCell<Token>>) -> Token {
        Token::Option(Self::new(type_path, inner))
    }

    pub fn get_inner(type_path: &str) -> CainomeResult<String, Error> {
        let generic_args = genericity::extract_generics_args(&type_path)?;

        if generic_args.len() != 1 {
            return Err(Error::InvalidOptionTypePath(type_path.to_string()));
        }

        let generic_arg_token = generic_args.into_iter().next().map(|(_, token)| token);

        Ok(generic_arg_token.unwrap())
    }
}
