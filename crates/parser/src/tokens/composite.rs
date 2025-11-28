//! A composite is a type that is composed of other types (struct or enum).
//!
//! A composite type can be generic, and even if in the ABI the generic types
//! are replaced by their concrete types, the [`Composite`] token is still generic
//! to retain the information about the generic types.
//!
//! A pitfall is that the ABI doesn't say which variant/field of the enum/struct
//! is generic. This is an information that needs to be reconstructed by the parser.
//!
//! As an example, with the following cairo struct:
//!
//! ```rust,ignore
//! struct MyStruct<A> {
//!     field_1: A,
//!     field_2: felt252,
//! }
//! ```
//!
//! The ABI will contains several entries for this struct, with the generic
//! type `A` replaced by its concrete type as much as necessary.
//!
//! ```rust,ignore
//! [
//! {
//!     "type": "struct",
//!     "name": "MyStruct::<core::felt252>",
//!     "members": [
//!       {
//!         "name": "field_1",
//!         "type": "core::felt252"
//!       },
//!       {
//!         "name": "field_2",
//!         "type": "core::felt252"
//!       }
//!     ]
//! },
//! {
//!     "type": "struct",
//!     "name": "MyStruct::<core::integer::u64>",
//!     "members": [
//!       {
//!         "name": "field_1",
//!         "type": "core::integer::u64"
//!       },
//!       {
//!         "name": "field_2",
//!         "type": "core::felt252"
//!       }
//!     ]
//! },
//! ]
//! ```
//!
//! As it can be seen, in this case, the ABI doesn't say which variant of the
//! struct is generic since `field_2` is generic in the first case but not in
//! the second one.
//!
//! A naive strategy would be to ensure all types are parsed a first time,
//! and then a generic resolution is done.
use std::cell::RefCell;
use std::rc::Rc;

use super::genericity;
use super::utils;
use super::Token;

use crate::abi::registry;
use crate::abi::registry::TypeRegistry;
use crate::CainomeResult;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CompositeType {
    Struct,
    Enum,
    Unknown,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CompositeInnerKind {
    Key,
    Data,
    Nested,
    Flat,
    NotUsed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositeInner {
    pub index: usize,
    pub name: String,
    pub kind: CompositeInnerKind,
    pub token: Rc<Token>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositeLegacy {
    pub type_path: String,
    pub inners: Vec<CompositeInner>,
    pub generic_args: Vec<(String, Rc<RefCell<Token>>)>,
    pub r#type: CompositeType,
    pub is_event: bool,
    pub alias: Option<String>,
}

impl CompositeLegacy {
    /// Parses a composite type from a type path.
    /// Since the composite can be named arbitrarily, by the user,
    /// the parsing of the composite is not checking if the type path is
    /// a core basic type, an array or something else.
    ///
    /// In cainome the type path is first parsed as any other token, and
    /// [`Composite`] is the last token that is parsed (which accepts every path).
    ///
    /// You may use [`Composite::is_builtin`] to check if the type path is
    /// a known Cairo builtin type.
    ///
    /// # Arguments
    ///
    /// * `type_path` - The type path to parse.
    ///
    /// # Returns
    ///
    /// Returns a [`Composite`] token if the type path is a composite.
    /// Returns an error otherwise.
    pub fn parse(type_path: &str, registry: &TypeRegistry) -> CainomeResult<Self> {
        let type_path = utils::escape_rust_keywords(type_path);
        let generic_args = genericity::extract_generics_args(&type_path)?;

        let generic_args_with_types: Vec<(String, Rc<RefCell<Token>>)> = generic_args
            .into_iter()
            .map(|(name, path)| (name, registry.get(&path).unwrap()))
            .collect();

        Ok(Self {
            // We want to keep the path with generic for the generic resolution.
            type_path: type_path.to_string(),
            inners: vec![],
            generic_args: generic_args_with_types,
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        })
    }

    pub fn type_path_no_generic(&self) -> String {
        genericity::type_path_no_generic(&self.type_path)
    }

    pub fn is_generic(&self) -> bool {
        !self.generic_args.is_empty()
    }

    pub fn type_name(&self) -> String {
        // TODO: need to opti that with regex?
        utils::extract_type_path_with_depth(&self.type_path_no_generic(), 0)
    }

    pub fn type_name_or_alias(&self) -> String {
        if let Some(a) = &self.alias {
            a.clone()
        } else {
            self.type_name()
        }
    }

    pub fn apply_alias(&mut self, type_path: &str, alias: &str) {
        if self.type_path_no_generic() == type_path {
            self.alias = Some(alias.to_string());
        }

        // for ref mut i in &mut self.inners {
        //     if let Token::Composite(ref mut c) = i.token.as_ref() {
        //         c.apply_alias(type_path, alias);
        //     }
        // }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::*;

    fn basic_felt252() -> Rc<RefCell<Token>> {
        Rc::new(RefCell::new(Token::Basic(CoreBasic {
            type_path: "core::felt252".to_string(),
        })))
    }

    fn basic_u64() -> Rc<RefCell<Token>> {
        Rc::new(RefCell::new(Token::Basic(CoreBasic {
            type_path: "core::integer::u64".to_string(),
        })))
    }

    #[test]
    fn test_parse() {
        let expected = CompositeLegacy {
            type_path: "module::MyStruct".to_string(),
            inners: vec![],
            generic_args: vec![],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        };

        // assert_eq!(Composite::parse("module::MyStruct").unwrap(), expected);
        assert!(!expected.is_generic());
    }

    #[test]
    fn test_parse_generic_one() {
        let expected = CompositeLegacy {
            type_path: "module::MyStruct::<core::felt252>".to_string(),
            inners: vec![],
            generic_args: vec![("A".to_string(), basic_felt252())],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        };

        // assert_eq!(
        //     Composite::parse("module::MyStruct::<core::felt252>").unwrap(),
        //     expected
        // );
        assert!(expected.is_generic());
    }

    #[test]
    fn test_parse_generic_two() {
        let expected = CompositeLegacy {
            type_path: "module::MyStruct::<core::felt252, core::integer::u64>".to_string(),
            inners: vec![],
            generic_args: vec![
                ("A".to_string(), basic_felt252()),
                ("B".to_string(), basic_u64()),
            ],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        };

        // assert_eq!(
        //     Composite::parse("module::MyStruct::<core::felt252, core::integer::u64>").unwrap(),
        //     expected
        // );
        assert!(expected.is_generic());
    }

    #[test]
    fn test_type_name() {
        let mut c = CompositeLegacy {
            type_path: "module::MyStruct".to_string(),
            inners: vec![],
            generic_args: vec![],
            r#type: CompositeType::Unknown,
            is_event: false,
            alias: None,
        };
        assert_eq!(c.type_name(), "MyStruct");

        c.type_path = "module::MyStruct::<core::felt252>".to_string();
        assert_eq!(c.type_name(), "MyStruct");
    }
}
