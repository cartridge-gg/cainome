use cainome::rs::abigen_legacy;

abigen_legacy!(MyContract, "./contracts/abi/kkrt.abi.json",);

#[tokio::main]
async fn main() {}
