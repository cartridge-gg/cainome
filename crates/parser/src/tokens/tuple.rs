//! A tuple is a collection of types which can be of different types.
//!
//! An empty tuple is considered as a unit type `()`, and has its own management
//! in the [`crate::tokens::CoreBasic`] module.
//!
//! A tuple can contain generic in cairo code, however in the ABI,
//! generic types are actually always replaced by their concrete types.
//! So a [`Tuple`] is not a generic type itself in the context of cainome.
use syn::Type;

use super::Token;
use crate::{CainomeResult, Error};

#[derive(Debug, Clone, PartialEq)]
pub struct Tuple {
    pub type_path: String,
    pub inners: Vec<Token>,
}

impl Tuple {
    /// Parses a tuple from a type path.
    ///
    /// # Arguments
    ///
    /// * `type_path` - The type path to parse.
    ///
    /// # Returns
    ///
    /// Returns a [`Tuple`] token if the type path is a tuple.
    /// Returns an error otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cainome_parser::tokens::{Token, Tuple, CoreBasic};
    ///
    /// let tuple = Tuple::parse("(core::felt252, core::integer::u64)").unwrap();
    /// assert_eq!(tuple.type_path, "(core::felt252, core::integer::u64)");
    /// assert_eq!(tuple.inners.len(), 2);
    /// assert_eq!(tuple.inners[0], Token::CoreBasic(CoreBasic { type_path: "core::felt252".to_string() }));
    /// assert_eq!(tuple.inners[1], Token::CoreBasic(CoreBasic { type_path: "core::integer::u64".to_string() }));
    /// ```
    pub fn parse(type_path: &str) -> CainomeResult<Self> {
        let t: Type = syn::parse_str(type_path)?;

        let mut inners = vec![];

        match t {
            Type::Tuple(t) => {
                if t.elems.is_empty() {
                    return Err(Error::TokenInitFailed(
                        "Unit type `()` is considered as a CoreBasic, not a tuple.".to_string(),
                    ));
                }

                for e in t.elems {
                    let ty = quote::quote!(#e).to_string().replace(' ', "");
                    inners.push(Token::parse(&ty)?);
                }
            }
            Type::Paren(t) => {
                // Tuple with one element are under `Paren` variant.
                let e = t.elem;
                let ty = quote::quote!(#e).to_string().replace(' ', "");
                inners.push(Token::parse(&ty)?);
            }
            _ => {
                return Err(Error::TokenInitFailed(format!(
                    "Tuple couldn't be initialized from `{}`.",
                    type_path,
                )));
            }
        }

        Ok(Self {
            type_path: type_path.to_string(),
            inners,
        })
    }

    pub fn apply_alias(&mut self, type_path: &str, alias: &str) {
        for i in &mut self.inners {
            i.apply_alias(type_path, alias);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::*;

    #[test]
    fn test_parse_unit_invalid() {
        assert!(Tuple::parse("()").is_err());
    }

    #[test]
    fn test_parse_one_inner() {
        assert_eq!(
            Tuple::parse("(core::felt252)").unwrap(),
            Tuple {
                type_path: "(core::felt252)".to_string(),
                inners: vec![Token::CoreBasic(CoreBasic {
                    type_path: "core::felt252".to_string()
                }),],
            }
        );
    }

    #[test]
    fn test_parse_multiple_inners() {
        assert_eq!(
            Tuple::parse("(core::felt252, core::integer::u64)").unwrap(),
            Tuple {
                type_path: "(core::felt252, core::integer::u64)".to_string(),
                inners: vec![
                    Token::CoreBasic(CoreBasic {
                        type_path: "core::felt252".to_string()
                    }),
                    Token::CoreBasic(CoreBasic {
                        type_path: "core::integer::u64".to_string()
                    }),
                ],
            }
        );
    }

    #[test]
    fn test_parse_other_type_invalid() {
        assert!(Tuple::parse("module::module2::MyStuct").is_err());
        assert!(Tuple::parse("core::integer::u64").is_err());
    }
}
