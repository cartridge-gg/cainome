//! A TupleContainer is a container for tuple type grouping other tokens
//!
//! An empty tuple is considered as a unit type `()`, and has its own management
//! in the [`crate::tokens::basic::TypePath`] module.
//!
//! A tuple can contain generic in cairo code, however in the ABI,
//! generic types are actually always replaced by their concrete types.
//! So a [`TupleContainer`] is not a generic type itself in the context of cainome.
use std::{cell::RefCell, rc::Rc};

use syn::Type;

use super::Token;
use crate::{CainomeResult, Error};

#[derive(Debug, Clone, PartialEq)]
pub struct TupleContainer {
    pub type_path: String,
    pub inners: Vec<Rc<RefCell<Token>>>,
}

impl TupleContainer {
    pub fn test_path(type_path: &str) -> bool {
        // regex
        type_path.trim().starts_with("(")
            && type_path.trim().ends_with(")")
            && type_path.trim().ne("()")
    }

    pub fn get_inner(type_path: &str) -> CainomeResult<Vec<String>> {
        let t: Type = syn::parse_str(type_path)?;
        let mut inners = vec![];

        match t {
            Type::Tuple(t) => {
                for e in t.elems {
                    inners.push(quote::quote!(#e).to_string());
                }
            }
            Type::Paren(t) => {
                // Tuple with one element are under `Paren` variant.
                let e = t.elem;
                inners.push(quote::quote!(#e).to_string());
            }
            _ => {
                return Err(Error::TokenInitFailed(format!(
                    "Tuple couldn't be initialized from `{type_path}`.",
                )));
            }
        }

        Ok(inners)
    }

    /// Parses a tuple from a type path.
    ///
    /// # Arguments
    ///
    /// * `type_path` - The full type path from ABI to parse. (example: `core::integer::u64`)
    /// * `inners` - The inner tokens of the tuple.
    /// # Returns
    ///
    /// Returns a [`TupleContainer`] token if the type path is a tuple.
    /// Returns an error otherwise.
    pub fn new(type_path: &str, inners: Vec<Rc<RefCell<Token>>>) -> Self {
        Self {
            type_path: type_path.to_string(),
            inners,
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_parse_unit_invalid() {
        // assert!(Tuple::parse("()").is_err());
    }

    #[test]
    fn test_parse_one_inner() {
        // assert_eq!(
        // Tuple::parse("(core::felt252)").unwrap(),
        // Tuple {
        //     type_path: "(core::felt252)".to_string(),
        //     inners: vec![Rc::new(Token::CoreBasic(CoreBasic {
        //         type_path: "core::felt252".to_string()
        //     }))],
        // }
        // );
    }

    #[test]
    fn test_parse_multiple_inners() {
        // assert_eq!(
        //     Tuple::parse("(core::felt252, core::integer::u64)").unwrap(),
        //     Tuple {
        //         type_path: "(core::felt252, core::integer::u64)".to_string(),
        //         inners: vec![
        //             Rc::new(Token::CoreBasic(CoreBasic {
        //                 type_path: "core::felt252".to_string()
        //             })),
        //             Rc::new(Token::CoreBasic(CoreBasic {
        //                 type_path: "core::integer::u64".to_string()
        //             })),
        //         ],
        //     }
        // );
    }

    #[test]
    fn test_parse_other_type_invalid() {
        // assert!(Tuple::parse("module::module2::MyStuct").is_err());
        // assert!(Tuple::parse("core::integer::u64").is_err());
    }
}
