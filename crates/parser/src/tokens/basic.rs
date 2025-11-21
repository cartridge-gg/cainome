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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        assert_eq!(
            CoreBasic::new("core::felt252"),
            CoreBasic {
                type_path: "core::felt252".to_string(),
            }
        );

        assert_eq!(
            CoreBasic::new("core::integer::u64"),
            CoreBasic {
                type_path: "core::integer::u64".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_unit() {
        assert_eq!(
            CoreBasic::new("()"),
            CoreBasic {
                type_path: "()".to_string(),
            }
        );
    }

    // #[test]
    // fn test_parse_array_span_invalid() {
    //     assert!(CoreBasic::parse("core::array::Array<core::felt252>").is_err());
    //     assert!(CoreBasic::parse("core::array::Span<core::felt252>").is_err());
    // }

    // #[test]
    // fn test_parse_composite_invalid() {
    //     assert!(CoreBasic::parse("mymodule::MyStruct").is_err());
    //     assert!(CoreBasic::parse("module2::module3::MyStruct<core::felt252>").is_err());
    // }
}
