use super::constants::CAIRO_CORE_SPAN_ARRAY;
use super::genericity;
use super::Token;
use crate::{CainomeResult, Error};

#[derive(Debug, Clone, PartialEq)]
pub struct Array {
    pub type_path: String,
    pub inner: Box<Token>,
}

impl Array {
    pub fn parse(type_path: &str) -> CainomeResult<Self> {
        for a in CAIRO_CORE_SPAN_ARRAY {
            if type_path.starts_with(a) {
                let generic_args = genericity::extract_generics_args(type_path)?;

                if generic_args.len() != 1 {
                    return Err(Error::TokenInitFailed(format!(
                        "Array/Span are expected exactly one generic argument, found {} in `{}`.",
                        generic_args.len(),
                        type_path,
                    )));
                }

                let (_, generic_arg_token) = &generic_args[0];

                return Ok(Self {
                    type_path: type_path.to_string(),
                    inner: Box::new(generic_arg_token.clone()),
                });
            }
        }

        Err(Error::TokenInitFailed(format!(
            "Array/Span couldn't be initialized from `{}`.",
            type_path,
        )))
    }

    pub fn resolve_generic(&self, generic_name: &str, generic_type_path: &str) -> Token {
        if self.type_path == generic_type_path {
            Token::GenericArg(generic_name.to_string())
        } else {
            Token::Array(Self {
                type_path: self.type_path.clone(),
                inner: Box::new(self.inner.resolve_generic(generic_name, generic_type_path)),
            })
        }
    }

    pub fn apply_alias(&mut self, type_path: &str, alias: &str) {
        self.inner.apply_alias(type_path, alias);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::*;

    #[test]
    fn test_parse() {
        assert_eq!(
            Array::parse("core::array::Array::<core::felt252>").unwrap(),
            Array {
                type_path: "core::array::Array::<core::felt252>".to_string(),
                inner: Box::new(Token::CoreBasic(CoreBasic {
                    type_path: "core::felt252".to_string()
                })),
            }
        );
    }

    #[test]
    fn test_parse_no_inner_invalid() {
        assert!(Array::parse("core::array::Array").is_err());
        assert!(Array::parse("core::array::Array<>").is_err());
    }

    #[test]
    fn test_parse_wrong_path_invalid() {
        assert!(Array::parse("array::Array::<core::felt252>").is_err());
    }

    #[test]
    fn test_parse_invalid_path_invalid() {
        assert!(Array::parse("module::module2::array::Array::<core::felt252>").is_err());
        assert!(Array::parse("module::module2::MyStruct::<core::felt252>").is_err());
    }
}
