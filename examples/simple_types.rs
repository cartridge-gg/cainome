use cainome::rs::abigen;

abigen!(MyContract, "./contracts/abi/simple_types.abi.json",);

#[tokio::main]
async fn main() {}
