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

    // Placeholder is a service level value, those should never be exposed.
    // This is how placeholders are filtered out from Registry.
    // https://github.com/cartridge-gg/cainome/blob/a13e7e97c7b69529134d9d048e44b9919f3b9e07/crates/parser/src/abi/parser.rs#L191
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
        matches!(self, Token::Basic(_))
    }

    // TODO: this is only used to add brackets around tuple types in enum and struct fields.
    // Consider moving this logic elsewhere. ??? Tuple.to_rust_type_path
    pub fn is_tuple(&self) -> bool {
        matches!(self, Token::Tuple(_))
    }

    pub fn should_be_skipped(&self) -> bool {
        matches!(self, Token::Skip(_) | Token::Substitute(_))
    }

    pub fn is_container(&self) -> bool {
        matches!(
            self,
            Token::Array(_)
                | Token::Option(_)
                | Token::Result(_)
                | Token::NonZero(_)
                | Token::Tuple(_)
        )
    }
}
