#[starknet::contract]
mod byte_array {
    use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};

    #[storage]
    struct Storage {
        string: ByteArray,
    }

    #[constructor]
    fn constructor(ref self: ContractState) {
        self.string.write("init")
    }

    #[external(v0)]
    fn get_byte_array(self: @ContractState) -> ByteArray {
        "cainome test a bit long to fit into a felt252"
    }

    #[external(v0)]
    fn get_byte_array_storage(self: @ContractState) -> ByteArray {
        self.string.read()
    }

    #[external(v0)]
    fn set_byte_array(ref self: ContractState, v: ByteArray) {
        self.string.write(v)
    }
}
