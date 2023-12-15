#[starknet::contract]
mod gen {
    use starknet::ContractAddress;

    #[storage]
    struct Storage {
        v1: felt252,
        v2: felt252,
    }

    #[derive(Serde, Drop)]
    struct MyStruct<T> {
        f1: felt252,
        f2: T,
        f3: felt252,
    }

    #[external(v0)]
    fn func1(ref self: ContractState, a: MyStruct<felt252>) {
        self.v1.write(a.f1);
        self.v2.write(a.f2);
    }

    #[external(v0)]
    fn func2(ref self: ContractState, a: MyStruct<u256>) {
        self.v1.write(a.f2.low.into());
        self.v2.write(a.f2.high.into());
    }

    #[external(v0)]
    fn read(self: @ContractState) -> (felt252, felt252) {
        (self.v1.read(), self.v2.read())
    }
}
