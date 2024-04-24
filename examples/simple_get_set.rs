use cainome::rs::abigen;
use starknet::{
    accounts::{Account, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount},
    core::types::{BlockId, BlockTag, FieldElement},
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
    "./contracts/target/dev/contracts_simple_get_set.contract_class.json"
);
//abigen!(MyContract, "./contracts/abi/simple_get_set.abi.json");

#[tokio::main]
async fn main() {
    let rpc_url = Url::parse("http://0.0.0.0:5050").expect("Expecting Starknet RPC URL");
    let provider =
        AnyProvider::JsonRpcHttp(JsonRpcClient::new(HttpTransport::new(rpc_url.clone())));

    let contract_address = FieldElement::from_hex_be(CONTRACT_ADDRESS).unwrap();

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

    // If you need to explicitely set the block id of the call, you can do as
    // following. The default value is "Pending". Or you can initialize a `ContractReader`
    // using the `with_block_id` method, that will be applied to each call.
    let b = contract
        .get_b()
        .block_id(BlockId::Tag(BlockTag::Latest))
        .call()
        .await
        .expect("Call to `get_b` failed");
    println!("b inital value: {:?}", b);

    // For the inputs / outputs of the ABI functions, all the types are
    // defined where the abigen macro is expanded. Consider using the macro abigen
    // in a separate module to avoid clashes if you have to use it multiple times.

    // If you want to do some invoke for external functions, you must use an account.
    let signer = LocalWallet::from(SigningKey::from_secret_scalar(
        FieldElement::from_hex_be(KATANA_PRIVKEY_0).unwrap(),
    ));
    let address = FieldElement::from_hex_be(KATANA_ACCOUNT_0).unwrap();

    let account = Arc::new(SingleOwnerAccount::new(
        provider,
        signer,
        address,
        FieldElement::from_hex_be(KATANA_CHAIN_ID).unwrap(),
        ExecutionEncoding::New,
    ));

    // A `Contract` exposes all the methods of the ABI, which includes the views (as the `ContractReader`) and
    // the externals (sending transaction).
    let contract = MyContract::new(contract_address, account);

    // The transaction is actually sent when `send()` is called.
    // You can before that configure the fees, or even only run an estimation of the
    // fees without actually sending the transaction.
    let _tx_res = contract
        .set_a(&(a + FieldElement::ONE))
        .max_fee(1000000000000000_u128.into())
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

    // Now let's say we want to do multicall, and in one transaction we want to set a and b.
    // You can call the same function name with `_getcall` prefix to get the
    // call only, ready to be added in a multicall array.
    let set_a_call = contract.set_a_getcall(&FieldElement::from_hex_be("0xee").unwrap());
    let set_b_call = contract.set_b_getcall(&U256 { low: 0xff, high: 0 });

    // Then, we use the account exposed by the contract to execute the multicall.
    // Once again, there is no abstraction on starknet-rs type, so you have
    // the full control from starknet-rs library.
    let _tx_res = contract
        .account
        .execute(vec![set_a_call, set_b_call])
        .send()
        .await
        .expect("Multicall failed");

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let a = contract
        .get_a()
        .call()
        .await
        .expect("Call to `get_a` failed");
    println!("a after multicall: {:?}", a);

    let b = contract
        .get_b()
        .call()
        .await
        .expect("Call to `get_b` failed");
    println!("b after multicall: {:?}", b);

    // Let's send this to an other thread.
    // Remember, ConnectedAccount is implemented for Arc<ConnectedAccount>.
    let arc_contract = Arc::new(contract);

    let handle = tokio::spawn(async move {
        other_func(arc_contract.clone()).await;
    });

    handle.await.unwrap();
}

async fn other_func<A: ConnectedAccount + Sync + 'static>(contract: Arc<MyContract<A>>) {
    let set_b = contract.set_b(&U256 { low: 0xfe, high: 0 });

    // Example of estimation of fees.
    let estimated_fee = set_b
        .estimate_fee()
        .await
        .expect("Fail to estimate")
        .overall_fee;

    // Use the estimated fees as a base.
    let _tx_res = set_b
        .max_fee(estimated_fee * FieldElement::TWO)
        .send()
        .await
        .expect("invoke failed");

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let b = contract
        .get_b()
        .call()
        .await
        .expect("Call to `get_b` failed");
    println!("b set in task: {:?}", b);

    let arr = vec![FieldElement::THREE, FieldElement::ONE, FieldElement::ZERO];

    let tx_res = contract
        .set_array(&arr)
        .send()
        .await
        .expect("invoke set_array failed");
    println!("tx_res in task: {:?}", tx_res);

    let a = contract
        .get_a()
        .call()
        .await
        .expect("Call to `get_a` failed");
    println!("a set in task: {:?}", a);
}
