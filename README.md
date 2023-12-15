# Cainome: bindings generation from Cairo ABI

Cainome is a library to generate bindings from Cairo ABI.

Cainome architecture provides a flexible way to work with Cairo ABI
for different languages (backends).

## Project structure

- **parser**: a run-time library to parse an ABI file into `Token`s [README](./crates/parser/README.md).
- **cairo-serde**: a compile-time library that implements serialization for native Rust types from `FieldElement` buffer [README](./crates/cairo-serde/README.md).
- **rs**: a compile-time library backend for the `abigen` macro to generate rust bindings [README](./crates/rs/README.md).
- **ts**: a compile-time library backend to generate `TypeScript` bindings (coming soon).

Currently those crates are not published on crates.io, please consider using them with the release tags.

## Cainome meaning

Cainome is a word combining `Cairo` and `Genome`. The idea of `Cairo ABI` being the DNA of our ecosystem,
and like the genome expresses several genes which turn into proteins, from an `ABI` we can generate several bindings in different languages.
