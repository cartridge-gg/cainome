use katana_runner::RunnerCtx;
use starknet_types_core::felt::Felt;

use cainome_cairo_serde::ByteArray;
use cainome_rs_macro::abigen;

abigen!(
    MyContract,
    "crates/integration-tests/src/bindings/byte_array.json",
    cainome_serde_path("cainome_cairo_serde")
);

const UDC_ADDRESS: Felt =
    Felt::from_hex_unwrap("0x41a78e741e5af2fec34b695679bc6891742439f7afb8484ecd7766661ad02bf");

#[tokio::main]
#[katana_runner::test(accounts = 2, fee = false, block_time = 1)]
async fn test_byte_array(runner: &RunnerCtx) {
    let path = std::path::Path::new("./src/bindings/byte_array.json");
    let account = runner.account(0);

    let class_hash = MyContract::declare(&path, &account).await.unwrap();

    runner.dev_client().generate_block().await.unwrap();

    let contract = MyContract::deploy(UDC_ADDRESS, account, class_hash, vec![])
        .await
        .unwrap();

    let actual_byte_array = contract
        .get_byte_array()
        .call()
        .await
        .expect("Call to `get_byte_array` failed");

    assert_eq!(
        actual_byte_array,
        ByteArray::from_string("cainome test a bit long to fit into a felt252").unwrap()
    );

    let string: String = actual_byte_array.to_string().unwrap();
    assert_eq!(string, "cainome test a bit long to fit into a felt252");

    let expected_byte_array =
        ByteArray::from_string("super long string that does not fit into a felt252").unwrap();

    let _tx_res = contract
        .set_byte_array(&expected_byte_array)
        .send()
        .await
        .expect("Call to `set_a` failed");

    runner.dev_client().generate_block().await.unwrap();

    let actual_byte_array = contract
        .get_byte_array_storage()
        .call()
        .await
        .expect("Call to `get_byte_array_storage` failed");

    assert_eq!(actual_byte_array, expected_byte_array);
}
