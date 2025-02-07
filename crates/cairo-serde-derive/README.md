# Cairo Serde Derive

Implement the `CairoSerde` macro to derive the `CairoSerde` trait on your types.
The expanded core uses the `CairoSerde` type from `cainome-cairo-serde` crate, which will need to be imported in the crate that uses this derive.

`CairoSerde` allows to serialize and deserialize cairo types to/from native rust types.

## Example

```rust
#[derive(Debug, CairoSerde, PartialEq)]
struct ExampleSimple {
    x: Vec<Felt>,
    y: u32,
}

let example = ExampleSimple {
    x: vec![Felt::ZERO],
    y: 2,
};

let serialized = ExampleSimple::cairo_serialize(&example);

let offset = 0;
let deserialized = ExampleSimple::cairo_deserialize(&serialized, offset).unwrap();

assert_eq!(deserialized, example);
```
