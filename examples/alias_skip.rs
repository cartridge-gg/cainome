use cainome::rs::abigen;
use cainome_cairo_serde_derive::CairoSerde;
use serde::Serialize;
use starknet::{
    accounts::{Account, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount},
    core::types::{BlockId, BlockTag, Felt},
    providers::{jsonrpc::HttpTransport, AnyProvider, JsonRpcClient},
    signers::{LocalWallet, SigningKey},
};
// use std::sync::Arc;
// use url::Url;

// This example uses an ABI where components introduce several enums with `Event` type name.
// This showcase how the type_aliases parameter can be leveraged to avoid conflicts.

#[derive(CairoSerde)]
pub struct GenericOneBis {
    pub f1: Felt,
}

#[derive(CairoSerde)]
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
    type_skips(contracts::abicov::structs::GenericOne, contracts::abicov::structs::GenericTwo)
);

#[tokio::main]
async fn main() {

    let one = GenericOneBis {
        f1: 1,
    };

}
