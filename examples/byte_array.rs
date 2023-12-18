use cainome::cairo_serde::ByteArray;
use cainome::rs::abigen;
use starknet::{
    accounts::{ExecutionEncoding, SingleOwnerAccount},
    core::types::FieldElement,
    providers::{jsonrpc::HttpTransport, AnyProvider, JsonRpcClient},
    signers::{LocalWallet, SigningKey},
};
use std::sync::Arc;
use url::Url;

abigen!(MyContract, "./contracts/abi/byte_array.abi.json",);

const CONTRACT_ADDRESS: &str = "0x0722c97752880529933e733e9a7d725685000b3989489d8fbbe7a247d4823921";
const KATANA_ACCOUNT_0: &str = "0x517ececd29116499f4a1b64b094da79ba08dfd54a3edaa316134c41f8160973";
const KATANA_PRIVKEY_0: &str = "0x1800000000300000180000000000030000000000003006001800006600";
const KATANA_CHAIN_ID: &str = "0x4b4154414e41";

#[tokio::main]
async fn main() {
    let rpc_url = Url::parse("http://0.0.0.0:5050").expect("Expecting Starknet RPC URL");
    let provider =
        AnyProvider::JsonRpcHttp(JsonRpcClient::new(HttpTransport::new(rpc_url.clone())));

    let contract_address = FieldElement::from_hex_be(CONTRACT_ADDRESS).unwrap();

    let signer = LocalWallet::from(SigningKey::from_secret_scalar(
        FieldElement::from_hex_be(KATANA_PRIVKEY_0).unwrap(),
    ));
    let address = FieldElement::from_hex_be(KATANA_ACCOUNT_0).unwrap();

    let account = Arc::new(SingleOwnerAccount::new(
        provider,
        signer,
        address,
        FieldElement::from_hex_be(KATANA_CHAIN_ID).unwrap(),
        ExecutionEncoding::Legacy,
    ));

    let contract = MyContract::new(contract_address, account);

    let byte_array =
        ByteArray::from_string("super long string that does not fit into a felt252").unwrap();

    let _tx_res = contract
        .set_byte_array(&byte_array)
        .send()
        .await
        .expect("Call to `set_a` failed");

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let byte_array = contract
        .get_byte_array()
        .call()
        .await
        .expect("Call to `get_byte_array` failed");

    println!("byte_array: {:?}", byte_array);

    let string: String = byte_array.to_string().unwrap();
    println!("byte_array str: {}", string);

    let byte_array = contract
        .get_byte_array_storage()
        .call()
        .await
        .expect("Call to `get_byte_array_storage` failed");

    println!("byte_array: {:?}", byte_array);

    let string: String = byte_array.to_string().unwrap();
    println!("byte_array str: {}", string);
}
