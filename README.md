# Cainome: bindings generation from Cairo ABI

Cainome is a library to generate bindings from Cairo ABI.

Cainome architecture provides a flexible way to work with Cairo ABI
for different languages (backends).

## Project structure

- **cli**: inside `src/bin/cli`, the cainome CLI binary can be built using `cargo build`: [README](./src/bin/cli/README.md).
- **lib**: inside `src/lib.rs`, the cainome library can be built using `cargo build --lib`.
- **parser**: a run-time library to parse an ABI file into `Token`s [README](./crates/parser/README.md).
- **cairo-serde**: a compile-time library that implements serialization for native Rust types from `Felt` buffer [README](./crates/cairo-serde/README.md).
- **rs-macro**: a compile-time library backend for the `abigen` macro to generate rust bindings [README](./crates/rs-macro/README.md).
- **rs**: a a run-time library to generated rust bindings [README](./crates/rs/README.md).
- **ts**: a compile-time library backend to generate `TypeScript` bindings (coming soon).

Currently those crates are not published on crates.io, please consider using them with the release tags.

## Plugin system

Cainome uses a plugin system that is for now only supporting `built-in` plugins (written in rust).
Cainome will support in the future plugins like `protobuf`, which can be written in any languages.

### How to write a plugin

Currently, to write a plugin you can take as example the `RustPlugin`.

1. Define a rust module inside `src/bin/cli/plugins/builtins`.
2. You can write your plugin code in a crate (like `rs` crate), or in the module you've created at the previous step (use a folder in this case).
   Writting a crate can be easier to re-use in other projects though.
3. The plugin takes a `PluginInput` as argument, where the [tokens from the parser crate](./crates/parser/src/tokens/mod.rs) are available for each contract.
   From these tokens, you can easily generate code that represent the ABI of the contract. In the case of rust, you can find in the `rs` crate
   some examples of how types are handled.
   You don't have to use `syn` crate as `rs` crate is doing. You can simply build strings.
4. In the current version, the plugin also receives the `output_dir`, so it is responsible of writing and organizing it's files.
5. Finally, add in the [PluginOptions](./src/bin/cli/args.rs) an option for your plugin.

## Cainome meaning

Cainome is a word combining `Cairo` and `Genome`. The idea of `Cairo ABI` being the DNA of our ecosystem,
and like the genome expresses several genes which turn into proteins, from an `ABI` we can generate several bindings in different languages.
