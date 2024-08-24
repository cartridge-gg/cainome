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
        }
    ]"#,
    my_other_parameter(path = "hello")
);
