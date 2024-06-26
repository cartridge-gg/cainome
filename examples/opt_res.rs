use cainome::rs::abigen;
// use starknet::{
//     accounts::{Account, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount},
//     core::types::{BlockId, BlockTag, Felt},
//     providers::{jsonrpc::HttpTransport, AnyProvider, JsonRpcClient},
//     signers::{LocalWallet, SigningKey},
// };
// use std::sync::Arc;
// use url::Url;

// This example uses an ABI where components introduce several enums with `Event` type name.
// This showcase how the type_aliases parameter can be leveraged to avoid conflicts.

abigen!(MyContract, "./contracts/abi/option_result.abi.json",);

#[tokio::main]
async fn main() {}
