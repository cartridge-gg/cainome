use cainome::rs::abigen;
use cainome_cairo_serde_derive::CairoSerde;
use serde::Serialize;
use starknet::core::types::Felt;
// This example uses an ABI where components introduce several enums with `Event` type name.
// This showcase how the type_aliases parameter can be leveraged to avoid conflicts.
#[derive(CairoSerde, Serialize)]
pub struct GenericOneBis {
    pub f1: Felt,
}

#[derive(CairoSerde, Serialize)]
pub struct GenericTwoBis {
    pub a: Felt,
}

abigen!(
    MyContract,
    "./contracts/abi/structs.abi.json",
    type_aliases {
        contracts::abicov::structs::GenericOne as GenericOneBis;
        contracts::abicov::structs::GenericTwo as GenericTwoBis;
    },
    type_skips(
        contracts::abicov::structs::GenericOne, contracts::abicov::structs::GenericTwo
    ),
    derives(serde::Serialize)
);

#[tokio::main]
async fn main() {
    let _one = GenericOneBis { f1: Felt::from(1) };
    let _two = GenericTwoBis { a: Felt::from(2) };
}