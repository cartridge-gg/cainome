use cainome_cairo_serde_derive::CairoSerde;
use starknet::core::types::Felt;

// This example uses an ABI where components introduce several enums with `Event` type name.
// This showcase how the type_aliases parameter can be leveraged to avoid conflicts.
#[derive(CairoSerde, serde::Serialize, serde::Deserialize)]
pub struct GenericOneBis {
    pub f1: Felt,
}

#[derive(CairoSerde, serde::Serialize, serde::Deserialize)]
pub struct GenericTwoBis {
    pub a: Felt,
}

#[derive(CairoSerde, serde::Serialize, serde::Deserialize)]
pub struct MyDef {
    pub a: Felt,
}

#[allow(unused)]
#[tokio::main]
async fn main() {}
