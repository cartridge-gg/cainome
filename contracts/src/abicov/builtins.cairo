//! A simple contract without interfaces to test builtins.

#[starknet::contract]
mod builtins {
    use starknet::{ClassHash, ContractAddress, EthAddress};
    use core::zeroable::NonZero;
    //use core::integer::BoundedInt;

    #[derive(Serde, Drop)]
    pub struct MyStruct {
        a: NonZero<felt252>,
    }

    #[storage]
    struct Storage {}

    #[constructor]
    fn constructor(ref self: ContractState) {}

    #[external(v0)]
    fn struct_non_zero(self: @ContractState, res: MyStruct) -> felt252 {
        1
    }

    #[external(v0)]
    fn non_zero(self: @ContractState, res: NonZero<felt252>) -> felt252 {
        2
    }
}
