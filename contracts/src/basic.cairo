#[starknet::contract]
mod basic {
    use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};

    #[storage]
    struct Storage {
        v1: NonZero<felt252>,
        v2: u256,
        v3: felt252,
    }

    #[external(v0)]
    fn set_storage(ref self: ContractState, v1: NonZero<felt252>, v2: u256) {
        self.v1.write(v1);
        self.v2.write(v2);
    }

    #[external(v0)]
    fn read_storage_tuple(self: @ContractState) -> (NonZero<felt252>, u256) {
        (self.v1.read(), self.v2.read())
    }
}
