use crate::CairoSerde;
pub struct U256 {
    pub low: u128,
    pub high: u128,
}
impl CairoSerde for U256 {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size += u128::cairo_serialized_size(&__rust.low);
        __size += u128::cairo_serialized_size(&__rust.high);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::FieldElement> {
        let mut __out: Vec<starknet::core::types::FieldElement> = vec![];
        __out.extend(u128::cairo_serialize(&__rust.low));
        __out.extend(u128::cairo_serialize(&__rust.high));
        __out
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::FieldElement],
        __offset: usize,
    ) -> Result<Self::RustType, crate::Error> {
        let mut __offset = __offset;
        let low = u128::cairo_deserialize(__felts, __offset)?;
        __offset += u128::cairo_serialized_size(&low);
        let high = u128::cairo_deserialize(__felts, __offset)?;
        __offset += u128::cairo_serialized_size(&high);
        Ok(U256 { low, high })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starknet::core::types::FieldElement;
    #[test]
    fn test_serialize_u256() {
        let low = 9_u128;
        let high = 8_u128;
        let felts = U256::cairo_serialize(&U256 { low, high });
        assert_eq!(felts.len(), 2);
        assert_eq!(felts[0], FieldElement::from(9_u128));
        assert_eq!(felts[1], FieldElement::from(8_u128));
    }
    #[test]
    fn test_serialize_u256_max() {
        let low = u128::MAX;
        let high = u128::MAX;
        let felts = U256::cairo_serialize(&U256 { low, high });
        assert_eq!(felts.len(), 2);
        assert_eq!(felts[0], FieldElement::from(u128::MAX));
        assert_eq!(felts[1], FieldElement::from(u128::MAX));
    }
    #[test]
    fn test_serialize_u256_min() {
        let low = u128::MIN;
        let high = u128::MIN;
        let felts = U256::cairo_serialize(&U256 { low, high });
        assert_eq!(felts.len(), 2);
        assert_eq!(felts[0], FieldElement::from(u128::MIN));
        assert_eq!(felts[1], FieldElement::from(u128::MIN));
    }
    #[test]
    fn test_deserialize_u256() {
        let felts = vec![FieldElement::from(9_u128), FieldElement::from(8_u128)];
            let num_u256 = U256::cairo_deserialize(&felts, 0).unwrap();
            assert_eq!(num_u256.low, 9_u128);
            assert_eq!(num_u256.high, 8_u128);
    }
    #[test]
    fn test_serialized_size_u256() {
        let u256 = U256 {
            low: 9_u128,
            high: 8_u128,
        };
        assert_eq!(U256::cairo_serialized_size(&u256), 2);
    }
}
