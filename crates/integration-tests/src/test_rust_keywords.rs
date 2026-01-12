use cainome_rs_macro::abigen;
use katana_runner::RunnerCtx;
use starknet_types_core::felt::Felt;

abigen!(
    MyContractEmbed,
    [{
        "type": "function",
        "name": "move",
        "inputs": [],
        "outputs": [],
        "state_mutability": "view"
    },
    {
        "type": "function",
        "name": "break",
        "inputs": [],
        "outputs": [],
        "state_mutability": "view"
    }],
    cainome_serde_path("cainome_cairo_serde")
);

#[tokio::main]
#[katana_runner::test(accounts = 2, fee = false, block_time = 1)]
#[ignore = "Can't deploy that contract"]
async fn test_rust_keyworkds_type_fields(runner: &RunnerCtx) {
    let provider = runner.provider();

    let contract = MyContractEmbedReader::new(Felt::from_hex("0x1337").unwrap(), &provider);
    contract.r#move().call().await.unwrap();
    contract.r#break().call().await.unwrap();
}
