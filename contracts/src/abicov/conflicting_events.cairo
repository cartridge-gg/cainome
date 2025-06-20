//! A contract with event that conflicts with simple_events.
//!
#[starknet::contract]
mod conflicting_events {
    #[storage]
    struct Storage {
        value: felt252,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        EventOnlyKey: EventOnlyKey,
    }

    #[derive(Drop, starknet::Event)]
    struct EventOnlyKey {
        #[key]
        value: felt252,
    }

    #[external(v0)]
    fn emit_super(ref self: ContractState) {
        self.emit(EventOnlyKey { value: 1 });
    }
}
