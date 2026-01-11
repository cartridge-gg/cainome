// use cainome_cairo_serde_derive::CairoSerde;
// use cainome_rs_macro::abigen;
// use serde::Serialize;
// use starknet::core::types::Felt;

// /*
// use std::sync::Arc;
// use katana_runner::{KatanaRunner, KatanaRunnerConfig};
// use starknet::contract::ContractFactory;
// use starknet::core::types::{contract::SierraClass, BlockId, BlockTag};
// use starknet::accounts::{Account, ExecutionEncoding, SingleOwnerAccount}; */
// pub const CONTRACT_ARTIFACT: &str = "./contracts/target/dev/contracts_structs.contract_class.json";

// // This example uses an ABI where components introduce several enums with `Event` type name.
// // This showcase how the type_aliases parameter can be leveraged to avoid conflicts.
// #[derive(CairoSerde, Serialize)]
// pub struct GenericOneBis {
//     pub f1: Felt,
// }

// #[derive(CairoSerde, Serialize)]
// pub struct GenericTwoBis {
//     pub a: Felt,
// }

// #[derive(CairoSerde, Serialize)]
// pub struct MyDef {
//     pub a: Felt,
// }

// abigen!(
//     MyContract,
//     "contracts/abi/structs.abi.json",
//     type_substitutions {
//         contracts::abicov::structs::GenericOne as GenericOneBis;
//         contracts::abicov::structs::GenericTwo as GenericTwoBis;
//         contracts::abicov::structs::ToAlias as MyDef;
//     },
//     cainome_serde_path("cainome_cairo_serde"),
//     root_module_path("crate::test_substitutions"),
//     derives(Serde)
// );
