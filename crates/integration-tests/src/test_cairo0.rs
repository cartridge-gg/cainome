#![allow(deprecated)]
use cainome_rs_macro::abigen_legacy;

abigen_legacy!(
    MyContract,
    "./contracts/cairo0/kkrt.abi.json",
    cainome_serde_path("cainome_cairo_serde")
);

#[tokio::test]
async fn test_cairo0_generation() {}
