pub mod array;
pub mod array_legacy;
pub mod boolean;
pub mod byte_array;
pub mod felt;
pub mod integers;
pub mod option;
pub mod result;
pub mod starknet;
pub mod tuple;

#[cfg(test)]
mod tests {
    use crate::CairoSerde;
    use ::starknet::core::types::FieldElement;

    #[test]
    fn test_serialize_several_values() {
        let o = Some(u32::MAX);
        let r = Err(FieldElement::TWO);
        let a: Vec<u64> = vec![1, 2, 3];

        let mut felts = vec![];
        felts.extend(Option::<u32>::cairo_serialize(&o));
        felts.extend(Result::<u64, FieldElement>::cairo_serialize(&r));
        felts.extend(Vec::<u64>::cairo_serialize(&a));

        assert_eq!(felts.len(), 8);
        assert_eq!(felts[0], FieldElement::ZERO);
        assert_eq!(felts[1], FieldElement::from(u32::MAX));
        assert_eq!(felts[2], FieldElement::ONE);
        assert_eq!(felts[3], FieldElement::TWO);
        assert_eq!(felts[4], FieldElement::THREE);
        assert_eq!(felts[5], FieldElement::ONE);
        assert_eq!(felts[6], FieldElement::TWO);
        assert_eq!(felts[7], FieldElement::THREE);
    }

    #[test]
    fn test_serialize_several_values_with_unit() {
        let o = Some(u32::MAX);
        let r = Ok(());
        let a: Vec<u64> = vec![1, 2, 3];

        let mut felts = vec![];
        felts.extend(Option::<u32>::cairo_serialize(&o));
        felts.extend(Result::<(), FieldElement>::cairo_serialize(&r));
        felts.extend(Vec::<u64>::cairo_serialize(&a));

        assert_eq!(felts.len(), 7);
        assert_eq!(felts[0], FieldElement::ZERO);
        assert_eq!(felts[1], FieldElement::from(u32::MAX));
        assert_eq!(felts[2], FieldElement::ZERO);
        assert_eq!(felts[3], FieldElement::THREE);
        assert_eq!(felts[4], FieldElement::ONE);
        assert_eq!(felts[5], FieldElement::TWO);
        assert_eq!(felts[6], FieldElement::THREE);
    }

    #[test]
    fn test_deserialize_several_values() {
        let felts = vec![
            FieldElement::ZERO,
            FieldElement::from(u32::MAX),
            FieldElement::ONE,
            FieldElement::TWO,
            FieldElement::THREE,
            FieldElement::ONE,
            FieldElement::TWO,
            FieldElement::THREE,
        ];

        let mut offset = 0;

        let o = Option::<u32>::cairo_deserialize(&felts, offset).unwrap();
        offset += Option::<u32>::cairo_serialized_size(&o);

        let r = Result::<u64, FieldElement>::cairo_deserialize(&felts, offset).unwrap();
        offset += Result::<u64, FieldElement>::cairo_serialized_size(&r);

        let a = Vec::<u64>::cairo_deserialize(&felts, offset).unwrap();
        offset += Vec::<u64>::cairo_serialized_size(&a);

        assert_eq!(o, Some(u32::MAX));
        assert_eq!(r, Err(FieldElement::TWO));
        assert_eq!(a, vec![1, 2, 3]);
        assert_eq!(offset, felts.len());
    }

    #[test]
    fn test_deserialize_several_values_with_unit() {
        let felts = vec![
            FieldElement::ZERO,
            FieldElement::from(u32::MAX),
            FieldElement::ZERO,
            FieldElement::THREE,
            FieldElement::ONE,
            FieldElement::TWO,
            FieldElement::THREE,
        ];

        let mut offset = 0;

        let o = Option::<u32>::cairo_deserialize(&felts, offset).unwrap();
        offset += Option::<u32>::cairo_serialized_size(&o);

        let r = Result::<(), FieldElement>::cairo_deserialize(&felts, offset).unwrap();
        offset += Result::<(), FieldElement>::cairo_serialized_size(&r);

        let a = Vec::<u64>::cairo_deserialize(&felts, offset).unwrap();
        offset += Vec::<u64>::cairo_serialized_size(&a);

        assert_eq!(o, Some(u32::MAX));
        assert_eq!(r, Ok(()));
        assert_eq!(a, vec![1, 2, 3]);
        assert_eq!(offset, felts.len());
    }
}
