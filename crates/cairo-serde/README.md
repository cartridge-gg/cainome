# Cairo Serde

Cairo serde is a compile-time library that implement a trait `CairoSerde` on Rust native types.
By implementing this trait, the Rust type becomes (de)serializable from / into an array of `FieldElement`.

## Built-in types

The types considered built-in by Cairo Serde are the following:

```rust
pub const CAIRO_BASIC_STRUCTS: [&str; 4] = ["Span", "ClassHash", "ContractAddress", "EthAddress"];

pub const CAIRO_BASIC_ENUMS: [&str; 3] = ["Option", "Result", "bool"];
```

All those types, even if they are represented in the ABI as an `enum` or a `struct`, has their built-in Cairo Serde implementation in this crate.

# Supported types

Cairo Serde provides serialization support for the following types:

- `boolean` -> `bool`.
- `felt252` -> `starknet::core::types::FieldElement`.
- `integers (signed and unsigned)` -> `u[8,16,32,64,128], i[8,16,32,64,128], usize`.
- `Option` -> `Option`
- `Result` -> `Result`
- `ContractAddress` -> Custom type in this crate `ContractAddress`.
- `EthAddress` -> Custom type in this crate `EthAddress` (TODO: use the EthAddress from `starknet-rs`).
- `ClassHash` -> Custom type in this crate `ClassHash`.
- `Array/Span` -> `Vec`.
- `Tuple` -> native tuples + the unit `()` type.

## `CairoSerde` trait

Cairo Serde trait has for now a first interface that is the following:

```rust
pub trait CairoSerde {
    type RustType;

    fn serialized_size(_rust: &Self::RustType) -> usize;
    fn serialize(rust: &Self::RustType) -> Vec<FieldElement>;
    fn deserialize(felts: &[FieldElement], offset: usize) -> Result<Self::RustType>;
}
```

For now, while using the `deserilialize` method, you must provide the index in the buffer.

Some work that is in the roadmap:

- Adding a `serialize_to(rust: &Self::RustType, out: &mut Vec<FieldElement>)` to avoid allocating a new array for each type in a big felt buffer.
- Adding/modifying to `deserialize(felts: &[FieldElement]) -> Result<Self::RustType>` without the offset using rust slice. The motivation of using an explicit offset in the first version was to keep the context of the current deserialization operation in the global buffer.

## Examples

```rust
# Array/Span

# The length is automatically inserted as the first element of the `Vec`
# and all the values are converted into `FieldElement`.
let v: Vec<u32> = vec![1, 2, 3];
let felts = Vec::<u32>::serialize(&v);

let values = Vec::<u32>::deserialize(&felts, 0).unwrap();
```

```rust
# Option

# The variant index is handled by the library.
let o: Option<u32> = None;
let felts = Option::<u32>::serialize(&o);

let felts = vec![FieldElement::ONE];
let o = Option::<u32>::deserialize(&felts, 0).unwrap();

let o = Some(u32::MAX);
let felts = Option::<u32>::serialize(&o);

let felts = vec![FieldElement::ZERO, FieldElement::from(u32::MAX)];
let o = Option::<u32>::deserialize(&felts, 0).unwrap();
```

```rust
# Tuples
let v = (FieldElement::ONE, 128_u32);
let felts = <(FieldElement, u32)>::serialize(&v);

let felts = vec![FieldElement::THREE, 99_u32.into()];
let vals = <(FieldElement, u32)>::deserialize(&felts, 0).unwrap();
```
