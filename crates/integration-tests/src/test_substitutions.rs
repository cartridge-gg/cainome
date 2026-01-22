use cainome_cairo_serde_derive::CairoSerde;
use starknet::core::types::Felt;

// #[derive(CairoSerde)]
// pub struct GenericOne<T>
// where
//     T: cainome_cairo_serde::CairoSerde,
// {
//     pub a: Felt,
//     pub b: Felt,
//     pub c: T,
// }

// This example uses an ABI where components introduce several enums with `Event` type name.
// This showcase how the type_aliases parameter can be leveraged to avoid conflicts.
#[derive(CairoSerde, serde::Serialize, serde::Deserialize)]
pub struct GenericOneFelt {
    pub a: Felt,
    pub b: Felt,
    pub c: cainome_cairo_serde::U256,
}

#[derive(CairoSerde, serde::Serialize, serde::Deserialize)]
pub struct GenericOneu256 {
    pub a: cainome_cairo_serde::U256,
    pub b: Felt,
    pub c: cainome_cairo_serde::U256,
}

#[derive(CairoSerde, serde::Serialize, serde::Deserialize)]
pub struct GenericOneu64 {
    pub a: u64,
    pub b: Felt,
    pub c: cainome_cairo_serde::U256,
}

#[derive(CairoSerde, serde::Serialize, serde::Deserialize)]
pub struct GenericOneSpanFelt {
    pub a: Vec<starknet::core::types::Felt>,
    pub b: Felt,
    pub c: cainome_cairo_serde::U256,
}

#[derive(CairoSerde, serde::Serialize, serde::Deserialize)]
pub struct MyDef {
    pub a: Felt,
}

#[allow(unused)]
#[tokio::main]
async fn main() {}
