//! CairoSerde implementation for NonZero.
//!
//! NonZero serializes with zero ( hehe :) ) overhead as the inner value
//!
//! https://github.com/starkware-libs/cairo/blob/main/corelib/src/zeroable.cairo#L38
use crate::{CairoSerde, ContractAddress, Result, U256};
use starknet::core::types::FieldElement;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct NonZero<T: Zeroable>(T);

impl<T: Zeroable> NonZero<T> {
    pub fn new(value: T) -> Option<Self> {
        if value.is_zero() {
            None
        } else {
            Some(NonZero(value))
        }
    }
    pub fn inner<'a>(&'a self) -> &'a T {
        &self.0
    }
    pub fn inner_mut<'a>(&'a mut self) -> &'a mut T {
        &mut self.0
    }
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T, RT> CairoSerde for NonZero<T>
where
    T: CairoSerde<RustType = RT>,
    T: Zeroable,
    RT: Zeroable,
{
    type RustType = NonZero<RT>;

    const SERIALIZED_SIZE: Option<usize> = T::SERIALIZED_SIZE;
    const DYNAMIC: bool = T::DYNAMIC;

    #[inline]
    fn cairo_serialized_size(rust: &Self::RustType) -> usize {
        T::cairo_serialized_size(&rust.0)
    }

    fn cairo_serialize(rust: &Self::RustType) -> Vec<FieldElement> {
        T::cairo_serialize(&rust.0)
    }

    fn cairo_deserialize(felts: &[FieldElement], offset: usize) -> Result<Self::RustType> {
        NonZero::new(T::cairo_deserialize(felts, offset)?).ok_or(crate::Error::ZeroedNonZero)
    }
}

pub trait Zeroable {
    fn is_zero(&self) -> bool;
}

macro_rules! implement_nonzeroable_for_integer {
    ($type:ty) => {
        impl Zeroable for $type {
            fn is_zero(&self) -> bool {
                *self == 0 as $type
            }
        }
    };
}

implement_nonzeroable_for_integer!(u8);
implement_nonzeroable_for_integer!(u16);
implement_nonzeroable_for_integer!(u32);
implement_nonzeroable_for_integer!(u64);
implement_nonzeroable_for_integer!(u128);
implement_nonzeroable_for_integer!(usize);
implement_nonzeroable_for_integer!(i8);
implement_nonzeroable_for_integer!(i16);
implement_nonzeroable_for_integer!(i32);
implement_nonzeroable_for_integer!(i64);
implement_nonzeroable_for_integer!(i128);
implement_nonzeroable_for_integer!(isize);

impl Zeroable for U256 {
    fn is_zero(&self) -> bool {
        self.low.is_zero() && self.high.is_zero()
    }
}

impl Zeroable for FieldElement {
    fn is_zero(&self) -> bool {
        *self == FieldElement::ZERO
    }
}

impl Zeroable for ContractAddress {
    fn is_zero(&self) -> bool {
        self.0 == FieldElement::ZERO
    }
}

#[cfg(test)]
mod tests {
    use crate::Error;

    use super::*;

    #[test]
    fn test_non_zero_cairo_serialize() {
        let non_zero = NonZero(1_u32);
        let felts = NonZero::<u32>::cairo_serialize(&non_zero);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], FieldElement::from(1_u32));
    }

    #[test]
    fn test_non_zero_cairo_deserialize() {
        let felts = vec![FieldElement::from(1_u32)];
        let non_zero = NonZero::<u32>::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(non_zero, NonZero(1_u32))
    }

    #[test]
    fn test_non_zero_cairo_deserialize_zero() {
        let felts = vec![FieldElement::ZERO, FieldElement::ZERO];
        let non_zero = NonZero::<U256>::cairo_deserialize(&felts, 0);
        match non_zero {
            Err(Error::ZeroedNonZero) => (),
            _ => panic!("Expected ZeroedNonZero error"),
        }
    }

    #[test]
    fn test_non_zero_const_size() {
        assert_eq!(NonZero::<u32>::SERIALIZED_SIZE, Some(1));
        assert_eq!(NonZero::<U256>::SERIALIZED_SIZE, Some(2));
        assert_eq!(NonZero::<i8>::DYNAMIC, false);
    }
}
