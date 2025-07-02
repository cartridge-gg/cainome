//! A contract with enums.

#[derive(Serde, Drop, starknet::Store)]
enum SimpleEnum {
    #[default]
    Variant1,
    Variant2,
}

#[derive(Serde, Drop, starknet::Store)]
enum TypedEnum {
    #[default]
    Variant1: felt252,
    Variant2: u256,
    Variant3: (felt252, u256),
    Variant4: starknet::ContractAddress,
}

#[derive(Serde, Drop, starknet::Store)]
enum MixedEnum {
    #[default]
    Variant1: felt252,
    Variant2,
}

#[starknet::contract]
mod enums {
    use starknet::storage::StoragePointerReadAccess;
    use super::{MixedEnum, SimpleEnum, TypedEnum};

    #[storage]
    struct Storage {
        simple: SimpleEnum,
        typed: TypedEnum,
        mixed: MixedEnum,
    }

    #[external(v0)]
    fn get_simple_1(self: @ContractState) -> SimpleEnum {
        self.simple.read()
    }

    #[external(v0)]
    fn get_simple_2(self: @ContractState) -> SimpleEnum {
        SimpleEnum::Variant2
    }

    #[external(v0)]
    fn get_typed_1(self: @ContractState) -> TypedEnum {
        TypedEnum::Variant1(0x123)
    }

    #[external(v0)]
    fn get_typed_2(self: @ContractState) -> TypedEnum {
        TypedEnum::Variant2(0xff_u256)
    }

    #[external(v0)]
    fn get_typed_3(self: @ContractState) -> TypedEnum {
        TypedEnum::Variant3((1, 0xffffff_u256))
    }

    #[external(v0)]
    fn get_typed_4(self: @ContractState) -> TypedEnum {
        TypedEnum::Variant4(42.try_into().unwrap())
    }

    #[external(v0)]
    fn get_typed_with_arg(self: @ContractState, e: TypedEnum) -> TypedEnum {
        e
    }

    #[external(v0)]
    fn get_typed_with_option_arg(self: @ContractState, e: Option<TypedEnum>) -> Option<TypedEnum> {
        e
    }

    #[external(v0)]
    fn get_mixed_1(self: @ContractState) -> MixedEnum {
        MixedEnum::Variant1(0x123)
    }

    #[external(v0)]
    fn get_mixed_2(self: @ContractState) -> MixedEnum {
        MixedEnum::Variant2
    }
}
