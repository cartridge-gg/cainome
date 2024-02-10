use cainome::rs::abigen_legacy;

abigen_legacy!(MyContract, "./contracts/cairo0/oz0.abi.json",);

#[tokio::main]
async fn main() {}
