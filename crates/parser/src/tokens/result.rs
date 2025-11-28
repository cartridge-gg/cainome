//! This module provides a token type for the `Result` type.
//!
//! <https://github.com/starkware-libs/cairo/blob/main/corelib/src/result.cairo>
use std::cell::RefCell;
use std::rc::Rc;

use crate::tokens::{Container, Token};
use crate::{CainomeResult, Error};

use super::genericity;

#[derive(Debug, Clone, PartialEq)]
pub struct ResultContainer {
    pub type_path: String,
    pub inner: Rc<RefCell<Token>>,
    pub error: Rc<RefCell<Token>>,
}

pub struct ResultContainerInnerTypes {
    pub inner: String,
    pub error: String,
}

impl ResultContainer {
    pub fn test_path(type_path: &str) -> bool {
        type_path.starts_with("core::result::Result")
    }

    pub fn get_inner(type_path: &str) -> CainomeResult<ResultContainerInnerTypes, Error> {
        let generic_args = genericity::extract_generics_args(&type_path)?;

        if generic_args.len() != 2 {
            return Err(Error::InvalidOptionTypePath(type_path.to_string()));
        }

        Ok(ResultContainerInnerTypes {
            inner: generic_args[0].1.clone(),
            error: generic_args[1].1.clone(),
        })
    }

    pub fn new(type_path: &str, inner: &Rc<RefCell<Token>>, error: &Rc<RefCell<Token>>) -> Self {
        Self {
            type_path: type_path.to_string(),
            inner: Rc::clone(inner),
            error: Rc::clone(error),
        }
    }

    pub fn new_token(
        type_path: &str,
        inner: &Rc<RefCell<Token>>,
        error: &Rc<RefCell<Token>>,
    ) -> Token {
        Token::Result(Self::new(type_path, inner, error))
    }
}
