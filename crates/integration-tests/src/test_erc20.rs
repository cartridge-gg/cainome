use katana_runner::RunnerCtx;

use starknet::accounts::Account;
use starknet_types_core::felt::Felt;

use crate::bindings::erc20::ERC20;

const UDC_ADDRESS: Felt =
    Felt::from_hex_unwrap("0x41a78e741e5af2fec34b695679bc6891742439f7afb8484ecd7766661ad02bf");

#[tokio::test]
#[katana_runner::test(accounts = 2, fee = false, block_time = 1)]
async fn deploy_erc_20_and_call_its_methods(runner: &RunnerCtx) {
    // Predeployed accounts in katana
    let account = runner.account(0);

    let path = std::path::Path::new("./src/bindings/erc20.json");

    let class_hash = ERC20::declare(path, &account).await.unwrap();

    println!("Class hash: {}", class_hash.to_hex_string());

    runner.dev_client().generate_block().await.unwrap();

    let erc20 = ERC20::deploy(UDC_ADDRESS, &account, class_hash, vec![account.address()])
        .await
        .unwrap();

    println!(
        "Deployed ERC20 at address: {}",
        erc20.address.to_hex_string()
    );

    let low = Felt::from(1374587365u32);

    // TODO: migrate u256. check core library implementation.
    // Let's mint some!
    erc20
        .mint(
            &account.address().into(),
            &cainome_cairo_serde::U256::try_from((low, Felt::ZERO)).unwrap(),
        )
        .send()
        .await
        .unwrap();

    // Commit
    runner.dev_client().generate_block().await.unwrap();

    // Check balance
    let actual = erc20
        .balance_of(&account.address().into())
        .call()
        .await
        .unwrap();

    let expected = cainome_cairo_serde::U256::try_from((low, Felt::ZERO)).unwrap();

    assert_eq!(actual, expected);
}
