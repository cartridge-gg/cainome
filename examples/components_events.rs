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

abigen!(
    MyContract,
    "../contracts/abi/components.abi.json",
    type_aliases {
        contracts::abicov::components::simple_component::Event as SimpleEvent;
        contracts::abicov::components::simple_component::Written as SimpleWritten;
        contracts::abicov::components::simple_component::MyStruct as MyStructSimple;
        contracts::abicov::components::simple_component_other::Event as OtherEvent;
        contracts::abicov::components::simple_component_other::Written as OtherWritten;
        contracts::abicov::components::simple_component_other::MyStruct as MyStructOther;
    }
);

// All components Events are renamed, and the only one that will remain with the name `Event`
// is the enum of the contract's events itself.

#[tokio::main]
async fn main() {}
