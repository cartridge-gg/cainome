#![no_main]
use cainome_rs_macro::abigen;

abigen!(
    MyStruct,
    r#"[
        {
            "type": "struct",
            "name": "core::integer::u256",
            "members": [
              {
                "name": "low",
                "type": "core::integer::u128"
              },
              {
                "name": "high",
                "type": "core::integer::u128"
              }
            ]
        },
        {
            "type": "struct",
            "name": "core::starknet::eth_address::EthAddress",
            "members": [
              {
                "name": "address",
                "type": "core::felt252"
              }
            ]
        }
    ]"#,
    type_aliases {
        core::integer::u256 as MyStruct;
        core::starknet::eth_address::EthAddress as MyStruct;
    }
);
