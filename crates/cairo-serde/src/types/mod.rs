pub mod array;
pub mod array_legacy;
pub mod boolean;
pub mod byte_array;
pub mod felt;
pub mod integers;
pub mod non_zero;
pub mod option;
pub mod result;
pub mod starknet;
pub mod tuple;
pub mod u256;

#[cfg(test)]
mod tests {
    use crate::CairoSerde;
    use ::starknet::core::types::Felt;

    #[test]
    fn test_serialize_several_values() {
        let o = Some(u32::MAX);
        let r = Err(Felt::TWO);
        let a: Vec<u64> = vec![1, 2, 3];

        let mut felts = vec![];
        felts.extend(Option::<u32>::cairo_serialize(&o));
        felts.extend(Result::<u64, Felt>::cairo_serialize(&r));
        felts.extend(Vec::<u64>::cairo_serialize(&a));

        assert_eq!(felts.len(), 8);
        assert_eq!(felts[0], Felt::ZERO);
        assert_eq!(felts[1], Felt::from(u32::MAX));
        assert_eq!(felts[2], Felt::ONE);
        assert_eq!(felts[3], Felt::TWO);
        assert_eq!(felts[4], Felt::THREE);
        assert_eq!(felts[5], Felt::ONE);
        assert_eq!(felts[6], Felt::TWO);
        assert_eq!(felts[7], Felt::THREE);
    }

    #[test]
    fn test_serialize_several_values_with_unit() {
        let o = Some(u32::MAX);
        let r = Ok(());
        let a: Vec<u64> = vec![1, 2, 3];

        let mut felts = vec![];
        felts.extend(Option::<u32>::cairo_serialize(&o));
        felts.extend(Result::<(), Felt>::cairo_serialize(&r));
        felts.extend(Vec::<u64>::cairo_serialize(&a));

        assert_eq!(felts.len(), 7);
        assert_eq!(felts[0], Felt::ZERO);
        assert_eq!(felts[1], Felt::from(u32::MAX));
        assert_eq!(felts[2], Felt::ZERO);
        assert_eq!(felts[3], Felt::THREE);
        assert_eq!(felts[4], Felt::ONE);
        assert_eq!(felts[5], Felt::TWO);
        assert_eq!(felts[6], Felt::THREE);
    }

    #[test]
    fn test_deserialize_several_values() {
        let felts = vec![
            Felt::ZERO,
            Felt::from(u32::MAX),
            Felt::ONE,
            Felt::TWO,
            Felt::THREE,
            Felt::ONE,
            Felt::TWO,
            Felt::THREE,
        ];

        let mut offset = 0;

        let o = Option::<u32>::cairo_deserialize(&felts, offset).unwrap();
        offset += Option::<u32>::cairo_serialized_size(&o);

        let r = Result::<u64, Felt>::cairo_deserialize(&felts, offset).unwrap();
        offset += Result::<u64, Felt>::cairo_serialized_size(&r);

        let a = Vec::<u64>::cairo_deserialize(&felts, offset).unwrap();
        offset += Vec::<u64>::cairo_serialized_size(&a);

        assert_eq!(o, Some(u32::MAX));
        assert_eq!(r, Err(Felt::TWO));
        assert_eq!(a, vec![1, 2, 3]);
        assert_eq!(offset, felts.len());
    }

    #[test]
    fn test_deserialize_several_values_with_unit() {
        let felts = vec![
            Felt::ZERO,
            Felt::from(u32::MAX),
            Felt::ZERO,
            Felt::THREE,
            Felt::ONE,
            Felt::TWO,
            Felt::THREE,
        ];

        let mut offset = 0;

        let o = Option::<u32>::cairo_deserialize(&felts, offset).unwrap();
        offset += Option::<u32>::cairo_serialized_size(&o);

        let r = Result::<(), Felt>::cairo_deserialize(&felts, offset).unwrap();
        offset += Result::<(), Felt>::cairo_serialized_size(&r);

        let a = Vec::<u64>::cairo_deserialize(&felts, offset).unwrap();
        offset += Vec::<u64>::cairo_serialized_size(&a);

        assert_eq!(o, Some(u32::MAX));
        assert_eq!(r, Ok(()));
        assert_eq!(a, vec![1, 2, 3]);
        assert_eq!(offset, felts.len());
    }
}
