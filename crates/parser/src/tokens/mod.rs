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

#[derive(Debug, Clone, PartialEq)]
pub enum Container {}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Basic type is well known cairo builtin.
    // It's stored in ABI as a reference and defined in caire
    // core lib.
    Basic(CoreBasic),

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

    // Not needed for now
    Constructor(Constructor),

    // Legacy
    CompositeLegacy(CompositeLegacy),
}

impl Token {
    pub fn is_basic(&self) -> bool {
        if let Token::Basic(_) = self {
            true
        } else {
            false
        }
    }
}
