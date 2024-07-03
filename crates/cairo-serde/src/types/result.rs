//! CairoSerde implementation for Result.
//!
//! <https://github.com/starkware-libs/cairo/blob/main/corelib/src/result.cairo#L6>
use crate::{CairoSerde, Error as CairoError, Result as CairoResult};
use starknet::core::types::Felt;

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

    fn cairo_serialize(rust: &Self::RustType) -> Vec<Felt> {
        let mut out = vec![];

        match rust {
            Result::Ok(r) => {
                out.push(Felt::ZERO);
                out.extend(T::cairo_serialize(r));
            }
            Result::Err(e) => {
                out.push(Felt::ONE);
                out.extend(E::cairo_serialize(e));
            }
        };

        out
    }

    fn cairo_deserialize(felts: &[Felt], offset: usize) -> CairoResult<Self::RustType> {
        if offset >= felts.len() {
            return Err(CairoError::Deserialize(format!(
                "Buffer too short to deserialize a Result: offset ({}) : buffer {:?}",
                offset, felts,
            )));
        }

        let idx = felts[offset];

        if idx == Felt::ZERO {
            // + 1 as the offset value is the index of the enum.
            CairoResult::Ok(Ok(T::cairo_deserialize(felts, offset + 1)?))
        } else if idx == Felt::ONE {
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
    use starknet::core::types::Felt;

    #[test]
    fn test_result_ok_cairo_serialize() {
        let r = Ok(u32::MAX);
        let felts = Result::<u32, Felt>::cairo_serialize(&r);
        assert_eq!(felts.len(), 2);
        assert_eq!(felts[0], Felt::ZERO);
        assert_eq!(felts[1], Felt::from(u32::MAX));
    }

    #[test]
    fn test_result_ok_cairo_deserialize() {
        let felts = vec![Felt::ZERO, Felt::from(u32::MAX)];
        let r = Result::<u32, Felt>::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(r, Ok(u32::MAX));
    }

    #[test]
    fn test_result_ok_unit_cairo_serialize() {
        let r = Ok(());
        let felts = Result::<(), Felt>::cairo_serialize(&r);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::ZERO);
    }

    #[test]
    fn test_result_ok_unit_cairo_deserialize() {
        let felts = vec![Felt::ZERO];
        let r = Result::<(), Felt>::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn test_result_err_cairo_serialize() {
        let r = Err(Felt::ONE);
        let felts = Result::<Felt, Felt>::cairo_serialize(&r);
        assert_eq!(felts.len(), 2);
        assert_eq!(felts[0], Felt::ONE);
        assert_eq!(felts[1], Felt::ONE);
    }

    #[test]
    fn test_result_err_cairo_deserialize() {
        let felts = vec![Felt::ONE, Felt::ONE];
        let r = Result::<Felt, Felt>::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(r, Err(Felt::ONE));
    }
}
