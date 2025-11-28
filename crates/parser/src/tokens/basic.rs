//! Core basic types are built-in types that are available in the core library
//! and which are not a struct nor an enum, nor an array.
//!
//! This module provides a parser for core basic types.
use super::constants::{CAIRO_CORE_BASIC, UNIT_TYPE};
use crate::{CainomeResult, Error};

#[derive(Debug, Clone, PartialEq)]
pub struct CoreBasic {
    pub type_path: String,
}

impl CoreBasic {
    pub fn new(type_path: &str) -> Self {
        return Self {
            type_path: type_path.to_string(),
        };
    }

    /// Returns the name of the core basic type.
    ///
    /// The type name is the last part of the type path, to remove any
    /// module name.
    /// The type name is also removing any generic parameters, if any.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cainome_parser::tokens::CoreBasic;
    ///
    /// let core_basic = CoreBasic::parse("core::felt252").unwrap();
    /// assert_eq!(core_basic.type_name(), "felt252");
    /// ```
    ///
    /// # Returns
    ///
    /// Returns the name of the core basic type.
    pub fn type_name(&self) -> String {
        let f = self
            .type_path
            .split('<')
            .nth(0)
            .unwrap_or(&self.type_path)
            .trim_end_matches("::")
            .to_string();

        f.split("::").last().unwrap_or(&f).to_string()
    }
}
