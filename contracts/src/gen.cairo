#[starknet::contract]
mod gen {
    use starknet::ContractAddress;

    #[storage]
    struct Storage {
        v1: felt252,
        v2: felt252,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        E1: E1,
    }

    #[derive(Drop, starknet::Event)]
    struct E1 {
        #[key]
        key: felt252,
        value: Span<felt252>,
    }

    #[derive(Serde, Drop)]
    struct PlainStruct {
        f1: u8,
        f2: u16,
        f3: u32,
        f4: u64,
        f5: u128,
        f6: felt252,
        f7: (felt252, u64),
        f8: Array<u8>,
        f9: Array<u128>,
    }

    #[derive(Serde, Drop)]
    struct MyStruct<T> {
        f1: felt252,
        f2: T,
        f3: felt252,
    }

    #[derive(Serde, Drop)]
    enum MyEnum {
        One: u8,
        Two: u16,
        Three: u32,
        Four: u64,
        Five: u128,
        Six: felt252,
        Seven: i32,
        Eight: i64,
        Nine: i128,
        Ten: (u8, u128),
        Eleven: (felt252, u8, u128),
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

    #[external(v0)]
    fn func3(self: @ContractState, _a: PlainStruct) {}

    #[external(v0)]
    fn func4(self: @ContractState, _a: MyEnum) {}
}

