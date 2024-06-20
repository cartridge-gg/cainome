//! CairoSerde implementation for Option.
//!
//! In cairo, `Some` is the first field and `None` the second one.
//! To follow the serialization rule, `Some` has index 0, and `None` index 1.
//!
//! <https://github.com/starkware-libs/cairo/blob/main/corelib/src/option.cairo#L6>
use crate::{CairoSerde, Error, Result};
use starknet::core::types::Felt;

impl<T, RT> CairoSerde for Option<T>
where
    T: CairoSerde<RustType = RT>,
{
    type RustType = Option<RT>;

    #[inline]
    fn cairo_serialized_size(rust: &Self::RustType) -> usize {
        match rust {
            Some(d) => 1 + T::cairo_serialized_size(d),
            None => 1,
        }
    }

    fn cairo_serialize(rust: &Self::RustType) -> Vec<Felt> {
        let mut out = vec![];

        match rust {
            Some(r) => {
                out.push(Felt::ZERO);
                out.extend(T::cairo_serialize(r));
            }
            None => out.push(Felt::ONE),
        };

        out
    }

    fn cairo_deserialize(felts: &[Felt], offset: usize) -> Result<Self::RustType> {
        if offset >= felts.len() {
            return Err(Error::Deserialize(format!(
                "Buffer too short to deserialize an Option: offset ({}) : buffer {:?}",
                offset, felts,
            )));
        }

        let idx = felts[offset];

        if idx == Felt::ZERO {
            // + 1 as the offset value is the index of the enum.
            Ok(Option::Some(T::cairo_deserialize(felts, offset + 1)?))
        } else if idx == Felt::ONE {
            Ok(Option::None)
        } else {
            Err(Error::Deserialize(
                "Option is expected 0 or 1 index only".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starknet::core::types::Felt;

    #[test]
    fn test_option_some_cairo_serialize() {
        let o = Some(u32::MAX);
        let felts = Option::<u32>::cairo_serialize(&o);
        assert_eq!(felts.len(), 2);
        assert_eq!(felts[0], Felt::ZERO);
        assert_eq!(felts[1], Felt::from(u32::MAX));
    }

    #[test]
    fn test_option_some_cairo_deserialize() {
        let felts = vec![Felt::ZERO, Felt::from(u32::MAX)];
        let o = Option::<u32>::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(o, Some(u32::MAX));

        let felts = vec![Felt::THREE, Felt::ZERO, Felt::from(u32::MAX)];
        let o = Option::<u32>::cairo_deserialize(&felts, 1).unwrap();
        assert_eq!(o, Some(u32::MAX));
    }

    #[test]
    fn test_option_some_unit_cairo_serialize() {
        let o = Some(());
        let felts = Option::<()>::cairo_serialize(&o);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::ZERO);
    }

    #[test]
    fn test_option_some_unit_cairo_deserialize() {
        let felts = vec![Felt::ZERO];
        let o = Option::<()>::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(o, Some(()));
    }

    #[test]
    fn test_option_some_array_cairo_serialize() {
        let o = Some(vec![u32::MAX, u32::MAX]);
        let felts = Option::<Vec<u32>>::cairo_serialize(&o);
        assert_eq!(felts.len(), 4);
        assert_eq!(felts[0], Felt::ZERO);
        assert_eq!(felts[1], Felt::from(2_u32));
        assert_eq!(felts[2], Felt::from(u32::MAX));
        assert_eq!(felts[3], Felt::from(u32::MAX));
    }

    #[test]
    fn test_option_some_array_cairo_deserialize() {
        let felts = vec![
            Felt::ZERO,
            Felt::from(2_u32),
            Felt::from(u32::MAX),
            Felt::from(u32::MAX),
        ];
        let o = Option::<Vec<u32>>::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(o, Some(vec![u32::MAX, u32::MAX]));

        let felts = vec![
            Felt::THREE,
            Felt::ZERO,
            Felt::from(2_u32),
            Felt::from(u32::MAX),
            Felt::from(u32::MAX),
        ];
        let o = Option::<Vec<u32>>::cairo_deserialize(&felts, 1).unwrap();
        assert_eq!(o, Some(vec![u32::MAX, u32::MAX]));
    }

    #[test]
    fn test_option_none_cairo_serialize() {
        let o: Option<u32> = None;
        let felts = Option::<u32>::cairo_serialize(&o);
        assert_eq!(felts.len(), 1);
        assert_eq!(felts[0], Felt::ONE);
    }

    #[test]
    fn test_option_none_cairo_deserialize() {
        let felts = vec![Felt::ONE];
        let o = Option::<u32>::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(o, None);

        let felts = vec![Felt::THREE, Felt::ONE];
        let o = Option::<u32>::cairo_deserialize(&felts, 1).unwrap();
        assert_eq!(o, None);
    }
}
