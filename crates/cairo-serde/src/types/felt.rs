use crate::{CairoSerde, Error, Result};
use starknet::core::types::Felt;

impl CairoSerde for Felt {
    type RustType = Self;

    fn cairo_serialize(rust: &Self::RustType) -> Vec<Felt> {
        vec![*rust]
    }

    fn cairo_deserialize(felts: &[Felt], offset: usize) -> Result<Self::RustType> {
        if offset >= felts.len() {
            return Err(Error::Deserialize(format!(
                "Buffer too short to deserialize a felt: offset ({}) : buffer {:?}",
                offset, felts,
            )));
        }

        Ok(felts[offset])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_field_element() {
        let f = Felt::ZERO;
        let felts = Felt::cairo_serialize(&f);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::ZERO);
    }

    #[test]
    fn test_deserialize_field_element() {
        let felts = vec![Felt::ZERO, Felt::ONE, Felt::TWO];
        assert_eq!(Felt::cairo_deserialize(&felts, 0).unwrap(), Felt::ZERO);
        assert_eq!(Felt::cairo_deserialize(&felts, 1).unwrap(), Felt::ONE);
        assert_eq!(Felt::cairo_deserialize(&felts, 2).unwrap(), Felt::TWO);
    }
}
