use std::sync::Arc;

use katana_runner::{KatanaDevClient, RunnerCtx};

use starknet::{
    accounts::{Account, ConnectedAccount},
    contract::UdcSelector,
    core::types::{BlockId, BlockTag},
};
use starknet_types_core::felt::Felt;

use crate::bindings::simple_get_set::{SimpleGetSet, SimpleGetSetReader};

const UDC_ADDRESS: Felt =
    Felt::from_hex_unwrap("0x41a78e741e5af2fec34b695679bc6891742439f7afb8484ecd7766661ad02bf");

#[tokio::test]
#[katana_runner::test(accounts = 2, fee = false, block_time = 1)]
async fn deploy_simple_get_set_and_assert_state(runner: &RunnerCtx) {
    let account = runner.account(0);

    let path = std::path::Path::new(
        "../../contracts/target/dev/contracts_simple_get_set.contract_class.json",
    );

    let class_hash = SimpleGetSet::declare(path, &account, true).await.unwrap();

    println!("Class hash: {}", class_hash.to_hex_string());

    runner.dev_client().generate_block().await.unwrap();

    let simple_get_set = SimpleGetSet::deploy(UdcSelector::Legacy, account.clone(), class_hash)
        .await
        .unwrap();

    println!(
        "Deployed SimpleGetSet at address: {}",
        simple_get_set.address.to_hex_string()
    );

    let reader_interface =
        SimpleGetSetReader::new(simple_get_set.address, runner.starknet_provider());

    runner.dev_client().generate_block().await.unwrap();

    // To call a view, there is no need to initialize an account. You can directly
    // use the name of the method in the ABI and then use the `call()` method.
    let a = reader_interface
        .get_a()
        .call()
        .await
        .expect("Call to `get_a` failed");

    assert_eq!(a, Felt::ZERO);

    // If you need to explicitely set the block id of the call, you can do as
    // following. The default value is "Pending". Or you can initialize a `ContractReader`
    // using the `with_block_id` method, that will be applied to each call.
    let b = reader_interface
        .get_b()
        .block_id(BlockId::Tag(BlockTag::Latest))
        .call()
        .await
        .expect("Call to `get_b` failed");

    assert_eq!(b, cainome_cairo_serde::U256 { low: 0, high: 0 });

    // The transaction is actually sent when `send()` is called.
    // You can before that configure the fees, or even only run an estimation of the
    // fees without actually sending the transaction.
    let _tx_res = simple_get_set
        .set_a(&(a + Felt::ONE))
        .send()
        .await
        .expect("Call to `set_a` failed");

    runner.dev_client().generate_block().await.unwrap();

    let a = simple_get_set
        .get_a()
        .call()
        .await
        .expect("Call to `get_a` failed");

    assert_eq!(a, Felt::ONE);

    // Now let's say we want to do multicall, and in one transaction we want to set a and b.
    // You can call the same function name with `_getcall` prefix to get the
    // call only, ready to be added in a multicall array.
    let set_a_call = simple_get_set.set_a_getcall(&Felt::from_hex("0xee").unwrap());
    let set_b_call =
        simple_get_set.set_b_getcall(&cainome_cairo_serde::U256 { low: 0xff, high: 0 });

    // Then, we use the account exposed by the contract to execute the multicall.
    // Once again, there is no abstraction on starknet-rs type, so you have
    // the full control from starknet-rs library.
    let _tx_res = simple_get_set
        .account
        .execute_v3(vec![set_a_call, set_b_call])
        .send()
        .await
        .expect("Multicall failed");

    runner.dev_client().generate_block().await.unwrap();

    let a = simple_get_set
        .get_a()
        .call()
        .await
        .expect("Call to `get_a` failed");

    assert_eq!(a, Felt::from_hex("0xee").unwrap());

    let b = simple_get_set
        .get_b()
        .call()
        .await
        .expect("Call to `get_b` failed");

    assert_eq!(b, cainome_cairo_serde::U256 { low: 0xff, high: 0 });

    // Let's send this to an other thread.
    // Remember, ConnectedAccount is implemented for Arc<ConnectedAccount>.
    let arc_contract = Arc::new(simple_get_set);

    let dev_client = runner.dev_client();
    let handle = tokio::spawn(async move {
        other_func(arc_contract.clone(), dev_client).await;
    });

    handle.await.unwrap()
}

async fn other_func<A: ConnectedAccount + Sync + 'static>(
    contract: Arc<SimpleGetSet<A>>,
    dev_client: KatanaDevClient,
) {
    let set_b = contract.set_b(&cainome_cairo_serde::U256 { low: 0xfe, high: 0 });

    let _tx_res = set_b.send().await.expect("invoke failed");

    dev_client.generate_block().await.unwrap();

    let b = contract
        .get_b()
        .call()
        .await
        .expect("Call to `get_b` failed");

    assert_eq!(b, cainome_cairo_serde::U256 { low: 0xfe, high: 0 });

    let arr = vec![Felt::THREE, Felt::ONE, Felt::ZERO];

    contract
        .set_array(&arr)
        .send()
        .await
        .expect("invoke set_array failed");

    let state_arr = contract.get_array().call().await.expect("get_array failed");

    assert_eq!(state_arr, arr);

    let a = contract
        .get_a()
        .call()
        .await
        .expect("Call to `get_a` failed");

    assert_eq!(a, Felt::THREE);
}
