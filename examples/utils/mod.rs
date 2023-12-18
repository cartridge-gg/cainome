use starknet::{
    accounts::{Account, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount},
    core::types::{BlockId, BlockTag, FieldElement},
    providers::{jsonrpc::HttpTransport, AnyProvider, JsonRpcClient},
    signers::{LocalWallet, SigningKey},
};
use std::sync::Arc;
use url::Url;

const KATANA_ACCOUNT_0: &str = "0x517ececd29116499f4a1b64b094da79ba08dfd54a3edaa316134c41f8160973";
const KATANA_PRIVKEY_0: &str = "0x1800000000300000180000000000030000000000003006001800006600";
const KATANA_CHAIN_ID: &str = "0x4b4154414e41";

async fn setup_account() -> {
    let rpc_url = Url::parse("http://0.0.0.0:5050").expect("Expecting Starknet RPC URL");
    let provider =
        AnyProvider::JsonRpcHttp(JsonRpcClient::new(HttpTransport::new(rpc_url.clone())));

    let signer = LocalWallet::from(SigningKey::from_secret_scalar(
        FieldElement::from_hex_be(KATANA_PRIVKEY_0).unwrap(),
    ));
    let address = FieldElement::from_hex_be(KATANA_ACCOUNT_0).unwrap();

    Arc::new(SingleOwnerAccount::new(
        provider,
        signer,
        address,
        FieldElement::from_hex_be(KATANA_CHAIN_ID).unwrap(),
        ExecutionEncoding::Legacy,
    ))
}
