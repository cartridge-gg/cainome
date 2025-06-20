# Cainome CLI

Cainome CLI tool.

This tools is still in early development.

## How to build

`cargo install --git https://github.com/cartridge-gg/cainome --features="build-binary"`

or locally:

`cargo build`

## How to use

Quick usage overview of the binary.

### Rust bindings

1. To generate Rust bindings from a local artifacts path:

   ```
   cainome --artifacts-path /path/target/dev --output-dir /tmp --rust --execution-version v3
   ```

2. To fetch ABI from a chain, the name of the contract must be given:
   ```
   cainome --contract-address 0x1234.. --contract-name MyContract --rpc-url https://node.url --output-dir /tmp --rust --execution-version v3
   ```

### Go bindings

1. To generate Go bindings from a local artifacts path:

   ```
   cainome --artifacts-path /path/target/dev --output-dir /tmp --golang --golang-package mycontract --execution-version v3
   ```

2. To fetch ABI from a chain:
   ```
   cainome --contract-address 0x1234.. --contract-name MyContract --rpc-url https://node.url --output-dir /tmp --golang --golang-package mycontract --execution-version v3
   ```

### Available options

- `--rust`: Generate Rust bindings
- `--golang`: Generate Go bindings
- `--golang-package <name>`: Specify Go package name (default: "abigen")
- `--execution-version <v1|v3>`: StarkNet execution version
- `--artifacts-path <path>`: Path to directory containing `.contract_class.json` files
- `--contract-address <address>`: Contract address to fetch ABI from
- `--contract-name <name>`: Contract name (required when fetching from chain)
- `--rpc-url <url>`: StarkNet RPC endpoint
- `--output-dir <path>`: Output directory for generated bindings
