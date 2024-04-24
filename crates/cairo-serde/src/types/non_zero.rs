//! CairoSerde implementation for NonZero.
//!
//! NonZero serializes with zero ( hehe :) ) overhead as the inner value
//!
//! https://github.com/starkware-libs/cairo/blob/main/corelib/src/zeroable.cairo#L38
use crate::{CairoSerde, Result};
use starknet::core::types::FieldElement;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct NonZero<T>(pub T);

impl<T, RT> CairoSerde for NonZero<T>
where
    T: CairoSerde<RustType = RT>,
{
    type RustType = NonZero<RT>;

    #[inline]
    fn cairo_serialized_size(rust: &Self::RustType) -> usize {
        T::cairo_serialized_size(&rust.0)
    }

    fn cairo_serialize(rust: &Self::RustType) -> Vec<FieldElement> {
        T::cairo_serialize(&rust.0)
    }

    fn cairo_deserialize(felts: &[FieldElement], offset: usize) -> Result<Self::RustType> {
        Ok(NonZero(T::cairo_deserialize(felts, offset)?))
    }
}
