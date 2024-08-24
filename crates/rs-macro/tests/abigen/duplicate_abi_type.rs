#![no_main]
use cainome_rs_macro::abigen;

abigen!(
    MyContract,
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
        }
    ]"#,
    type_aliases {
        core::integer::u256 as MyStruct1;
        core::integer::u256 as MyStruct2;
    }
);
