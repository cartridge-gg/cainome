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
pub use composite::{CompositeInner, CompositeInnerKind, CompositeLegacy, CompositeType};
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
pub enum Container {
    Array(ArrayContainer),
    Option(OptionContainer),
    Result(ResultContainer),
    NonZero(NonZeroContainer),
    Tuple(TupleContainer),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntryToken {
    // Composite types
    Struct(Struct),
    Event(Event),
    Enum(Enum),
    Function(Function),
    Interface(Interface),

    // Placeholder is a service level value, those should never be exposed
    Placeholder,

    // Not needed for now
    Constructor(Constructor),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Basic type is well known cairo builtin.
    // It's stored in ABI as a reference and defined in caire
    // core lib.
    Basic(CoreBasic),
    // Container types are similar to Basic, but are well
    // known generic containers.
    Container(Container),
    // These are types defined by user, they might contain complex references.
    Entry(EntryToken),

    // Legacy
    CompositeLegacy(CompositeLegacy),
}

impl EntryToken {
    pub fn type_path(&self) -> String {
        let type_path = match self {
            EntryToken::Struct(s) => &s.type_path,
            EntryToken::Event(event) => &event.type_path,
            EntryToken::Enum(e) => &e.type_path,
            EntryToken::Function(function) => &function.name,
            EntryToken::Interface(interface) => &interface.type_path,
            EntryToken::Constructor(constructor) => &constructor.type_path,
            EntryToken::Placeholder => unreachable!(),
        };
        type_path.clone()
    }
}

impl Token {
    pub fn type_name(&self) -> String {
        match self {
            Token::Basic(t) => t.type_name(),

            Token::Container(Container::Array(_)) => "array".to_string(),
            Token::Container(Container::Tuple(_)) => "tuple".to_string(),
            Token::Container(Container::Option(_)) => "option".to_string(),
            Token::Container(Container::Result(_)) => "result".to_string(),
            Token::Container(Container::NonZero(_)) => "non_zero".to_string(),

            Token::Entry(EntryToken::Function(_)) => "function".to_string(),
            Token::Entry(EntryToken::Enum(e)) => e.type_name(),
            Token::Entry(EntryToken::Struct(s)) => s.type_name(),
            Token::Entry(EntryToken::Event(s)) => s.type_name(),
            Token::Entry(EntryToken::Interface(_)) => "interface".to_string(),
            Token::Entry(EntryToken::Constructor(_)) => "constructor".to_string(),
            Token::Entry(EntryToken::Placeholder) => unreachable!(),

            Token::CompositeLegacy(t) => t.type_name(),
        }
    }

    pub fn type_path(&self) -> String {
        match self {
            Token::Basic(t) => t.type_path.to_string(),

            Token::Container(Container::Array(t)) => t.type_path.to_string(),
            Token::Container(Container::Tuple(t)) => t.type_path.to_string(),
            Token::Container(Container::Option(t)) => t.type_path.to_string(),
            Token::Container(Container::Result(t)) => t.type_path.to_string(),
            Token::Container(Container::NonZero(t)) => t.type_path.to_string(),

            Token::Entry(EntryToken::Function(t)) => t.name.clone(),
            Token::Entry(EntryToken::Enum(e)) => e.type_path_no_generic(),
            Token::Entry(EntryToken::Struct(s)) => s.type_path_no_generic(),
            Token::Entry(EntryToken::Event(s)) => s.type_path_no_generic(),
            Token::Entry(EntryToken::Interface(i)) => i.type_path.to_string(),
            Token::Entry(EntryToken::Constructor(c)) => c.type_path.to_string(),
            Token::Entry(EntryToken::Placeholder) => unreachable!(),

            Token::CompositeLegacy(t) => t.type_path_no_generic(),
        }
    }

    // TODO: we may remove these two functions...! And change types somewhere..
    pub fn to_composite(&self) -> CainomeResult<&CompositeLegacy> {
        match self {
            Token::CompositeLegacy(t) => Ok(t),
            _ => Err(Error::ConversionFailed(format!(
                "Can't convert token into composite, got {:?}",
                self
            ))),
        }
    }

    pub fn to_function(&self) -> CainomeResult<&Function> {
        match self {
            Token::Entry(EntryToken::Function(t)) => Ok(t),
            _ => Err(Error::ConversionFailed(format!(
                "Can't convert token into function, got {:?}",
                self
            ))),
        }
    }
}
