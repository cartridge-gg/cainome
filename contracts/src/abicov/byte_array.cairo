#[starknet::contract]
mod byte_array {
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

//! Naive implementation of `Store` trait for `ByteArray`.
//!
//! The idea in this implementation is to store the `ByteArray`
//! data by hashing a key for each element of it.
//!
//! 1. First, the `data.len()` value is stored at the given base address.
//! 2. The `pending_word` is stored computing a key with `PENDING_WORD_KEY`.
//! 3. The `pending_word_len` is stored computing a key with `PENDING_WORD_LEN_KEY`.
//! 4. For each element of data, the key is computed using the index of the element in the array.
//!
use core::ByteArray;
use core::hash::LegacyHash;
use starknet::{
    storage_read_syscall, storage_write_syscall, storage_base_address_from_felt252,
    storage_address_to_felt252, storage_address_from_base, storage_address_from_base_and_offset,
    SyscallResult, SyscallResultTrait, StorageBaseAddress, Store,
};

const BYTE_ARRAY_KEY: felt252 = '__BYTE_ARRAY__';
const PENDING_WORD_KEY: felt252 = '__PENDING_WORD__';
const PENDING_WORD_LEN_KEY: felt252 = '__PENDING_WORD_LEN__';

/// Implementation of `LegacyHash` for `ByteArray`.
/// The hash is given by the poseidon hash of the `data` and `pending_word` fields.
impl LegacyHashByteArray of LegacyHash<ByteArray> {
    fn hash(state: felt252, value: ByteArray) -> felt252 {
        let mut buffer = array![value.pending_word];

        let mut index = 0;

        loop {
            if index == value.data.len() {
                break;
            }

            buffer.append((*value.data[index]).into());

            index += 1;
        };

        poseidon::poseidon_hash_span(buffer.span())
    }
}
