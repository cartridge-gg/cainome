use cainome_rs_macro::abigen;

abigen!(
    MyContract,
    "contracts/abi/simple_types.abi.json",
    cainome_serde_path("cainome_cairo_serde")
);

#[tokio::main]
async fn main() {}
