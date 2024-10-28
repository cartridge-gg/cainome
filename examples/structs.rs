use std::str::FromStr;

use cainome::rs::abigen;
use paste::paste;
use starknet::core::types::Felt;

abigen!(
    MyContract,
    "./contracts/abi/gen.abi.json",
    derives(
        Debug,
        Clone,
        PartialEq,
        serde::Serialize,
        serde::Deserialize
    )
);

/// Uses paste since `concat_ident` is not available for stable Rust yet.
macro_rules! test_enum {
    ($name:ident, $variant:expr) => {
        paste! {
            let $name = $variant;
            let [<$name _deser>] = serde_json::from_str(&serde_json::to_string(&$name).unwrap()).unwrap();
            assert_eq!($name, [<$name _deser>]);
        }
    };
}

#[tokio::main]
async fn main() {
    assert_eq!(
        E1::selector(),
        Felt::from_str("0x00ba2026c84b59ce46a4007300eb97e3e275d4119261ee402d7a3eb40ad58807")
            .unwrap()
    );

    assert_eq!(E1::event_name(), "E1");

    let s = PlainStruct {
        f1: 1,
        f2: 2,
        f3: 3,
        f4: 4,
        f5: 5,
        f6: Felt::from(6),
        f7: (Felt::from(7), 8),
        f8: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        f9: vec![1_u128, 2_u128],
    };

    let s_str = serde_json::to_string(&s).unwrap();
    println!("{}", s_str);

    let s_deser = serde_json::from_str(&s_str).unwrap();
    assert_eq!(s, s_deser);
    println!("{:?}", s_deser);

    let _s2 = s.clone();

    test_enum!(e1, MyEnum::One(1_u8));
    test_enum!(e2, MyEnum::Two(1_u16));
    test_enum!(e3, MyEnum::Three(1_u32));
    test_enum!(e4, MyEnum::Four(1_u64));
    test_enum!(e5, MyEnum::Five(1_u128));
    test_enum!(e6, MyEnum::Six(Felt::from(6)));
    test_enum!(e7, MyEnum::Seven(-1_i32));
    test_enum!(e8, MyEnum::Eight(-1_i64));
    test_enum!(e9, MyEnum::Nine(-1_i128));
    test_enum!(e10, MyEnum::Ten((1_u8, 1_u128)));
    test_enum!(e11, MyEnum::Eleven((Felt::from(1), 1_u8, 1_u128)));
}
