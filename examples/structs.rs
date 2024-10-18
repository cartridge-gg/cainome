use cainome::rs::abigen;
use starknet::core::types::Felt;

abigen!(
    MyContract,
    "./contracts/abi/gen.abi.json",
    derives(Debug, Clone, serde::Serialize)
);

#[tokio::main]
async fn main() {
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

    println!("{}", serde_json::to_string(&s).unwrap());

    let _s2 = s.clone();

    let e = MyEnum::One(1_u8);
    println!("{}", serde_json::to_string(&e).unwrap());

    let e = MyEnum::Two(1_u16);
    println!("{}", serde_json::to_string(&e).unwrap());

    let e = MyEnum::Three(1_u32);
    println!("{}", serde_json::to_string(&e).unwrap());

    let e = MyEnum::Four(1_u64);
    println!("{}", serde_json::to_string(&e).unwrap());

    let e = MyEnum::Five(1_u128);
    println!("{}", serde_json::to_string(&e).unwrap());

    let e = MyEnum::Six(Felt::from(6));
    println!("{}", serde_json::to_string(&e).unwrap());

    let e = MyEnum::Seven(-1_i32);
    println!("{}", serde_json::to_string(&e).unwrap());

    let e = MyEnum::Eight(-1_i64);
    println!("{}", serde_json::to_string(&e).unwrap());

    let e = MyEnum::Nine(-1_i128);
    println!("{}", serde_json::to_string(&e).unwrap());

    let e = MyEnum::Ten((1_u8, 1_u128));
    println!("{}", serde_json::to_string(&e).unwrap());

    let e = MyEnum::Eleven((Felt::from(1), 1_u8, 1_u128));
    println!("{}", serde_json::to_string(&e).unwrap());
}
