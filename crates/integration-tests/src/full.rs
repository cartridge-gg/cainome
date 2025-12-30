use core::panic;
use katana_runner::RunnerCtx;
use starknet::{
    accounts::Account,
    core::{
        types::{Call, TransactionReceiptWithBlockInfo},
        utils::get_selector_from_name,
    },
    providers::Provider,
};
use starknet_types_core::felt::Felt;
use std::sync::Arc;

use crate::bindings::{erc20::ERC20, udc};

pub fn get_class_hash(
    path: &std::path::Path,
) -> Result<starknet::core::types::Felt, Box<dyn std::error::Error>> {
    let sierra_class: cairo_lang_starknet_classes::contract_class::ContractClass =
        serde_json::from_slice::<cairo_lang_starknet_classes::contract_class::ContractClass>(
            std::fs::read(path)?.as_slice(),
        )?;

    let casm_class =
        cairo_lang_starknet_classes::casm_contract_class::CasmContractClass::from_contract_class(
            sierra_class,
            false,
            180000, // TODO: to settings
        )?;

    let class_hash = casm_class.compiled_class_hash();

    Ok(starknet::core::types::Felt::from_bytes_be(
        &class_hash.to_bytes_be(),
    ))
}

pub async fn declare<'a, A>(
    path: &std::path::Path,
    account: &'a A,
) -> Result<starknet::core::types::Felt, Box<dyn std::error::Error>>
where
    A: starknet::accounts::ConnectedAccount + Sync,
    A::SignError: 'static,
{
    let class_hash = get_class_hash(path)?;

    let contract_artifact: starknet::core::types::contract::SierraClass =
        serde_json::from_reader(std::fs::File::open(path)?)?;

    let z = account.declare_v3(
        std::sync::Arc::new(contract_artifact.flatten()?),
        class_hash,
    );

    let x = z.send().await?;

    Ok(x.class_hash)
}

const UDC_ADDRESS: Felt =
    Felt::from_hex_unwrap("0x41a78e741e5af2fec34b695679bc6891742439f7afb8484ecd7766661ad02bf");
const SALT: Felt = Felt::from_hex_unwrap("0x123");

pub async fn deploy<'a, A>(
    account: &'a A,
    class_hash: Felt,
    constructor_args: Vec<Felt>,
) -> Result<starknet::core::types::InvokeTransactionResult, Box<dyn std::error::Error>>
where
    A: starknet::accounts::ConnectedAccount + Sync,
    A::SignError: 'static,
{
    let mut calldata = vec![
        class_hash,                    // class hash
        SALT,                          // salt
        Felt::ZERO,                    // unique
        constructor_args.len().into(), // constructor length
    ];

    calldata.extend(constructor_args);

    let deploy_call = vec![Call {
        to: UDC_ADDRESS,
        selector: get_selector_from_name("deployContract").unwrap(),
        calldata: calldata,
    }];

    let res = account.execute_v3(deploy_call).send().await?;

    Ok(res)
}

#[tokio::test]
#[katana_runner::test(accounts = 2, fee = false, block_time = 1)]
async fn deploy_erc_20_and_call_its_methods(runner: &RunnerCtx) {
    // Predeployed accounts in katana
    let account = runner.account(0);

    let path = std::path::Path::new("./src/bindings/erc20.json");
    let class_hash = declare(path, &account).await.unwrap();

    // Need to commit current block to make the declared class available for deployment
    runner.dev_client().generate_block().await.unwrap();

    let udc = udc::UDC::new(UDC_ADDRESS, &account);

    // UDC contract is Legacy, so calldata array should be explicitely prefixed with its length
    // TODO: discuss better bindings generation AsRef usage
    let deploy_result = udc
        .deployContract(
            &class_hash,
            &SALT,
            &Felt::ZERO,
            &Felt::ONE,
            &vec![account.address()].into(),
        )
        .send()
        .await
        .unwrap();

    // To get the actual deployed contract address we need to fetch transaction events and
    // find ContractDeployed event with class_hash we need
    let receipt_with_block_info: TransactionReceiptWithBlockInfo = runner
        .starknet_provider()
        .get_transaction_receipt(deploy_result.transaction_hash)
        .await
        .unwrap();

    let starknet::core::types::TransactionReceipt::Invoke(receipt) =
        receipt_with_block_info.receipt
    else {
        panic!("Expected Invoke receipt");
    };

    let mut deployed_address = None;
    for event in receipt.events {
        let Ok(event) = udc::ContractDeployed::try_from(event) else {
            continue;
        };

        if event.classHash != class_hash {
            continue;
        }

        deployed_address = Some(event.address);
    }

    let Some(deployed_address) = deployed_address else {
        panic!("Deployed contract address not found in events");
    };

    let erc20 = ERC20::new(deployed_address, &account);

    let low = Felt::from(1374587365u32);

    // Let's mint some!
    erc20
        .mint(
            &account.address().into(),
            // TODO: ugly
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
