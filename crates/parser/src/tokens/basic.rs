//! Core basic types are built-in types that are available in the core library
//! and which are not a struct nor an enum, nor an array.
//!
//! This module provides a parser for core basic types.

#[derive(Debug, Clone, PartialEq)]
pub struct TypePath {
    pub type_path: String,
}

impl TypePath {
    pub fn new(type_path: &str) -> Self {
        return Self {
            type_path: type_path.to_string(),
        };
    }

    pub fn type_path(&self) -> &str {
        &self.type_path
    }
}
