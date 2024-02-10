//! Cairo ABI tokens.
//!
//! TODO.

mod array;
mod basic;
mod composite;
mod constants;
mod function;
mod genericity;
mod tuple;

pub use array::Array;
pub use basic::CoreBasic;
pub use composite::{Composite, CompositeInner, CompositeInnerKind, CompositeType};
pub use function::{Function, FunctionOutputKind, StateMutability};
pub use tuple::Tuple;

use crate::{CainomeResult, Error};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    CoreBasic(CoreBasic),
    Array(Array),
    Tuple(Tuple),
    Composite(Composite),
    GenericArg(String),
    Function(Function),
}

impl Token {
    pub fn parse(type_path: &str) -> CainomeResult<Self> {
        if let Ok(b) = CoreBasic::parse(type_path) {
            return Ok(Token::CoreBasic(b));
        }

        if let Ok(a) = Array::parse(type_path) {
            return Ok(Token::Array(a));
        }

        if let Ok(t) = Tuple::parse(type_path) {
            return Ok(Token::Tuple(t));
        }

        if let Ok(c) = Composite::parse(type_path) {
            return Ok(Token::Composite(c));
        }

        Err(Error::TokenInitFailed(format!(
            "Couldn't initialize a Token from type path `{}`",
            type_path,
        )))
    }

    pub fn type_name(&self) -> String {
        match self {
            Token::CoreBasic(t) => t.type_name(),
            Token::Array(_) => "array".to_string(),
            Token::Tuple(_) => "tuple".to_string(),
            Token::Composite(t) => t.type_name(),
            Token::GenericArg(_) => "generic_arg".to_string(),
            Token::Function(_) => "function".to_string(),
        }
    }

    pub fn type_path(&self) -> String {
        match self {
            Token::CoreBasic(t) => t.type_path.to_string(),
            Token::Array(t) => t.type_path.to_string(),
            Token::Tuple(t) => t.type_path.to_string(),
            Token::Composite(t) => t.type_path_no_generic(),
            Token::GenericArg(_) => "generic".to_string(),
            Token::Function(t) => t.name.clone(),
        }
    }

    // TODO: we may remove these two functions...! And change types somewhere..
    pub fn to_composite(&self) -> CainomeResult<&Composite> {
        match self {
            Token::Composite(t) => Ok(t),
            _ => Err(Error::ConversionFailed(format!(
                "Can't convert token into composite, got {:?}",
                self
            ))),
        }
    }

    pub fn to_function(&self) -> CainomeResult<&Function> {
        match self {
            Token::Function(t) => Ok(t),
            _ => Err(Error::ConversionFailed(format!(
                "Can't convert token into function, got {:?}",
                self
            ))),
        }
    }

    pub fn resolve_generic(&self, generic_name: &str, generic_type_path: &str) -> Self {
        match self {
            Token::CoreBasic(t) => {
                if t.type_path == generic_type_path {
                    Token::GenericArg(generic_name.to_string())
                } else {
                    self.clone()
                }
            }
            Token::Array(t) => t.resolve_generic(generic_name, generic_type_path),
            Token::Tuple(t) => t.resolve_generic(generic_name, generic_type_path),
            Token::Composite(t) => t.resolve_generic(generic_name, generic_type_path),
            Token::GenericArg(_) => self.clone(),
            Token::Function(_) => self.clone(),
        }
    }

    pub fn apply_alias(&mut self, type_path: &str, alias: &str) {
        match self {
            Token::Array(t) => t.apply_alias(type_path, alias),
            Token::Tuple(t) => t.apply_alias(type_path, alias),
            Token::Composite(t) => t.apply_alias(type_path, alias),
            Token::Function(t) => t.apply_alias(type_path, alias),
            _ => (),
        }
    }
}
