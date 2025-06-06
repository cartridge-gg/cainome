//! Cairo ABI tokens.
//!
//! TODO.

mod array;
mod basic;
mod composite;
mod constants;
mod function;
mod genericity;
mod non_zero;
mod option;
mod result;
mod tuple;

use std::collections::HashMap;

pub use array::Array;
pub use basic::CoreBasic;
pub use composite::{Composite, CompositeInner, CompositeInnerKind, CompositeType};
pub use function::{Function, FunctionOutputKind, StateMutability};
pub use non_zero::NonZero;
pub use option::Option;
pub use result::Result;
pub use tuple::Tuple;

use crate::{CainomeResult, Error};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    CoreBasic(CoreBasic),
    Array(Array),
    Tuple(Tuple),
    Composite(Composite),
    Function(Function),
    Option(Option),
    Result(Result),
    NonZero(NonZero),
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

        if let Ok(o) = Option::parse(type_path) {
            return Ok(Token::Option(o));
        }

        if let Ok(r) = Result::parse(type_path) {
            return Ok(Token::Result(r));
        }

        if let Ok(n) = NonZero::parse(type_path) {
            return Ok(Token::NonZero(n));
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
            Token::Function(_) => "function".to_string(),
            Token::Option(_) => "option".to_string(),
            Token::Result(_) => "result".to_string(),
            Token::NonZero(_) => "non_zero".to_string(),
        }
    }

    pub fn type_path(&self) -> String {
        match self {
            Token::CoreBasic(t) => t.type_path.to_string(),
            Token::Array(t) => t.type_path.to_string(),
            Token::Tuple(t) => t.type_path.to_string(),
            Token::Composite(t) => t.type_path_no_generic(),
            Token::Function(t) => t.name.clone(),
            Token::Option(t) => t.type_path.to_string(),
            Token::Result(t) => t.type_path.to_string(),
            Token::NonZero(t) => t.type_path.to_string(),
        }
    }

    pub fn deepen(&mut self, depth: usize) {
        match self {
            Token::Composite(t) => t.depth += depth,
            _ => (),
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

    pub fn apply_alias(&mut self, type_path: &str, alias: &str) {
        match self {
            Token::Array(t) => t.apply_alias(type_path, alias),
            Token::Tuple(t) => t.apply_alias(type_path, alias),
            Token::Composite(t) => t.apply_alias(type_path, alias),
            Token::Function(t) => t.apply_alias(type_path, alias),
            Token::Option(t) => t.apply_alias(type_path, alias),
            Token::Result(t) => t.apply_alias(type_path, alias),
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
    /// * `recursion_max_depth` - Max depth recursion for token to hydrate.
    /// * `iteration_count` - Current iteration count.
    ///
    pub fn hydrate(
        token: Self,
        filtered: &HashMap<String, Token>,
        recursion_max_depth: usize,
        iteration_count: usize,
    ) -> Self {
        if recursion_max_depth < iteration_count {
            return token;
        }
        match token {
            Token::CoreBasic(_) => token,
            Token::Array(arr) => Token::Array(Array {
                inner: Box::new(Self::hydrate(
                    *arr.inner,
                    filtered,
                    recursion_max_depth,
                    iteration_count + 1,
                )),
                type_path: arr.type_path,
                is_legacy: arr.is_legacy,
            }),
            Token::Tuple(tup) => Token::Tuple(Tuple {
                inners: tup
                    .inners
                    .into_iter()
                    .map(|inner| {
                        Self::hydrate(inner, filtered, recursion_max_depth, iteration_count + 1)
                    })
                    .collect(),
                type_path: tup.type_path,
            }),
            Token::Option(opt) => Token::Option(Option {
                type_path: opt.type_path,
                inner: Box::new(Self::hydrate(
                    *opt.inner,
                    filtered,
                    recursion_max_depth,
                    iteration_count + 1,
                )),
            }),
            Token::NonZero(non_zero) => Token::NonZero(NonZero {
                type_path: non_zero.type_path,
                inner: Box::new(Self::hydrate(
                    *non_zero.inner,
                    filtered,
                    recursion_max_depth,
                    iteration_count + 1,
                )),
            }),
            Token::Result(res) => Token::Result(Result {
                type_path: res.type_path,
                inner: Box::new(Self::hydrate(
                    *res.inner,
                    filtered,
                    recursion_max_depth,
                    iteration_count + 1,
                )),
                error: Box::new(Self::hydrate(
                    *res.error,
                    filtered,
                    recursion_max_depth,
                    iteration_count + 1,
                )),
            }),
            Token::Composite(comp) => {
                let type_path = comp.type_path_no_generic();

                if comp.r#type == CompositeType::Unknown && !comp.is_builtin() {
                    if let Some(hydrated) = filtered.get(&type_path) {
                        return Token::hydrate(
                            hydrated.clone(),
                            filtered,
                            recursion_max_depth,
                            iteration_count + 1,
                        );
                    } else {
                        panic!("Composite {} not found in filtered tokens", type_path);
                    }
                }
                Token::Composite(Composite {
                    type_path,
                    inners: comp
                        .inners
                        .into_iter()
                        .map(|i| CompositeInner {
                            index: i.index,
                            name: i.name,
                            kind: i.kind,
                            token: Self::hydrate(
                                i.token,
                                filtered,
                                recursion_max_depth,
                                iteration_count + 1,
                            ),
                        })
                        .collect(),
                    generic_args: comp
                        .generic_args
                        .into_iter()
                        .map(|(name, token)| {
                            (
                                name,
                                Self::hydrate(
                                    token,
                                    filtered,
                                    recursion_max_depth,
                                    iteration_count + 1,
                                ),
                            )
                        })
                        .collect(),
                    r#type: comp.r#type,
                    is_event: comp.is_event,
                    alias: comp.alias,
                    depth: 0,
                })
            }
            Token::Function(func) => Token::Function(Function {
                name: func.name,
                inputs: func
                    .inputs
                    .into_iter()
                    .map(|(name, token)| {
                        (
                            name,
                            Self::hydrate(
                                token,
                                filtered,
                                recursion_max_depth,
                                iteration_count + 1,
                            ),
                        )
                    })
                    .collect(),
                outputs: func
                    .outputs
                    .into_iter()
                    .map(|token| {
                        Self::hydrate(token, filtered, recursion_max_depth, iteration_count + 1)
                    })
                    .collect(),
                named_outputs: func
                    .named_outputs
                    .into_iter()
                    .map(|(name, token)| {
                        (
                            name,
                            Self::hydrate(
                                token,
                                filtered,
                                recursion_max_depth,
                                iteration_count + 1,
                            ),
                        )
                    })
                    .collect(),
                state_mutability: func.state_mutability,
            }),
        }
    }
}
