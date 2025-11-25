//! Cairo ABI tokens.
//!
//! TODO.

mod array;
mod basic;
mod composite;
pub mod constants;
mod constructor;
mod enumeration;
mod event;
mod function;
mod genericity;
mod interface;
mod non_zero;
mod option;
mod result;
mod structure;
mod tuple;
pub mod utils;

pub use array::ArrayContainer;
pub use basic::CoreBasic;
pub use composite::{Composite, CompositeInner, CompositeInnerKind, CompositeType};
pub use constructor::Constructor;
pub use enumeration::{Enum, EnumInner};
pub use event::{Event, EventInner};
pub use function::{FuncInner, Function, FunctionOutputKind, StateMutability};
pub use interface::Interface;
pub use non_zero::NonZeroContainer;
pub use option::OptionContainer;
pub use result::ResultContainer;
pub use structure::{Struct, StructInner};
pub use tuple::TupleContainer;

use crate::{CainomeResult, Error};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Builtin types
    CoreBasic(CoreBasic),

    // Container Types
    Array(ArrayContainer),
    Enum(Enum),
    Option(OptionContainer),
    Result(ResultContainer),
    NonZero(NonZeroContainer),

    // Composite types
    Tuple(TupleContainer),
    Struct(Struct),
    Event(Event),

    Function(Function),
    Interface(Interface),

    // Not needed for now
    Constructor(Constructor),

    // Suspicious
    Blank,

    // Legacy
    Composite(Composite),
}

impl Token {
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
            Token::Interface(f) => "interface".to_string(),
            Token::Constructor(f) => "constructor".to_string(),
            Token::Blank => "empty".to_string(),
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
            Token::Constructor(constructor) => constructor.type_path.to_string(),
            Token::Interface(interface) => interface.type_path.to_string(),
            Token::Blank => "".to_string(),
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
}
