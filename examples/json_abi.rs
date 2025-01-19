use cainome::rs::abigen;
use starknet::{
    macros::felt,
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
};
use url::Url;

abigen!(MyContractFile, "./contracts/abi/simple_types.abi.json",);

abigen!(MyContractEmbed, [
    {
    "type": "function",
    "name": "get_bool",
    "inputs": [],
    "outputs": [
      {
        "type": "core::bool"
      }
    ],
    "state_mutability": "view"
  },
  {
    "type": "function",
    "name": "set_bool",
    "inputs": [
      {
        "name": "v",
        "type": "core::bool"
      }
    ],
    "outputs": [],
    "state_mutability": "external"
  },
  {
    "type": "function",
    "name": "get_felt",
    "inputs": [],
    "outputs": [
      {
        "type": "core::felt252"
      }
    ],
    "state_mutability": "view"
  }
]);

#[tokio::main]
async fn main() {
    let url = Url::parse("http://localhost:5050").unwrap();
    let provider = JsonRpcClient::new(HttpTransport::new(url));

    let contract = MyContractEmbedReader::new(felt!("0x1337"), &provider);
    let _ = contract.get_bool().call().await.unwrap();
    let _ = contract.get_felt().call().await.unwrap();

    let contract = MyContractFileReader::new(felt!("0x1337"), &provider);
    let _ = contract.get_bool().call().await.unwrap();
    let _ = contract.get_felt().call().await.unwrap();
}
