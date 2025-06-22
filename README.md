# Cainome: bindings generation from Cairo ABI

Cainome is a library to generate bindings from Cairo ABI.

Cainome architecture provides a flexible way to work with Cairo ABI
for different languages (backends).

## When Cainome can be useful?

When you have to interact with a Cairo contract from **Rust** or **Go**, you can use Cainome to generate the bindings for you.

Cainome will totally abstract the Cairo serialization and deserialization, and you can focus on the logic around your contract.

### Rust usage example:

```rust
use cainome::rs::abigen;

abigen!(MyContract, "/path/project.contract_class.json");

fn main() -> Result<()> {
   // starknet-rs provider + contract address.
   let contract = MyContract::new(contract_address, provider);

   // Send transactions.
   let tx_result = contract.my_func(Felt::ONE).send().await?;

   // Call functions.
   let res = contract.my_view().call().await?;
}
```

### Go usage example:

```bash
# Generate Go bindings using CLI
cainome --golang --golang-package mycontract --output-dir ./bindings /path/project.contract_class.json
```

```go
package main

import (
    "context"
    "github.com/NethermindEth/starknet.go/account"
    "github.com/NethermindEth/starknet.go/rpc"
    "mycontract"
)

func main() {
    // Setup StarkNet provider and account
    provider := rpc.NewProvider("https://starknet-mainnet.public.blastapi.io")
    
    // Create contract reader for view functions
    reader := mycontract.NewReader(contractAddress, provider)
    
    // Call view functions
    result, err := reader.MyView(context.Background())
    if err != nil {
        panic(err)
    }
    
    // Create contract writer for transactions
    writer := mycontract.NewWriter(contractAddress, account)
    
    // Send transactions
    txResult, err := writer.MyFunc(context.Background(), feltValue)
    if err != nil {
        panic(err)
    }
}
```

For more details, refer to the different READMEs in the [github repository](https://github.com/cartridge-gg/cainome).

## Project structure

- **cli**: inside `src/bin/cli`, the cainome CLI binary can be built using `cargo build`: [README](./src/bin/cli/README.md).
- **lib**: inside `src/lib.rs`, the cainome library can be built using `cargo build --lib`.
- **parser**: a run-time library to parse an ABI file into `Token`s [README](./crates/parser/README.md).
- **cairo-serde**: a compile-time library that implements serialization for native Rust types from `Felt` buffer [README](./crates/cairo-serde/README.md).
- **rs-macro**: a compile-time library backend for the `abigen` macro to generate rust bindings [README](./crates/rs-macro/README.md).
- **rs**: a a run-time library to generated rust bindings [README](./crates/rs/README.md).
- **ts**: a compile-time library backend to generate `TypeScript` bindings (coming soon).
- **golang**: a built-in plugin for generating Go bindings via CLI.

Currently those crates are not published on crates.io, please consider using them with the release tags.

## Plugin system

Cainome uses a plugin system that currently supports two main approaches:

1. **Built-in plugins** (written in Rust): Rust and Go plugins are built into the CLI
2. **Future external plugins**: Cainome will support plugins like `protobuf`, which can be written in any language

### Available plugins:

- **Rust plugin**: Generate Rust bindings (via macro or CLI)
- **Go plugin**: Generate Go bindings (via CLI with `--golang` flag)

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
