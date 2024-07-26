use cainome::rs::abigen;

use starknet::core::types::Felt;

abigen!(
    MyContract,
    "./contracts/abi/gen.abi.json",
    derives(Debug, Clone)
);

#[tokio::main]
async fn main() {
    let s = MyStruct::<Felt> {
        f1: Felt::ONE,
        f2: Felt::TWO,
        f3: Felt::THREE,
    };

    println!("{:?}", s);

    let _s2 = s.clone();
}
