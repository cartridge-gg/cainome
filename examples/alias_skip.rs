use cainome::rs::abigen;
use cainome_cairo_serde_derive::CairoSerde;
use serde::Serialize;
use starknet::core::types::Felt;

/*
use std::sync::Arc;
use katana_runner::{KatanaRunner, KatanaRunnerConfig};
use starknet::contract::ContractFactory;
use starknet::core::types::{contract::SierraClass, BlockId, BlockTag};
use starknet::accounts::{Account, ExecutionEncoding, SingleOwnerAccount}; */

pub const CONTRACT_ARTIFACT: &str = "./contracts/target/dev/contracts_structs.contract_class.json";

// This example uses an ABI where components introduce several enums with `Event` type name.
// This showcase how the type_aliases parameter can be leveraged to avoid conflicts.
#[derive(CairoSerde, Serialize)]
pub struct GenericOneBis {
    pub f1: Felt,
}

#[derive(CairoSerde, Serialize)]
pub struct GenericTwoBis {
    pub a: Felt,
}

#[derive(CairoSerde, Serialize)]
pub struct MyDef {
    pub a: Felt,
}

abigen!(
    MyContract,
    "./contracts/abi/structs.abi.json",
    type_aliases {
        contracts::abicov::structs::GenericOne as GenericOneBis;
        contracts::abicov::structs::GenericTwo as GenericTwoBis;
        contracts::abicov::structs::ToAlias as MyDef;
    },
    type_skips(
        contracts::abicov::structs::GenericOne, contracts::abicov::structs::GenericTwo, contracts::abicov::structs::ToAlias
    ),
    derives(serde::Serialize)
);

#[tokio::main]
async fn main() {
    let _one = GenericOneBis { f1: Felt::from(1) };
    let _two = GenericTwoBis { a: Felt::from(2) };
    let _three = MyDef { a: Felt::from(3) };

    // GenericOneBis, GenericTwoBis and MyDef not generated since they are skipped.

    /*  TODO: Katana runner not working with v3 yet.
    let katana_config = KatanaRunnerConfig {
        program_name: Some("/tmp/katana".to_string()),
        disable_fee: true,
        ..Default::default()
    };

    let katana = KatanaRunner::new_with_config(katana_config).unwrap();

    let contract_address = declare_deploy_contract(&katana).await;
    let mut account = katana.account(1);
    account.set_block_id(BlockId::Tag(BlockTag::PreConfirmed));

    let contract = MyContract::new(contract_address, account);

    let res = contract
        .set_from_alias(&vec![MyDef { a: Felt::from(1) }])
        .send()
        .await
        .unwrap();

    println!("res: {:?}", res); */
}
/*
async fn declare_deploy_contract(katana: &KatanaRunner) -> Felt {
    let mut account = katana.account(1);
    account.set_block_id(BlockId::Tag(BlockTag::PreConfirmed));

    let contract_artifact: SierraClass = serde_json::from_reader(
        std::fs::File::open("./contracts/target/dev/contracts_structs.contract_class.json")
            .unwrap(),
    )
    .unwrap();

    let class_hash = contract_artifact.class_hash().unwrap();

    let flattened_class = contract_artifact.flatten().unwrap();

    let _ = account
        .declare_v3(Arc::new(flattened_class), class_hash)
        .send()
        .await
        .unwrap();

    let contract_factory = ContractFactory::new(class_hash, account);

    let addr = contract_factory
        .deploy_v3(vec![], Felt::from(1122), false)
        .deployed_address();

    let _ = contract_factory
        .deploy_v3(vec![], Felt::from(1122), false)
        .send()
        .await
        .unwrap();

    addr
}
 */
