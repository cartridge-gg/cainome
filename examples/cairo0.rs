#![allow(deprecated)]
use cainome::rs::abigen_legacy;

abigen_legacy!(MyContract, "./contracts/cairo0/kkrt.abi.json",);

#[tokio::main]
async fn main() {}
