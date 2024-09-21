use cainome_cairo_serde::CairoSerde;
use cairo_serde_derive::CairoSerde;
use starknet_types_core::felt::Felt;

#[derive(Debug, CairoSerde)]
struct Example {
    x: Felt,
    y: Example2,
}

#[derive(Debug, CairoSerde)]
struct Example2 {
    x: Vec<Felt>,
    y: Felt,
}

fn main() {
    let example = Example {
        x: Felt::from(1),
        y: Example2 {
            x: vec![Felt::from(2), Felt::from(3)],
            y: Felt::from(4),
        },
    };

    let serialized = Example::cairo_serialize(&example);
    println!("serialized = {:?}", serialized);

    let deserialized = Example::cairo_deserialize(&serialized, 0);
    println!("deserialized = {:?}", deserialized);
}
