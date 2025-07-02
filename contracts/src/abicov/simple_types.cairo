//! A simple contract without interfaces
//! to test very basic types.

#[starknet::contract]
mod simple_types {
    use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
    use starknet::{ClassHash, ContractAddress, EthAddress};

    #[storage]
    struct Storage {
        felt: felt252,
        uint256: u256,
        uint64: u64,
        address: ContractAddress,
        class_hash: ClassHash,
        eth_address: EthAddress,
        tuple: (felt252, u256),
        boolean: bool,
    }

    #[constructor]
    fn constructor(ref self: ContractState) {
        self.felt.write(0x1234);
    }

    #[external(v0)]
    fn get_bool(self: @ContractState) -> bool {
        self.boolean.read()
    }

    #[external(v0)]
    fn set_bool(ref self: ContractState, v: bool) {
        self.boolean.write(v);
    }

    #[external(v0)]
    fn get_felt(self: @ContractState) -> felt252 {
        self.felt.read()
    }

    #[external(v0)]
    fn set_felt(ref self: ContractState, felt: felt252) {
        self.felt.write(felt);
    }

    #[external(v0)]
    fn get_u256(self: @ContractState) -> u256 {
        self.uint256.read()
    }

    #[external(v0)]
    fn set_u256(ref self: ContractState, uint256: u256) {
        self.uint256.write(uint256);
    }

    #[external(v0)]
    fn get_u64(self: @ContractState) -> u64 {
        self.uint64.read()
    }

    #[external(v0)]
    fn set_u64(ref self: ContractState, uint64: u64) {
        self.uint64.write(uint64);
    }

    #[external(v0)]
    fn get_address(self: @ContractState) -> ContractAddress {
        self.address.read()
    }

    #[external(v0)]
    fn set_address(ref self: ContractState, address: ContractAddress) {
        self.address.write(address);
    }

    #[external(v0)]
    fn get_class_hash(self: @ContractState) -> ClassHash {
        self.class_hash.read()
    }

    #[external(v0)]
    fn set_class_hash(ref self: ContractState, class_hash: ClassHash) {
        self.class_hash.write(class_hash);
    }

    #[external(v0)]
    fn get_eth_address(self: @ContractState) -> EthAddress {
        self.eth_address.read()
    }

    #[external(v0)]
    fn set_eth_address(ref self: ContractState, eth_address: EthAddress) {
        self.eth_address.write(eth_address);
    }

    #[external(v0)]
    fn get_tuple(self: @ContractState) -> (felt252, u256) {
        self.tuple.read()
    }

    #[external(v0)]
    fn set_tuple(ref self: ContractState, tuple: (felt252, u256)) {
        self.tuple.write(tuple);
    }

    #[external(v0)]
    fn get_bool_with_tuple_args(self: @ContractState, nonce: (felt252, u128)) -> bool {
        true
    }

    #[external(v0)]
    fn get_array(self: @ContractState) -> Span<felt252> {
        let felt = self.felt.read();
        let uint256 = self.uint256.read();
        let uint64 = self.uint64.read();

        array![felt, uint256.low.into(), uint256.high.into(), uint64.into()].span()
    }

    #[external(v0)]
    fn set_array(ref self: ContractState, data: Span<felt252>) {
        assert(data.len() == 4, 'bad data len (expected 4)');
        self.felt.write(*data[0]);
        self
            .uint256
            .write(
                u256 {
                    low: (*data[1]).try_into().expect('invalid u128'),
                    high: (*data[2]).try_into().expect('invalid u128'),
                },
            );
        self.uint64.write((*data[3]).try_into().expect('invalid u64'));
    }
}
