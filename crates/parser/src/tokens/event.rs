use std::rc::Rc;

use super::constants::{CAIRO_COMPOSITE_BUILTINS, CAIRO_GENERIC_BUILTINS};
use super::genericity;
use super::Token;

use crate::abi::registry::TypeRegistry;
use crate::tokens::utils;
use crate::CainomeResult;

#[derive(Debug, Clone, PartialEq)]
pub struct EventInner {
    pub name: String,
    pub token: Rc<Token>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub type_path: String,
    pub keys: Vec<EventInner>,
    pub data: Vec<EventInner>,
    pub nested: Vec<EventInner>,
    pub flat: Vec<EventInner>,
    pub generic_args: Vec<(String, Rc<Token>)>,
    pub alias: Option<String>,
}

impl Event {
    pub fn new(type_path: String, registry: &TypeRegistry) -> CainomeResult<Self> {
        let type_path = utils::escape_rust_keywords(&type_path);
        let generic_args = genericity::extract_generics_args(&type_path)?;

        let generic_args_with_types: Vec<(String, Rc<Token>)> = generic_args
            .into_iter()
            .map(|(name, path)| (name, registry.get(&path).unwrap()))
            .collect();

        return Ok(Self {
            type_path,
            generic_args: generic_args_with_types,
            alias: None,
            keys: vec![],
            data: vec![],
            nested: vec![],
            flat: vec![],
        });
    }

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
    pub fn parse(type_path: &str) -> CainomeResult<Self> {
        let type_path = utils::escape_rust_keywords(type_path);
        let generic_args = genericity::extract_generics_args(&type_path)?;

        // We want to keep the path with generic for the generic resolution.
        // Ok(Self::new(type_path.to_string(), generic_args))
        Err(crate::Error::ParsingFailed("asd".to_string()))
    }

    pub fn type_path_no_generic(&self) -> String {
        genericity::type_path_no_generic(&self.type_path)
    }

    pub fn is_generic(&self) -> bool {
        !self.generic_args.is_empty()
    }

    /// Returns true if the current composite is considered as Cairo builtin.
    /// This is useful to avoid expanding the structure if already managed by
    /// the backend (like Option and Result for instance).
    /// Spans and Arrays are handled by `array`.
    pub fn is_builtin(&self) -> bool {
        for b in CAIRO_GENERIC_BUILTINS {
            if self.type_path.starts_with(b) {
                return true;
            }
        }

        for b in CAIRO_COMPOSITE_BUILTINS {
            if self.type_path.starts_with(b) {
                return true;
            }
        }

        false
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

        // for ref mut i in &mut self.nested {
        //     if let Token::Composite(ref mut c) = i.token.as_ref() {
        //         c.apply_alias(type_path, alias);
        //     }
        // }

        // for ref mut i in &mut self.flat {
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

    fn basic_felt252() -> Token {
        Token::CoreBasic(CoreBasic {
            type_path: "core::felt252".to_string(),
        })
    }

    fn basic_u64() -> Token {
        Token::CoreBasic(CoreBasic {
            type_path: "core::integer::u64".to_string(),
        })
    }

    #[test]
    fn test_parse() {
        // let expected = Event::new("module::MyStruct".to_string()).unwrap();

        // assert_eq!(Event::parse("module::MyStruct").unwrap(), expected);
        // assert!(!expected.is_generic());
    }

    #[test]
    fn test_parse_generic_one() {
        // let expected = Event::new("module::MyStruct::<core::felt252>".to_string()).unwrap();

        // assert_eq!(
        //     Event::parse("module::MyStruct::<core::felt252>").unwrap(),
        //     expected
        // );
        // assert!(expected.is_generic());
    }

    #[test]
    fn test_parse_generic_two() {
        // let expected =
        //     Event::new("module::MyStruct::<core::felt252, core::integer::u64>".to_string())
        //         .unwrap();

        // assert_eq!(
        //     Event::parse("module::MyStruct::<core::felt252, core::integer::u64>").unwrap(),
        //     expected
        // );
        // assert!(expected.is_generic());
    }

    #[test]
    fn test_type_name() {
        // let mut c = Event::new("module::MyStruct".to_string()).unwrap();
        // assert_eq!(c.type_name(), "MyStruct");

        // c.type_path = "module::MyStruct::<core::felt252>".to_string();
        // assert_eq!(c.type_name(), "MyStruct");
    }
}
