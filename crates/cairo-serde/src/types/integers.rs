//! CairoSerde implementation for integers (signed/unsigned).
use crate::{CairoSerde, Error, Result};
use starknet::core::types::Felt;

macro_rules! implement_trait_for_unsigned {
    ($type:ty) => {
        impl CairoSerde for $type {
            type RustType = Self;

            fn cairo_serialize(rust: &Self::RustType) -> Vec<Felt> {
                vec![Felt::from(*rust)]
            }

            fn cairo_deserialize(felts: &[Felt], offset: usize) -> Result<Self::RustType> {
                if offset >= felts.len() {
                    return Err(Error::Deserialize(format!(
                        "Buffer too short to deserialize a unsigned integer: offset ({}) : buffer {:?}",
                        offset,
                        felts,
                    )));
                }

                let temp: u128 = felts[offset].try_into().unwrap();
                Ok(temp as $type)
            }
        }
    };
}

macro_rules! implement_trait_for_signed {
    ($type:ty) => {
        impl CairoSerde for $type {
            type RustType = Self;

            fn cairo_serialize(rust: &Self::RustType) -> Vec<Felt> {
                vec![Felt::from(*rust as usize)]
            }

            fn cairo_deserialize(felts: &[Felt], offset: usize) -> Result<Self::RustType> {
                if offset >= felts.len() {
                    return Err(Error::Deserialize(format!(
                        "Buffer too short to deserialize a signed integer: offset ({}) : buffer {:?}",
                        offset,
                        felts,
                    )));
                }

                let temp: u128 = felts[offset].try_into().unwrap();
                Ok(temp as $type)
            }
        }
    };
}

implement_trait_for_unsigned!(u8);
implement_trait_for_unsigned!(u16);
implement_trait_for_unsigned!(u32);
implement_trait_for_unsigned!(u64);
implement_trait_for_unsigned!(u128);
implement_trait_for_unsigned!(usize);

implement_trait_for_signed!(i8);
implement_trait_for_signed!(i16);
implement_trait_for_signed!(i32);
implement_trait_for_signed!(i64);
implement_trait_for_signed!(i128);
implement_trait_for_signed!(isize);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_u8() {
        let v = 12_u8;
        let felts = u8::cairo_serialize(&v);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::from(12_u8));
    }

    #[test]
    fn test_deserialize_u8() {
        let felts = vec![Felt::from(12_u8), Felt::from(10_u8)];
        assert_eq!(u8::cairo_deserialize(&felts, 0).unwrap(), 12);
        assert_eq!(u8::cairo_deserialize(&felts, 1).unwrap(), 10);
    }

    #[test]
    fn test_serialize_u16() {
        let v = 12_u16;
        let felts = u16::cairo_serialize(&v);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::from(12_u16));
    }

    #[test]
    fn test_deserialize_u16() {
        let felts = vec![Felt::from(12_u16), Felt::from(10_u8)];
        assert_eq!(u16::cairo_deserialize(&felts, 0).unwrap(), 12);
        assert_eq!(u16::cairo_deserialize(&felts, 1).unwrap(), 10);
    }

    #[test]
    fn test_serialize_u32() {
        let v = 123_u32;
        let felts = u32::cairo_serialize(&v);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::from(123_u32));
    }

    #[test]
    fn test_deserialize_u32() {
        let felts = vec![Felt::from(123_u32), Felt::from(99_u32)];
        assert_eq!(u32::cairo_deserialize(&felts, 0).unwrap(), 123);
        assert_eq!(u32::cairo_deserialize(&felts, 1).unwrap(), 99);
    }

    #[test]
    fn test_serialize_u64() {
        let v = 123_u64;
        let felts = u64::cairo_serialize(&v);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::from(123_u64));
    }

    #[test]
    fn test_deserialize_u64() {
        let felts = vec![Felt::from(123_u64), Felt::from(99_u64)];
        assert_eq!(u64::cairo_deserialize(&felts, 0).unwrap(), 123);
        assert_eq!(u64::cairo_deserialize(&felts, 1).unwrap(), 99);
    }

    #[test]
    fn test_serialize_u128() {
        let v = 123_u128;
        let felts = u128::cairo_serialize(&v);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::from(123_u128));
    }

    #[test]
    fn test_deserialize_u128() {
        let felts = vec![Felt::from(123_u128), Felt::from(99_u128)];
        assert_eq!(u128::cairo_deserialize(&felts, 0).unwrap(), 123);
        assert_eq!(u128::cairo_deserialize(&felts, 1).unwrap(), 99);
    }

    #[test]
    fn test_serialize_usize() {
        let v = 123;
        let felts = usize::cairo_serialize(&v);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::from(123_u128));
    }

    #[test]
    fn test_deserialize_usize() {
        let felts = vec![Felt::from(123_u128), Felt::from(99_u64)];
        assert_eq!(usize::cairo_deserialize(&felts, 0).unwrap(), 123);
        assert_eq!(usize::cairo_deserialize(&felts, 1).unwrap(), 99);
    }

    #[test]
    fn test_serialize_i8() {
        let v = i8::MAX;
        let felts = i8::cairo_serialize(&v);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::from(i8::MAX as u8));
    }

    #[test]
    fn test_deserialize_i8() {
        let felts = vec![Felt::from(i8::MAX as u8), Felt::from(i8::MAX as u8)];
        assert_eq!(i8::cairo_deserialize(&felts, 0).unwrap(), i8::MAX);
        assert_eq!(i8::cairo_deserialize(&felts, 1).unwrap(), i8::MAX);
    }

    #[test]
    fn test_serialize_i16() {
        let v = i16::MAX;
        let felts = i16::cairo_serialize(&v);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::from(i16::MAX as u16));
    }

    #[test]
    fn test_deserialize_i16() {
        let felts = vec![Felt::from(i16::MAX as u16), Felt::from(i16::MAX as u16)];
        assert_eq!(i16::cairo_deserialize(&felts, 0).unwrap(), i16::MAX);
        assert_eq!(i16::cairo_deserialize(&felts, 1).unwrap(), i16::MAX);
    }

    #[test]
    fn test_serialize_i32() {
        let v = i32::MAX;
        let felts = i32::cairo_serialize(&v);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::from(i32::MAX as u32));
    }

    #[test]
    fn test_deserialize_i32() {
        let felts = vec![Felt::from(i32::MAX as u32), Felt::from(i32::MAX as u32)];
        assert_eq!(i32::cairo_deserialize(&felts, 0).unwrap(), i32::MAX);
        assert_eq!(i32::cairo_deserialize(&felts, 1).unwrap(), i32::MAX);
    }

    #[test]
    fn test_serialize_i64() {
        let v = i64::MAX;
        let felts = i64::cairo_serialize(&v);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::from(i64::MAX as u64));
    }

    #[test]
    fn test_deserialize_i64() {
        let felts = vec![Felt::from(i64::MAX as u64), Felt::from(i64::MAX as u64)];
        assert_eq!(i64::cairo_deserialize(&felts, 0).unwrap(), i64::MAX);
        assert_eq!(i64::cairo_deserialize(&felts, 1).unwrap(), i64::MAX);
    }

    #[test]
    fn test_deserialize_i128() {
        let felts = vec![Felt::from(i128::MAX as u128), Felt::from(i128::MAX as u128)];
        assert_eq!(i128::cairo_deserialize(&felts, 0).unwrap(), i128::MAX);
        assert_eq!(i128::cairo_deserialize(&felts, 1).unwrap(), i128::MAX);
    }
}
