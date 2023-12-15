//! CairoSerde implementation for bool.
use crate::{CairoSerde, Error, Result};
use starknet::core::types::FieldElement;

impl CairoSerde for bool {
    type RustType = Self;

    fn cairo_serialize(rust: &Self::RustType) -> Vec<FieldElement> {
        vec![FieldElement::from(*rust as u32)]
    }

    fn cairo_deserialize(felts: &[FieldElement], offset: usize) -> Result<Self::RustType> {
        if offset >= felts.len() {
            return Err(Error::Deserialize(format!(
                "Buffer too short to deserialize a boolean: offset ({}) : buffer {:?}",
                offset, felts,
            )));
        }

        if felts[offset] == FieldElement::ONE {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_bool() {
        let v = true;
        let felts = bool::cairo_serialize(&v);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], FieldElement::ONE);

        let v = false;
        let felts = bool::cairo_serialize(&v);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], FieldElement::ZERO);
    }

    #[test]
    fn test_deserialize_bool() {
        let felts = vec![FieldElement::ZERO, FieldElement::ONE, FieldElement::TWO];
        assert!(!bool::cairo_deserialize(&felts, 0).unwrap());
        assert!(bool::cairo_deserialize(&felts, 1).unwrap());
        assert!(!bool::cairo_deserialize(&felts, 2).unwrap());
    }
}
