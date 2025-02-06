use cainome::rs::abigen;
use starknet::{
    macros::felt,
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
};
use url::Url;

abigen!(MyContractEmbed, [
    {
    "type": "function",
    "name": "move",
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
    "name": "break",
    "inputs": [],
    "outputs": [
      {
        "type": "core::bool"
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
    let _ = contract.r#move().call().await.unwrap();
    let _ = contract.r#break().call().await.unwrap();
}
