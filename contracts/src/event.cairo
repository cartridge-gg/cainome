#[starknet::contract]
mod event {
    use starknet::ContractAddress;

    #[storage]
    struct Storage {}

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        MyEventA: MyEventA,
        MyEventB: MyEventB,
        MyEventC: MyEventC,
    }

    #[derive(Drop, starknet::Event)]
    struct MyEventA {
        #[key]
        header: felt252,
        value: Span<felt252>,
    }

    #[derive(Drop, starknet::Event)]
    struct MyEventB {
        value: felt252,
    }

    #[derive(Drop, starknet::Event)]
    struct MyEventC {
        #[key]
        v1: felt252,
        #[key]
        v2: felt252,
        v3: felt252,
        v4: ContractAddress,
    }

    #[external(v0)]
    fn read(ref self: ContractState) -> felt252 {
        2
    }

    #[external(v0)]
    fn emit_a(ref self: ContractState, header: felt252, value: Span<felt252>) {
        self.emit(MyEventA { header, value });
    }

    #[external(v0)]
    fn emit_b(ref self: ContractState, value: felt252) {
        self.emit(MyEventB { value });
    }

    #[external(v0)]
    fn emit_c(ref self: ContractState, v1: felt252, v2: felt252, v3: felt252, v4: ContractAddress) {
        self.emit(MyEventC { v1, v2, v3, v4 });
    }
}
