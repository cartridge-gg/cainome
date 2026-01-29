use cainome_cairo_serde::CairoSerde;
use cainome_cairo_serde_derive::CairoSerde;
use serde::Serialize;
use starknet::macros::felt;
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

#[derive(Debug, CairoSerde, PartialEq)]
struct SimpleTypes {
    x: u32,
    y: u64,
    a: i32,
    b: i64,
    c: bool,
}

#[tokio::test]
async fn main() {
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

    assert_eq!(
        s,
        r#"[{"x":"0x1","y":{"x":["0x2","0x3"],"y":"0x4","z":"0x6712bd50"}},["0x1"]]"#
    );

    let example = ExampleEnum::Struct {
        x: tuple,
        y: ExampleSimple {
            x: vec![Felt::from(5), Felt::from(6)],
            y: Felt::from(7),
            z: 1729281360,
        },
    };

    let serialized = ExampleEnum::cairo_serialize(&example);
    assert_eq!(
        serialized,
        vec![
            felt!("0x3"),
            felt!("0x1"),
            felt!("0x2"),
            felt!("0x2"),
            felt!("0x3"),
            felt!("0x4"),
            felt!("0x6712bd50"),
            felt!("0x1"),
            felt!("0x1"),
            felt!("0x2"),
            felt!("0x5"),
            felt!("0x6"),
            felt!("0x7"),
            felt!("0x6712bd50")
        ]
    );

    let deserialized = ExampleEnum::cairo_deserialize(&serialized, 0).unwrap();

    assert_eq!(deserialized, example);

    let simple = SimpleTypes {
        x: 1,
        y: 2,
        a: 3,
        b: 4,
        c: true,
    };

    let serialized = SimpleTypes::cairo_serialize(&simple);
    assert_eq!(
        serialized,
        vec![
            felt!("0x1"),
            felt!("0x2"),
            felt!("0x3"),
            felt!("0x4"),
            felt!("0x1")
        ]
    );

    let deserialized = SimpleTypes::cairo_deserialize(&serialized, 0).unwrap();
    assert_eq!(
        deserialized,
        SimpleTypes {
            x: 1,
            y: 2,
            a: 3,
            b: 4,
            c: true
        }
    );
}
