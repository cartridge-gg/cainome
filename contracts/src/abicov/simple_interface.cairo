//! A simple contract with an interface.
//!
#[starknet::interface]
trait MyInterface<T> {
    fn get_value(self: @T) -> felt252;
    fn set_value(ref self: T, value: felt252);
}

#[starknet::contract]
mod simple_interface {
    use super::MyInterface;

    #[storage]
    struct Storage {
        value: felt252,
    }

    #[abi(embed_v0)]
    impl MyInterfaceImpl of MyInterface<ContractState> {
        fn get_value(self: @ContractState) -> felt252 {
            self.value.read()
        }

        fn set_value(ref self: ContractState, value: felt252) {
            self.value.write(value);
        }
    }
}
