//! This module provides a token type for the `Result` type.
//!
//! <https://github.com/starkware-libs/cairo/blob/main/corelib/src/result.cairo>
use crate::tokens::Token;
use crate::{CainomeResult, Error};

use super::composite::escape_rust_keywords;
use super::genericity;

#[derive(Debug, Clone, PartialEq)]
pub struct Result {
    pub type_path: String,
    pub inner: Box<Token>,
    pub error: Box<Token>,
}

impl Result {
    pub fn parse(type_path: &str) -> CainomeResult<Self> {
        let type_path = escape_rust_keywords(type_path);

        if type_path.starts_with("core::result::Result") {
            let generic_args = genericity::extract_generics_args(&type_path)?;

            if generic_args.len() != 2 {
                return Err(Error::InvalidResultTypePath(type_path.to_string()));
            }

            let (_, generic_arg_token) = &generic_args[0];
            let (_, error_token) = &generic_args[1];

            Ok(Self {
                type_path: type_path.to_string(),
                inner: Box::new(generic_arg_token.clone()),
                error: Box::new(error_token.clone()),
            })
        } else {
            Err(Error::TokenInitFailed(format!(
                "Result couldn't be initialized from `{}`.",
                type_path,
            )))
        }
    }

    pub fn apply_alias(&mut self, type_path: &str, alias: &str) {
        self.inner.apply_alias(type_path, alias);
        self.error.apply_alias(type_path, alias);
    }
}
