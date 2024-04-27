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
        let mut size = 0;
        size += u128::cairo_serialized_size(&__rust.low);
        size += u128::cairo_serialized_size(&__rust.high);
        size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::FieldElement> {
        let mut out: Vec<starknet::core::types::FieldElement> = vec![];
        out.extend(u128::cairo_serialize(&__rust.low));
        out.extend(u128::cairo_serialize(&__rust.high));
        out
    }
    fn cairo_deserialize(
        felts: &[starknet::core::types::FieldElement],
        offset: usize,
    ) -> Result<Self::RustType, crate::Error> {
        let mut offset = offset;
        let low = u128::cairo_deserialize(felts, offset)?;
        offset += u128::cairo_serialized_size(&low);
        let high = u128::cairo_deserialize(felts, offset)?;
        offset += u128::cairo_serialized_size(&high);
        Ok(U256 { low, high })
    }
}
impl U256 {
    pub fn to_bytes_be(&self) -> [u8; 32]{
        let mut bytes = [0; 32];
        bytes[0..16].copy_from_slice(&self.high.to_be_bytes());
        bytes[16..32].copy_from_slice(&self.low.to_be_bytes());
        bytes
    }
    pub fn to_bytes_le(&self) -> [u8 ;32]{
        let mut bytes = [0; 32];
        bytes[0..16].copy_from_slice(&self.low.to_le_bytes());
        bytes[16..32].copy_from_slice(&self.high.to_le_bytes());
        bytes
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
    #[test]
    fn test_to_bytes_be() {
        let u256 = U256 {
            low: 9_u128,
            high: 8_u128,
        };
        let bytes = u256.to_bytes_be();
        let expected_bytes: [u8; 32] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8,  // high
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9,  // low
        ];
        assert_eq!(bytes,expected_bytes);
    }
    #[test]
    fn test_to_bytes_le() {
        let u256 = U256 {
            low: 9_u128,
            high: 8_u128,
        };
        let bytes = u256.to_bytes_le();
        let expected_bytes: [u8; 32] = [
            9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,  // low
            8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,  // high
        ];
        assert_eq!(bytes,expected_bytes);
    }
}
