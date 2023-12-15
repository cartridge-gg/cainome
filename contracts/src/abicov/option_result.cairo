//! A simple contract without interfaces
//! to test very basic types.

#[starknet::contract]
mod option_result {
    use starknet::{ClassHash, ContractAddress, EthAddress};

    #[derive(Serde, Drop)]
    struct GenericOne<T> {
        a: T,
        b: felt252,
        c: u256,
    }

    #[storage]
    struct Storage {}

    #[constructor]
    fn constructor(ref self: ContractState) {}

    #[external(v0)]
    fn result_ok_unit(self: @ContractState, res: Result<(), felt252>) -> Result<u64, felt252> {
        Result::Ok(2_u64)
    }

    #[external(v0)]
    fn result_ok_struct(
        self: @ContractState, res: Result<GenericOne<felt252>, felt252>
    ) -> Result<u64, felt252> {
        Result::Ok(2_u64)
    }

    #[external(v0)]
    fn result_ok_tuple_struct(
        self: @ContractState, res: Result<(GenericOne<felt252>, felt252), felt252>
    ) -> Result<u64, felt252> {
        Result::Ok(2_u64)
    }

    #[external(v0)]
    fn result_ok(self: @ContractState, res: Result<felt252, u256>) -> Result<u64, felt252> {
        Result::Ok(2_u64)
    }

    #[external(v0)]
    fn result_err(self: @ContractState, res: Result<felt252, felt252>) -> Result<felt252, u256> {
        Result::Err(0xff_u256)
    }

    #[external(v0)]
    fn option_some(self: @ContractState, opt: Option<felt252>) -> Option<Span<felt252>> {
        Option::Some(array![1, 2].span())
    }

    #[external(v0)]
    fn option_none(self: @ContractState, opt: Option<felt252>) -> Option<u64> {
        Option::None
    }
}
