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

use std::collections::HashMap;

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

    /// Recursively hydrates nested tokens
    ///
    /// Once abi is parsed, a flat list of tokens defined in cairo code is generated from parsed
    /// json abi string.
    /// Then token list are filtered to only keep single copy of each token.
    /// Some tokens can have nested tokens that may not have inners defined inside thus leading to
    /// confusion while using tokens. i.e Enums does not have inner variants defined.
    ///
    /// # Arguments
    ///
    /// * `token` - The token to hydrate.
    /// * `filtered` - A map of type path to token that have already been hydrated.
    ///
    pub fn hydrate(token: Self, filtered: &HashMap<String, Token>) -> Self {
        match token {
            Token::CoreBasic(_) | Token::GenericArg(_) => token,
            Token::Array(arr) => Token::Array(Array {
                inner: Box::new(Self::hydrate(*arr.inner, filtered)),
                type_path: arr.type_path,
                is_legacy: arr.is_legacy,
            }),
            Token::Tuple(tup) => Token::Tuple(Tuple {
                inners: tup
                    .inners
                    .into_iter()
                    .map(|inner| Self::hydrate(inner, filtered))
                    .collect(),
                type_path: tup.type_path,
            }),
            Token::Composite(comp) => {
                if comp.r#type == CompositeType::Unknown && !comp.is_builtin() {
                    if let Some(hydrated) = filtered.get(&comp.type_path) {
                        return hydrated.clone();
                    } else {
                        panic!("Composite {} not found in filtered tokens", comp.type_path);
                    }
                }
                Token::Composite(Composite {
                    type_path: comp.type_path,
                    inners: comp
                        .inners
                        .into_iter()
                        .map(|i| CompositeInner {
                            index: i.index,
                            name: i.name,
                            kind: i.kind,
                            token: Self::hydrate(i.token, filtered),
                        })
                        .collect(),
                    generic_args: comp
                        .generic_args
                        .into_iter()
                        .map(|(name, token)| (name, Self::hydrate(token, filtered)))
                        .collect(),
                    r#type: comp.r#type,
                    is_event: comp.is_event,
                    alias: comp.alias,
                })
            }
            Token::Function(func) => Token::Function(Function {
                name: func.name,
                inputs: func
                    .inputs
                    .into_iter()
                    .map(|(name, token)| (name, Self::hydrate(token, filtered)))
                    .collect(),
                outputs: func
                    .outputs
                    .into_iter()
                    .map(|token| Self::hydrate(token, filtered))
                    .collect(),
                named_outputs: func
                    .named_outputs
                    .into_iter()
                    .map(|(name, token)| (name, Self::hydrate(token, filtered)))
                    .collect(),
                state_mutability: func.state_mutability,
            }),
        }
    }
}
