//! This crate contains the definition of traits and types
//! that map to Cairo types that can then be (de)serializable from an array of `Felt`.
//!
//! Some of the Cairo types are provided in the ABI event if they are very generic
//! like `Option`, `Result`, etc...
//! This crate provides the `CairoSerde` implementation for those types and all basic
//! types from Cairo (integers, felt etc...).
//!
mod error;
pub use error::{Error, Result};

pub mod call;
pub mod serde_hex;
pub mod types;

pub use serde_hex::*;
pub use types::array_legacy::*;
pub use types::byte_array::*;
pub use types::non_zero::*;
pub use types::starknet::*;
pub use types::u256::*;
pub use types::*;

use ::starknet::core::types::Felt;

/// CairoSerde trait to implement in order to serialize/deserialize
/// a Rust type to/from a CairoSerde.
pub trait CairoSerde {
    /// The corresponding Rust type.
    type RustType;

    /// The serialized size of the type in felts, if known at compile time.
    const SERIALIZED_SIZE: Option<usize> = Some(1);

    /// Whether the serialized size is dynamic.
    const DYNAMIC: bool = Self::SERIALIZED_SIZE.is_none();

    /// Calculates the serialized size of the data for a single felt
    /// it will always be 1.
    /// If the type is dynamic, SERIALIZED_SIZE is None, but this

    /// function is overriden to correctly compute the size.
    #[inline]
    fn cairo_serialized_size(_rust: &Self::RustType) -> usize {
        Self::SERIALIZED_SIZE.unwrap()
    }

    /// Serializes the given type into a Felt sequence.
    fn cairo_serialize(rust: &Self::RustType) -> Vec<Felt>;

    /// TODO: add `serialize_to(rust: &Self::RustType, out: &mut Vec<Felt>)`.
    /// for large buffers optimization.

    /// Deserializes an array of felts into the given type.
    fn cairo_deserialize(felts: &[Felt], offset: usize) -> Result<Self::RustType>;
}
