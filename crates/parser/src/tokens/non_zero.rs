//! This module provides a token type for the `NonZero` type.
//!
//! <https://github.com/starkware-libs/cairo/blob/main/corelib/src/zeroable.cairo#L80>
use std::rc::Rc;

use crate::abi::registry::TypeRegistry;
use crate::tokens::Token;
use crate::{CainomeResult, Error};

use super::genericity;
use super::utils;

#[derive(Debug, Clone, PartialEq)]
pub struct NonZeroContainer {
    pub type_path: String,
    pub inner: Rc<Token>,
}

impl NonZeroContainer {
    pub fn test_path(type_path: &str) -> bool {
        type_path.starts_with("core::zeroable::NonZero")
    }

    pub fn new(type_path: &str, inner: &Rc<Token>) -> Self {
        Self {
            type_path: type_path.to_string(),
            inner: Rc::clone(inner),
        }
    }

    pub fn get_inner(type_path: &str) -> CainomeResult<String, Error> {
        let generic_args = genericity::extract_generics_args(&type_path)?;

        if generic_args.len() != 1 {
            return Err(Error::InvalidNonZeroTypePath(type_path.to_string()));
        }

        let generic_arg_token = generic_args.into_iter().next().map(|(_, token)| token);

        Ok(generic_arg_token.unwrap())
    }
}
