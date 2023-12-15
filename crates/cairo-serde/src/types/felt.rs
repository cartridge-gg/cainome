use crate::{CairoSerde, Error, Result};
use starknet::core::types::FieldElement;

impl CairoSerde for FieldElement {
    type RustType = Self;

    fn cairo_serialize(rust: &Self::RustType) -> Vec<FieldElement> {
        vec![*rust]
    }

    fn cairo_deserialize(felts: &[FieldElement], offset: usize) -> Result<Self::RustType> {
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
        let f = FieldElement::ZERO;
        let felts = FieldElement::cairo_serialize(&f);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], FieldElement::ZERO);
    }

    #[test]
    fn test_deserialize_field_element() {
        let felts = vec![FieldElement::ZERO, FieldElement::ONE, FieldElement::TWO];
        assert_eq!(
            FieldElement::cairo_deserialize(&felts, 0).unwrap(),
            FieldElement::ZERO
        );
        assert_eq!(
            FieldElement::cairo_deserialize(&felts, 1).unwrap(),
            FieldElement::ONE
        );
        assert_eq!(
            FieldElement::cairo_deserialize(&felts, 2).unwrap(),
            FieldElement::TWO
        );
    }
}
