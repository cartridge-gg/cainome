pub const CAIRO_CORE_BASIC: [&str; 16] = [
    "core::felt252",
    "core::bool",
    "core::integer::u8",
    "core::integer::u16",
    "core::integer::u32",
    "core::integer::u64",
    "core::integer::u128",
    "core::integer::usize",
    "core::integer::i8",
    "core::integer::i16",
    "core::integer::i32",
    "core::integer::i64",
    "core::integer::i128",
    "core::starknet::contract_address::ContractAddress",
    "core::starknet::eth_address::EthAddress",
    "core::starknet::class_hash::ClassHash",
];

// Technically, a span is a struct. But it's here
// to match array pattern.
pub const CAIRO_CORE_SPAN_ARRAY: [&str; 2] = ["core::array::Span", "core::array::Array"];

pub const CAIRO_GENERIC_BUILTINS: [&str; 2] = ["core::option::Option", "core::result::Result"];
