//! CairoSerde implementation for Result.
//!
//! <https://github.com/starkware-libs/cairo/blob/main/corelib/src/result.cairo#L6>
use crate::{CairoSerde, Error as CairoError, Result as CairoResult};
use starknet::core::types::FieldElement;

impl<T, RT, E, RE> CairoSerde for Result<T, E>
where
    T: CairoSerde<RustType = RT>,
    E: CairoSerde<RustType = RE>,
{
    type RustType = Result<RT, RE>;

    #[inline]
    fn cairo_serialized_size(rust: &Self::RustType) -> usize {
        match rust {
            Ok(d) => 1 + T::cairo_serialized_size(d),
            Err(e) => 1 + E::cairo_serialized_size(e),
        }
    }

    fn cairo_serialize(rust: &Self::RustType) -> Vec<FieldElement> {
        let mut out = vec![];

        match rust {
            Result::Ok(r) => {
                out.push(FieldElement::ZERO);
                out.extend(T::cairo_serialize(r));
            }
            Result::Err(e) => {
                out.push(FieldElement::ONE);
                out.extend(E::cairo_serialize(e));
            }
        };

        out
    }

    fn cairo_deserialize(felts: &[FieldElement], offset: usize) -> CairoResult<Self::RustType> {
        if offset >= felts.len() {
            return Err(CairoError::Deserialize(format!(
                "Buffer too short to deserialize a Result: offset ({}) : buffer {:?}",
                offset, felts,
            )));
        }

        let idx = felts[offset];

        if idx == FieldElement::ZERO {
            // + 1 as the offset value is the index of the enum.
            CairoResult::Ok(Ok(T::cairo_deserialize(felts, offset + 1)?))
        } else if idx == FieldElement::ONE {
            CairoResult::Ok(Err(E::cairo_deserialize(felts, offset + 1)?))
        } else {
            Err(CairoError::Deserialize(
                "Result is expected 0 or 1 index only".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starknet::core::types::FieldElement;

    #[test]
    fn test_result_ok_cairo_serialize() {
        let r = Ok(u32::MAX);
        let felts = Result::<u32, FieldElement>::cairo_serialize(&r);
        assert_eq!(felts.len(), 2);
        assert_eq!(felts[0], FieldElement::ZERO);
        assert_eq!(felts[1], FieldElement::from(u32::MAX));
    }

    #[test]
    fn test_result_ok_cairo_deserialize() {
        let felts = vec![FieldElement::ZERO, FieldElement::from(u32::MAX)];
        let r = Result::<u32, FieldElement>::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(r, Ok(u32::MAX));
    }

    #[test]
    fn test_result_ok_unit_cairo_serialize() {
        let r = Ok(());
        let felts = Result::<(), FieldElement>::cairo_serialize(&r);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], FieldElement::ZERO);
    }

    #[test]
    fn test_result_ok_unit_cairo_deserialize() {
        let felts = vec![FieldElement::ZERO];
        let r = Result::<(), FieldElement>::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn test_result_err_cairo_serialize() {
        let r = Err(FieldElement::ONE);
        let felts = Result::<FieldElement, FieldElement>::cairo_serialize(&r);
        assert_eq!(felts.len(), 2);
        assert_eq!(felts[0], FieldElement::ONE);
        assert_eq!(felts[1], FieldElement::ONE);
    }

    #[test]
    fn test_result_err_cairo_deserialize() {
        let felts = vec![FieldElement::ONE, FieldElement::ONE];
        let r = Result::<FieldElement, FieldElement>::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(r, Err(FieldElement::ONE));
    }
}
