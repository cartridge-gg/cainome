use cainome::rs::abigen;
use starknet::{
    accounts::{ExecutionEncoding, SingleOwnerAccount},
    core::types::Felt,
    providers::{jsonrpc::HttpTransport, AnyProvider, JsonRpcClient},
    signers::{LocalWallet, SigningKey},
};
use std::sync::Arc;
use url::Url;

// To run this example, please first run `make setup_simple_get_set` in the contracts directory with a Katana running. This will declare and deploy the testing contract.

const CONTRACT_ADDRESS: &str = "0x007997dd654f2c079597a6c461489ee89981d0df733b8bcd3525153b0e700f98";
const KATANA_ACCOUNT_0: &str = "0x6162896d1d7ab204c7ccac6dd5f8e9e7c25ecd5ae4fcb4ad32e57786bb46e03";
const KATANA_PRIVKEY_0: &str = "0x1800000000300000180000000000030000000000003006001800006600";
const KATANA_CHAIN_ID: &str = "0x4b4154414e41";

// You can load of the sierra class entirely from the artifact.
// Or you can use the extracted abi entries with jq in contracts/abi/.
abigen!(
    MyContract,
    "./contracts/target/dev/contracts_simple_get_set.contract_class.json",
    execution_version("V3"),
);
//abigen!(MyContract, "./contracts/abi/simple_get_set.abi.json");

#[tokio::main]
async fn main() {
    let rpc_url = Url::parse("http://0.0.0.0:5050").expect("Expecting Starknet RPC URL");
    let provider =
        AnyProvider::JsonRpcHttp(JsonRpcClient::new(HttpTransport::new(rpc_url.clone())));

    let contract_address = Felt::from_hex(CONTRACT_ADDRESS).unwrap();

    // If you only plan to call views functions, you can use the `Reader`, which
    // only requires a provider along with your contract address.
    let contract = MyContractReader::new(contract_address, &provider);

    // To call a view, there is no need to initialize an account. You can directly
    // use the name of the method in the ABI and then use the `call()` method.
    let a = contract
        .get_a()
        .call()
        .await
        .expect("Call to `get_a` failed");
    println!("a initial value: {:?}", a);

    // If you want to do some invoke for external functions, you must use an account.
    let signer = LocalWallet::from(SigningKey::from_secret_scalar(
        Felt::from_hex(KATANA_PRIVKEY_0).unwrap(),
    ));
    let address = Felt::from_hex(KATANA_ACCOUNT_0).unwrap();

    let account = Arc::new(SingleOwnerAccount::new(
        provider,
        signer,
        address,
        Felt::from_hex(KATANA_CHAIN_ID).unwrap(),
        ExecutionEncoding::New,
    ));

    // A `Contract` exposes all the methods of the ABI, which includes the views (as the `ContractReader`) and
    // the externals (sending transaction).
    let contract = MyContract::new(contract_address, account);

    // The transaction is actually sent when `send()` is called.
    // You can before that configure the fees, or even only run an estimation of the
    // fees without actually sending the transaction.
    let _tx_res = contract
        .set_a(&(a + Felt::ONE))
        .gas_estimate_multiplier(1.2)
        .send()
        .await
        .expect("Call to `set_a` failed");

    // In production code, you want to poll the transaction status.
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let a = contract
        .get_a()
        .call()
        .await
        .expect("Call to `get_a` failed");
    println!("a after invoke: {:?}", a);
}
