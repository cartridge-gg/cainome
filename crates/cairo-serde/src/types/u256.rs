use crate::CairoSerde;
use starknet::core::types::FieldElement;
pub struct U256 {
    pub low: u128,
    pub high: u128,
}
impl CairoSerde for U256 {
    type RustType = Self;
    const SERIALIZED_SIZE: Option<usize> = None;
    #[inline]
    fn cairo_serialized_size(this: &U256) -> usize {
        u128::cairo_serialized_size(&this.low) + u128::cairo_serialized_size(&this.high)
    }
    fn cairo_serialize(this: &U256) -> Vec<FieldElement> {
        [
            u128::cairo_serialize(&this.low),
            u128::cairo_serialize(&this.high),
        ]
        .concat()
    }
    fn cairo_deserialize(felts: &[FieldElement], offset: usize) -> Result<U256, crate::Error> {
        let low = u128::cairo_deserialize(felts, offset)?;
        let high = u128::cairo_deserialize(felts, offset + u128::cairo_serialized_size(&low))?;
        Ok(U256 { low, high })
    }
}
// Implementing From<(FieldElement, FieldElement)> for U256 as big endian
impl From<(FieldElement, FieldElement)> for U256 {
    fn from(item: (FieldElement, FieldElement)) -> Self {
        let high = item
            .0
            .to_string()
            .parse::<u128>()
            .expect("Failed to parse FieldElement to u128");
        let low = item
            .1
            .to_string()
            .parse::<u128>()
            .expect("Failed to parse FieldElement to u128");
        U256 { high, low }
    }
}

impl U256 {
    pub fn to_bytes_be(&self) -> [u8; 32] {
        let mut bytes = [0; 32];
        bytes[0..16].copy_from_slice(&self.high.to_be_bytes());
        bytes[16..32].copy_from_slice(&self.low.to_be_bytes());
        bytes
    }
    pub fn to_bytes_le(&self) -> [u8; 32] {
        let mut bytes = [0; 32];
        bytes[0..16].copy_from_slice(&self.low.to_le_bytes());
        bytes[16..32].copy_from_slice(&self.high.to_le_bytes());
        bytes
    }
    pub fn from_bytes_be(bytes: &[u8; 32]) -> Self {
        let high = u128::from_be_bytes(bytes[0..16].try_into().unwrap());
        let low = u128::from_be_bytes(bytes[16..32].try_into().unwrap());
        U256 { low, high }
    }
    pub fn from_bytes_le(bytes: &[u8; 32]) -> Self {
        let low = u128::from_le_bytes(bytes[0..16].try_into().unwrap());
        let high = u128::from_le_bytes(bytes[16..32].try_into().unwrap());
        U256 { low, high }
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
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, // high
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, // low
        ];
        assert_eq!(bytes, expected_bytes);
    }
    #[test]
    fn test_to_bytes_le() {
        let u256 = U256 {
            low: 9_u128,
            high: 8_u128,
        };
        let bytes = u256.to_bytes_le();
        let expected_bytes: [u8; 32] = [
            9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // low
            8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // high
        ];
        assert_eq!(bytes, expected_bytes);
    }
    #[test]
    fn test_from_bytes_be() {
        let bytes: [u8; 32] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, // high
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, // low
        ];
        let u256 = U256::from_bytes_be(&bytes);
        assert_eq!(u256.low, 9_u128);
        assert_eq!(u256.high, 8_u128);
    }
    #[test]
    fn test_from_bytes_le() {
        let bytes: [u8; 32] = [
            9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // low
            8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // high
        ];
        let u256 = U256::from_bytes_le(&bytes);
        assert_eq!(u256.low, 9_u128);
        assert_eq!(u256.high, 8_u128);
    }
    #[test]
    fn test_from_field_element() {
        let felts = (FieldElement::from(9_u128), FieldElement::from(8_u128));
        let u256 = U256::from(felts);
        assert_eq!(u256.low, 8_u128);
        assert_eq!(u256.high, 9_u128);
    }
}
