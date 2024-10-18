use cainome_cairo_serde::CairoSerde;
use cainome_cairo_serde_derive::CairoSerde;
use serde::Serialize;
use starknet_types_core::felt::Felt;

#[derive(Debug, CairoSerde, PartialEq, Serialize)]
struct ExampleSimple {
    x: Vec<Felt>,
    y: Felt,
    #[serde(serialize_with = "cainome_cairo_serde::serialize_as_hex")]
    z: u128,
}

#[derive(Debug, CairoSerde, PartialEq, Serialize)]
struct ExampleNested {
    x: Felt,
    y: ExampleSimple,
}

#[derive(Debug, CairoSerde, PartialEq, Serialize)]
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
                z: 1729281360,
            },
        },
        vec![Felt::from(1)],
    );

    let s = serde_json::to_string(&tuple).unwrap();
    println!("s = {}", s);

    let example = ExampleEnum::Struct {
        x: tuple,
        y: ExampleSimple {
            x: vec![Felt::from(5), Felt::from(6)],
            y: Felt::from(7),
            z: 1729281360,
        },
    };

    let serialized = ExampleEnum::cairo_serialize(&example);
    println!("serialized = {:?}", serialized);

    let deserialized = ExampleEnum::cairo_deserialize(&serialized, 0).unwrap();

    assert_eq!(deserialized, example);

    println!("deserialized = {:?}", deserialized);
}
