//! A contract with structs.
use starknet::{ClassHash, ContractAddress, EthAddress};

#[derive(Serde, Drop)]
struct Simple {
    felt: felt252,
    uint256: u256,
    uint64: u64,
    address: ContractAddress,
    class_hash: ClassHash,
    eth_address: EthAddress,
    tuple: (felt252, u256),
    span: Span<felt252>,
}

#[derive(Serde, Drop)]
struct StructWithStruct {
    simple: Simple,
}

#[derive(Serde, Drop)]
struct GenericOne<T> {
    a: T,
    b: felt252,
    c: u256,
}

#[derive(Serde, Drop)]
struct GenericTwo<T, U> {
    a: T,
    b: U,
    c: felt252,
}

// NOT SUPPORTED.
// #[derive(Serde, Drop)]
// struct GenericThree<T, U, V> {
//     a: T,
//     b: U,
//     c: V,
// }

// NOT SUPPORTED.
// #[derive(Serde, Drop)]
// struct GenericOfGeneric<T> {
//     a: GenericOne<T>,
// }

#[starknet::contract]
mod structs {
    use super::{Simple, StructWithStruct, GenericOne, GenericTwo};

    #[storage]
    struct Storage {}

    #[external(v0)]
    fn get_simple(self: @ContractState) -> Simple {
        Simple {
            felt: 1,
            uint256: 2_u256,
            uint64: 3_u64,
            address: 0x1234.try_into().unwrap(),
            class_hash: 0x1122.try_into().unwrap(),
            eth_address: 0x3344.try_into().unwrap(),
            tuple: (1, 2_u256),
            span: array![1, 2, 3, 4].span(),
        }
    }

    #[external(v0)]
    fn set_simple(ref self: ContractState, simple: Simple) {}

    #[external(v0)]
    fn get_struct_w_struct(self: @ContractState) -> StructWithStruct {
        StructWithStruct {
            simple: Simple {
                felt: 1,
                uint256: 2_u256,
                uint64: 3_u64,
                address: 0x1234.try_into().unwrap(),
                class_hash: 0x1122.try_into().unwrap(),
                eth_address: 0x3344.try_into().unwrap(),
                tuple: (1, 2_u256),
                span: array![1, 2, 3, 4].span(),
            }
        }
    }

    #[external(v0)]
    fn set_struct_w_struct(ref self: ContractState, sws: StructWithStruct) {}

    #[external(v0)]
    fn get_generic_one(self: @ContractState) -> GenericOne<felt252> {
        GenericOne { a: 1, b: 2, c: 3_u256, }
    }

    #[external(v0)]
    fn get_generic_one_array(self: @ContractState) -> GenericOne<Span<felt252>> {
        GenericOne { a: array![1, 2].span(), b: 2, c: 3_u256, }
    }

    #[external(v0)]
    fn set_generic_one(ref self: ContractState, generic: GenericOne<u256>) {}

    #[external(v0)]
    fn set_generic_two_2(ref self: ContractState, generic: GenericTwo<u64, u64>) {}

    #[external(v0)]
    fn set_generic_two_0(ref self: ContractState, generic: GenericTwo<u128, u64>) {}

    #[external(v0)]
    fn set_generic_two(ref self: ContractState, generic: GenericTwo<u64, u128>) {}

    #[external(v0)]
    fn get_generic_two(self: @ContractState) -> GenericTwo<felt252, u256> {
        GenericTwo { a: 1, b: 2_u256, c: 3, }
    }


    #[external(v0)]
    fn set_tuple_generic(
        ref self: ContractState, value: (GenericOne<u256>, GenericTwo<felt252, u64>)
    ) {}

    #[external(v0)]
    fn get_tuple_of_array_generic(self: @ContractState) -> (Span<GenericOne<u64>>, Span<felt252>) {
        (array![GenericOne { a: 0x1, b: 0x2, c: 0x3_u256, },].span(), array![1, 2, 3].span(),)
    }
// #[external(v0)]
// fn set_generic_three_1(ref self: ContractState, generic: GenericThree<u64, u64, u64>) {}

// #[external(v0)]
// fn set_generic_three_2(ref self: ContractState, generic: GenericThree<u64, u64, u128>) {}

// #[external(v0)]
// fn set_generic_three_3(ref self: ContractState, generic: GenericThree<u128, u32, u128>) {}
}
