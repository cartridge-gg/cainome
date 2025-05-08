#[starknet::contract]
mod simple_get_set {
    use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};

    #[storage]
    struct Storage {
        a: felt252,
        b: u256,
    }

    #[derive(Serde, Drop)]
    enum TestEnum {
        V1: felt252,
        V2,
    }

    #[external(v0)]
    fn get_set_enum(self: @ContractState, v: TestEnum) -> TestEnum {
        match v {
            TestEnum::V1(v) => TestEnum::V1(v),
            TestEnum::V2 => TestEnum::V2,
        }
    }

    #[external(v0)]
    fn get_a(self: @ContractState) -> felt252 {
        self.a.read()
    }

    #[external(v0)]
    fn set_a(ref self: ContractState, a: felt252) {
        self.a.write(a);
    }

    #[external(v0)]
    fn get_b(self: @ContractState) -> u256 {
        self.b.read()
    }

    #[external(v0)]
    fn set_b(ref self: ContractState, b: u256) {
        self.b.write(b);
    }

    #[external(v0)]
    fn set_array(ref self: ContractState, data: Span<felt252>) {
        assert(data.len() == 3, 'bad data len');
        self.a.write(*data[0]);
        self
            .b
            .write(
                u256 { low: (*data[1]).try_into().unwrap(), high: (*data[2]).try_into().unwrap() },
            );
    }

    #[external(v0)]
    fn get_array(self: @ContractState) -> Span<felt252> {
        let b = self.b.read();

        array![self.a.read(), b.low.into(), b.high.into()].span()
    }
}
