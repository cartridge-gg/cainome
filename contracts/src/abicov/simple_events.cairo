//! A simple contract with events.
//!
#[starknet::contract]
mod simple_events {
    #[storage]
    struct Storage {
        value: felt252,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        EventOnlyKey: EventOnlyKey,
        EventOnlyData: EventOnlyData,
        EventAll: EventAll,
        EventMultiple: EventMultiple,
        EventNothing: EventNothing,
        SuperEvent: EventWithOtherName,
    }

    #[derive(Drop, starknet::Event)]
    struct EventOnlyKey {
        #[key]
        value: felt252,
    }

    #[derive(Drop, starknet::Event)]
    struct EventOnlyData {
        value: felt252,
    }

    #[derive(Drop, starknet::Event)]
    struct EventAll {
        #[key]
        header: felt252,
        value: Span<felt252>,
    }

    #[derive(Drop, starknet::Event)]
    struct EventMultiple {
        #[key]
        key1: felt252,
        #[key]
        key2: felt252,
        data1: felt252,
        data2: u256,
        data3: (felt252, felt252),
    }

    #[derive(Drop, starknet::Event)]
    struct EventNothing {}

    #[derive(Drop, starknet::Event)]
    struct EventWithOtherName {
        value: felt252,
    }

    #[external(v0)]
    fn emit_only_key(ref self: ContractState) {
        self.emit(EventOnlyKey { value: 1 });
    }

    #[external(v0)]
    fn emit_only_data(ref self: ContractState) {
        self.emit(EventOnlyData { value: 1 });
    }

    #[external(v0)]
    fn emit_all(ref self: ContractState) {
        self.emit(EventAll { header: 1, value: array![1].span() });
    }

    #[external(v0)]
    fn emit_multiple(ref self: ContractState) {
        self.emit(EventMultiple { key1: 1, key2: 2, data1: 3, data2: 4_u256, data3: (5, 6) });
    }

    #[external(v0)]
    fn emit_nothing(ref self: ContractState) {
        self.emit(EventNothing {});
    }

    #[external(v0)]
    fn emit_super(ref self: ContractState) {
        self.emit(EventWithOtherName { value: 1 });
    }
}
