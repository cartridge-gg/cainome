//! Cairo ABI tokens.
//!
//! TODO.

mod array;
mod basic;
pub mod constants;
mod constructor;
mod enumeration;
mod event;
mod function;
pub mod genericity;
mod interface;
mod named_token;
mod non_zero;
mod option;
mod result;
mod structure;
mod tuple;
pub mod utils;

pub use array::ArrayContainer;
pub use basic::TypePath;
pub use constructor::Constructor;
pub use enumeration::Enum;
pub use event::{Event, EventKind};
pub use function::{Function, FunctionOutputKind, StateMutability};
pub use interface::Interface;
pub use named_token::NamedToken;
pub use non_zero::NonZeroContainer;
pub use option::OptionContainer;
pub use result::ResultContainer;
pub use structure::Struct;
pub use tuple::TupleContainer;

#[derive(Debug, Clone, PartialEq)]
pub enum Container {}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Basic type is well known cairo builtin.
    // It's stored in ABI as a reference and defined in caire
    // core lib.
    Basic(TypePath),

    // Container types are similar to Basic, but are well
    // known generic containers.
    Array(ArrayContainer),
    Option(OptionContainer),
    Result(ResultContainer),
    NonZero(NonZeroContainer),
    Tuple(TupleContainer),

    // Composite types
    Struct(Struct),
    Event(Event),

    Enum(Enum),
    Function(Function),
    Interface(Interface),

    // Placeholder is a service level value, those should never be exposed
    Placeholder,

    // Extension token is not expanded by default, it's used to mark a token
    // that implementation is provided externally via a custom crate
    Substitute(TypePath),

    // Types to be skipped from generation
    Skip(TypePath),

    // Not needed for now
    Constructor(Constructor),
}

impl Token {
    pub fn is_basic(&self) -> bool {
        if let Token::Basic(_) = self {
            true
        } else {
            false
        }
    }

    // TODO: this is only used to add brackets around tuple types in enum and struct fields.
    // Consider moving this logic elsewhere. ??? Tuple.to_rust_type_path
    pub fn is_tuple(&self) -> bool {
        if let Token::Tuple(_) = self {
            true
        } else {
            false
        }
    }
}
