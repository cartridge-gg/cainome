//! CairoSerde implementation for tuples.
use crate::{CairoSerde, Result};
use starknet::core::types::Felt;

impl CairoSerde for () {
    type RustType = Self;

    #[inline]
    fn cairo_serialized_size(_rust: &Self::RustType) -> usize {
        0
    }

    fn cairo_serialize(_rust: &Self::RustType) -> Vec<Felt> {
        vec![]
    }

    fn cairo_deserialize(_felts: &[Felt], _offset: usize) -> Result<Self::RustType> {
        Ok(())
    }
}

macro_rules! impl_tuples {
    ($num:expr, $( $ty:ident : $rt:ident : $var:ident : $no:tt ),+ $(,)?) => {
        impl<$( $ty, $rt ),+> CairoSerde for ($( $ty, )+)
        where
            $($ty: CairoSerde<RustType = $rt>,)+
        {
            type RustType = ($( $rt ),*);

            const SERIALIZED_SIZE: Option<usize> = None;

            #[inline]
            fn cairo_serialized_size(rust: &Self::RustType) -> usize {
                let mut size = 0;
                $(
                    size += $ty::cairo_serialized_size(& rust.$no);
                )*

                size
            }

            fn cairo_serialize(rust: &Self::RustType) -> Vec<Felt> {
                let mut out: Vec<Felt> = vec![];

                $( out.extend($ty::cairo_serialize(& rust.$no)); )*

                out
            }

            fn cairo_deserialize(felts: &[Felt], offset: usize) -> Result<Self::RustType> {
                let mut offset = offset;

                $(
                    let $var : $rt = $ty::cairo_deserialize(felts, offset)?;
                    offset += $ty::cairo_serialized_size(& $var);
                )*

                // Remove warning.
                let _offset = offset;

                Ok(($( $var ),*))
            }
        }
    }
}

impl_tuples!(2, A:RA:r0:0, B:RB:r1:1);
impl_tuples!(3, A:RA:r0:0, B:RB:r1:1, C:RC:r2:2);
impl_tuples!(4, A:RA:r0:0, B:RB:r1:1, C:RC:r2:2, D:RD:r3:3);
impl_tuples!(5, A:RA:r0:0, B:RB:r1:1, C:RC:r2:2, D:RD:r3:3, E:RE:r4:4);

#[cfg(test)]
mod tests {
    use starknet::core::types::Felt;

    use super::*;

    #[test]
    fn test_serialize_tuple2() {
        let v = (Felt::ONE, 128_u32);
        let felts = <(Felt, u32)>::cairo_serialize(&v);
        assert_eq!(felts.len(), 2);
        assert_eq!(felts[0], Felt::ONE);
        assert_eq!(felts[1], Felt::from(128_u32));
    }

    #[test]
    fn test_deserialize_tuple2() {
        let felts = vec![Felt::THREE, 99_u32.into()];
        let vals = <(Felt, u32)>::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(vals.0, Felt::THREE);
        assert_eq!(vals.1, 99_u32);
    }

    #[test]
    fn test_serialize_tuple2_array() {
        let v = (vec![Felt::ONE], 128_u32);
        let felts = <(Vec<Felt>, u32)>::cairo_serialize(&v);
        assert_eq!(felts.len(), 3);
        assert_eq!(felts[0], Felt::ONE);
        assert_eq!(felts[1], Felt::ONE);
        assert_eq!(felts[2], Felt::from(128_u32));
    }

    #[test]
    fn test_deserialize_tuple2_array() {
        let felts = vec![Felt::ONE, Felt::ONE, 99_u32.into()];
        let vals = <(Vec<Felt>, u32)>::cairo_deserialize(&felts, 0).unwrap();
        assert_eq!(vals.0, vec![Felt::ONE]);
        assert_eq!(vals.1, 99_u32);
    }
}
