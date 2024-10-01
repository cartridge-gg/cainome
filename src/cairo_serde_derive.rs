pub use cainome_cairo_serde_derive::*;

#[cfg(test)]
mod tests {
    use std::vec;

    use cainome_cairo_serde::CairoSerde;
    use cainome_cairo_serde_derive::CairoSerde;
    use starknet::macros::felt;
    use starknet_types_core::felt::Felt;

    #[derive(Debug, CairoSerde, PartialEq)]
    struct ExampleSimple {
        x: Vec<Felt>,
        y: Felt,
    }

    #[derive(Debug, CairoSerde, PartialEq)]
    struct ExampleNested {
        x: Felt,
        y: ExampleSimple,
    }

    #[derive(Debug, CairoSerde, PartialEq)]
    struct ExampleTuple(ExampleNested, Vec<Felt>);

    #[derive(Debug, CairoSerde, PartialEq)]
    enum ExampleEnum {
        None,
        One(ExampleTuple),
        Tuple(ExampleSimple, ExampleSimple),
        Struct { x: ExampleTuple, y: ExampleSimple },
    }
    #[test]
    fn test_derive_struct() {
        let tuple = ExampleTuple(
            ExampleNested {
                x: Felt::from(1),
                y: ExampleSimple {
                    x: vec![Felt::from(2), Felt::from(3)],
                    y: Felt::from(4),
                },
            },
            vec![Felt::from(1)],
        );

        let serialized = ExampleTuple::cairo_serialize(&tuple);

        assert_eq!(
            serialized,
            vec![
                felt!("1"),
                felt!("2"),
                felt!("2"),
                felt!("3"),
                felt!("4"),
                felt!("1"),
                felt!("1"),
            ]
        );

        let deserialized = ExampleTuple::cairo_deserialize(&serialized, 0).unwrap();

        assert_eq!(deserialized, tuple);
    }

    #[test]
    fn test_derive_enum() {
        let tuple = ExampleTuple(
            ExampleNested {
                x: Felt::from(1),
                y: ExampleSimple {
                    x: vec![Felt::from(2), Felt::from(3)],
                    y: Felt::from(4),
                },
            },
            vec![Felt::from(1)],
        );

        let enum_ = ExampleEnum::Struct {
            x: tuple,
            y: ExampleSimple {
                x: vec![Felt::from(2), Felt::from(3)],
                y: Felt::from(4),
            },
        };

        let serialized = ExampleEnum::cairo_serialize(&enum_);

        assert_eq!(
            serialized,
            vec![
                felt!("3"),
                felt!("1"),
                felt!("2"),
                felt!("2"),
                felt!("3"),
                felt!("4"),
                felt!("1"),
                felt!("1"),
                felt!("2"),
                felt!("2"),
                felt!("3"),
                felt!("4"),
            ]
        );

        let deserialized = ExampleEnum::cairo_deserialize(&serialized, 0).unwrap();

        assert_eq!(deserialized, enum_);
    }

    #[derive(Debug, CairoSerde, PartialEq)]
    enum CountEnum {
        Zero,
        One,
        Two,
        Three,
        Four,
        Five,
    }

    #[test]
    fn test_derive_enum_variants() {
        assert_eq!(
            CountEnum::cairo_serialize(&CountEnum::Zero),
            vec![felt!("0")]
        );
        assert_eq!(
            CountEnum::cairo_serialize(&CountEnum::One),
            vec![felt!("1")]
        );
        assert_eq!(
            CountEnum::cairo_serialize(&CountEnum::Two),
            vec![felt!("2")]
        );
        assert_eq!(
            CountEnum::cairo_serialize(&CountEnum::Three),
            vec![felt!("3")]
        );
        assert_eq!(
            CountEnum::cairo_serialize(&CountEnum::Four),
            vec![felt!("4")]
        );
        assert_eq!(
            CountEnum::cairo_serialize(&CountEnum::Five),
            vec![felt!("5")]
        );

        assert_eq!(
            CountEnum::cairo_deserialize(&[felt!("0")], 0).unwrap(),
            CountEnum::Zero
        );
        assert_eq!(
            CountEnum::cairo_deserialize(&[felt!("1")], 0).unwrap(),
            CountEnum::One
        );
        assert_eq!(
            CountEnum::cairo_deserialize(&[felt!("2")], 0).unwrap(),
            CountEnum::Two
        );
        assert_eq!(
            CountEnum::cairo_deserialize(&[felt!("3")], 0).unwrap(),
            CountEnum::Three
        );
        assert_eq!(
            CountEnum::cairo_deserialize(&[felt!("4")], 0).unwrap(),
            CountEnum::Four
        );
        assert_eq!(
            CountEnum::cairo_deserialize(&[felt!("5")], 0).unwrap(),
            CountEnum::Five
        );
    }
}
