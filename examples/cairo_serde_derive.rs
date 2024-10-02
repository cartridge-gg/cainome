use cainome_cairo_serde::CairoSerde;
use cainome_cairo_serde_derive::CairoSerde;
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

fn main() {
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

    let example = ExampleEnum::Struct {
        x: tuple,
        y: ExampleSimple {
            x: vec![Felt::from(5), Felt::from(6)],
            y: Felt::from(7),
        },
    };

    let serialized = ExampleEnum::cairo_serialize(&example);
    println!("serialized = {:?}", serialized);

    let deserialized = ExampleEnum::cairo_deserialize(&serialized, 0).unwrap();

    assert_eq!(deserialized, example);

    println!("deserialized = {:?}", deserialized);
}
