//! Cairo ABI tokens.
//!
//! TODO.

mod array;
mod basic;
mod composite;
mod constants;
mod enumeration;
mod event;
mod function;
mod genericity;
mod non_zero;
mod option;
mod result;
mod structure;
mod tuple;
pub mod utils;

use std::{collections::HashMap, rc::Rc};

pub use array::Array;
pub use basic::CoreBasic;
pub use composite::{Composite, CompositeInner, CompositeInnerKind, CompositeType};
pub use enumeration::{Enum, EnumInner};
pub use event::{Event, EventInner};
pub use function::{Function, FunctionOutputKind, StateMutability};
pub use non_zero::NonZero;
pub use option::Option;
pub use result::Result;
pub use structure::{Struct, StructInner};
pub use tuple::Tuple;

use crate::{CainomeResult, Error};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    CoreBasic(CoreBasic),
    Array(Array),
    Tuple(Tuple),
    Enum(Enum),
    Struct(Struct),
    Event(Event),
    Composite(Composite),
    Function(Function),
    Option(Option),
    Result(Result),
    NonZero(NonZero),
}

type Parser = fn(&str) -> CainomeResult<Token>;

impl Token {
    const BASIC_PARSERS: &[Parser] = &[
        |s| CoreBasic::parse(s).map(Token::CoreBasic),
        |s| Array::parse(s).map(Token::Array),
        |s| Tuple::parse(s).map(Token::Tuple),
        |s| Option::parse(s).map(Token::Option),
        |s| Result::parse(s).map(Token::Result),
        |s| NonZero::parse(s).map(Token::NonZero),
        |s| Struct::parse(s).map(Token::Struct),
        |s| Enum::parse(s).map(Token::Enum),
        |s| Event::parse(s).map(Token::Event),
        |s| Composite::parse(s).map(Token::Composite),
    ];

    pub fn parse(
        type_path: &str,
        // registry: &mut HashMap<String, Rc<Token>>,
    ) -> CainomeResult<Rc<Self>> {
        // if let Some(token) = registry.get(type_path) {
        //     return Ok(Rc::clone(token));
        // }

        for parser in Self::BASIC_PARSERS {
            if let Ok(tok) = parser(type_path) {
                let value = Rc::new(tok);
                // registry.insert(type_path.to_string(), Rc::clone(&value));
                return Ok(value);
            }
        }

        Err(Error::TokenInitFailed(format!(
            "Couldn't initialize a Token from type path `{}`. Please, check if parser is registered.",
            type_path,
        )))
    }

    pub fn type_name(&self) -> String {
        match self {
            Token::CoreBasic(t) => t.type_name(),
            Token::Array(_) => "array".to_string(),
            Token::Tuple(_) => "tuple".to_string(),
            Token::Function(_) => "function".to_string(),
            Token::Option(_) => "option".to_string(),
            Token::Result(_) => "result".to_string(),
            Token::NonZero(_) => "non_zero".to_string(),
            Token::Composite(t) => t.type_name(),
            Token::Enum(e) => e.type_name(),
            Token::Struct(s) => s.type_name(),
            Token::Event(s) => s.type_name(),
        }
    }

    pub fn type_path(&self) -> String {
        match self {
            Token::CoreBasic(t) => t.type_path.to_string(),
            Token::Array(t) => t.type_path.to_string(),
            Token::Tuple(t) => t.type_path.to_string(),
            Token::Function(t) => t.name.clone(),
            Token::Option(t) => t.type_path.to_string(),
            Token::Result(t) => t.type_path.to_string(),
            Token::NonZero(t) => t.type_path.to_string(),
            Token::Composite(t) => t.type_path_no_generic(),
            Token::Enum(e) => e.type_path_no_generic(),
            Token::Struct(s) => s.type_path_no_generic(),
            Token::Event(s) => s.type_path_no_generic(),
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
                inner: Rc::new(Self::hydrate(
                    arr.inner.as_ref().clone(),
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
                        Self::hydrate(
                            inner.as_ref().clone(),
                            filtered,
                            recursion_max_depth,
                            iteration_count + 1,
                        )
                    })
                    .map(Rc::new)
                    .collect(),
                type_path: tup.type_path,
            }),
            Token::Option(opt) => Token::Option(Option {
                type_path: opt.type_path,
                inner: Rc::new(Self::hydrate(
                    opt.inner.as_ref().clone(),
                    filtered,
                    recursion_max_depth,
                    iteration_count + 1,
                )),
            }),
            Token::NonZero(non_zero) => Token::NonZero(NonZero {
                type_path: non_zero.type_path,
                inner: Rc::new(Self::hydrate(
                    non_zero.inner.as_ref().clone(),
                    filtered,
                    recursion_max_depth,
                    iteration_count + 1,
                )),
            }),
            Token::Result(res) => Token::Result(Result {
                type_path: res.type_path,
                inner: Rc::new(Self::hydrate(
                    res.inner.as_ref().clone(),
                    filtered,
                    recursion_max_depth,
                    iteration_count + 1,
                )),
                error: Rc::new(Self::hydrate(
                    res.inner.as_ref().clone(),
                    filtered,
                    recursion_max_depth,
                    iteration_count + 1,
                )),
            }),
            Token::Composite(comp) => {
                let type_path = comp.type_path_no_generic();
                let is_builtin = utils::is_builtin(&comp.type_path);
                if comp.r#type == CompositeType::Unknown && is_builtin {
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
                            token: Rc::new(Self::hydrate(
                                i.token.as_ref().clone(),
                                filtered,
                                recursion_max_depth,
                                iteration_count + 1,
                            )),
                        })
                        .collect(),
                    generic_args: comp
                        .generic_args
                        .into_iter()
                        .map(|(name, token)| {
                            (
                                name,
                                Rc::new(Self::hydrate(
                                    token.as_ref().clone(),
                                    filtered,
                                    recursion_max_depth,
                                    iteration_count + 1,
                                )),
                            )
                        })
                        .collect(),
                    r#type: comp.r#type,
                    is_event: comp.is_event,
                    alias: comp.alias,
                })
            }
            Token::Enum(e) => Token::Enum(Enum {
                type_path: e.type_path,
                variants: e
                    .variants
                    .into_iter()
                    .map(|inner| EnumInner {
                        name: inner.name,
                        token: Rc::new(Self::hydrate(
                            inner.token.as_ref().clone(),
                            filtered,
                            recursion_max_depth,
                            iteration_count + 1,
                        )),
                    })
                    .collect(),
                generic_args: e
                    .generic_args
                    .into_iter()
                    .map(|(name, token)| {
                        (
                            name,
                            Rc::new(Self::hydrate(
                                token.as_ref().clone(),
                                filtered,
                                recursion_max_depth,
                                iteration_count + 1,
                            )),
                        )
                    })
                    .collect(),
                alias: e.alias,
            }),
            Token::Struct(e) => Token::Struct(Struct {
                type_path: e.type_path,
                fields: e
                    .fields
                    .into_iter()
                    .map(|inner| StructInner {
                        name: inner.name,
                        token: Rc::new(Self::hydrate(
                            inner.token.as_ref().clone(),
                            filtered,
                            recursion_max_depth,
                            iteration_count + 1,
                        )),
                    })
                    .collect(),
                generic_args: e
                    .generic_args
                    .into_iter()
                    .map(|(name, token)| {
                        (
                            name,
                            Rc::new(Self::hydrate(
                                token.as_ref().clone(),
                                filtered,
                                recursion_max_depth,
                                iteration_count + 1,
                            )),
                        )
                    })
                    .collect(),
                alias: e.alias,
            }),
            Token::Event(e) => Token::Event(Event {
                type_path: e.type_path,
                keys: e
                    .keys
                    .into_iter()
                    .map(|inner| EventInner {
                        name: inner.name,
                        token: Rc::new(Self::hydrate(
                            inner.token.as_ref().clone(),
                            filtered,
                            recursion_max_depth,
                            iteration_count + 1,
                        )),
                    })
                    .collect(),
                data: e
                    .data
                    .into_iter()
                    .map(|inner| EventInner {
                        name: inner.name,
                        token: Rc::new(Self::hydrate(
                            inner.token.as_ref().clone(),
                            filtered,
                            recursion_max_depth,
                            iteration_count + 1,
                        )),
                    })
                    .collect(),
                nested: e
                    .nested
                    .into_iter()
                    .map(|inner| EventInner {
                        name: inner.name,
                        token: Rc::new(Self::hydrate(
                            inner.token.as_ref().clone(),
                            filtered,
                            recursion_max_depth,
                            iteration_count + 1,
                        )),
                    })
                    .collect(),
                flat: e
                    .flat
                    .into_iter()
                    .map(|inner| EventInner {
                        name: inner.name,
                        token: Rc::new(Self::hydrate(
                            inner.token.as_ref().clone(),
                            filtered,
                            recursion_max_depth,
                            iteration_count + 1,
                        )),
                    })
                    .collect(),
                generic_args: e
                    .generic_args
                    .into_iter()
                    .map(|(name, token)| {
                        (
                            name,
                            Rc::new(Self::hydrate(
                                token.as_ref().clone(),
                                filtered,
                                recursion_max_depth,
                                iteration_count + 1,
                            )),
                        )
                    })
                    .collect(),
                alias: e.alias,
            }),
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
