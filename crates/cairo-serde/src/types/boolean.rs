//! CairoSerde implementation for bool.
use crate::{CairoSerde, Error, Result};
use starknet::core::types::Felt;

impl CairoSerde for bool {
    type RustType = Self;

    fn cairo_serialize(rust: &Self::RustType) -> Vec<Felt> {
        vec![Felt::from(*rust as u32)]
    }

    fn cairo_deserialize(felts: &[Felt], offset: usize) -> Result<Self::RustType> {
        if offset >= felts.len() {
            return Err(Error::Deserialize(format!(
                "Buffer too short to deserialize a boolean: offset ({}) : buffer {:?}",
                offset, felts,
            )));
        }

        if felts[offset] == Felt::ONE {
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
        assert_eq!(felts[0], Felt::ONE);

        let v = false;
        let felts = bool::cairo_serialize(&v);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::ZERO);
    }

    #[test]
    fn test_deserialize_bool() {
        let felts = vec![Felt::ZERO, Felt::ONE, Felt::TWO];
        assert!(!bool::cairo_deserialize(&felts, 0).unwrap());
        assert!(bool::cairo_deserialize(&felts, 1).unwrap());
        assert!(!bool::cairo_deserialize(&felts, 2).unwrap());
    }
}
