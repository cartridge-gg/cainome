#![allow(deprecated)]
use cainome::rs::abigen_legacy;

// From an extracted ABI.
//abigen_legacy!(MyContract, "./contracts/cairo0/oz0.abi.json",);

// From a LegacyContractClass extracting the ABI from it.
abigen_legacy!(MyContract, "./contracts/cairo0/kkrt_account_cairo0.json");

#[tokio::main]
async fn main() {}
