# Cainome CLI

Cainome CLI tool.

This tools is still in early development, but it's goal is to support plugin development easily.

## How to build

`cargo build`

## How to use

Quick usage overview of the binary.

1. To generate bindings from a local artifacts path:

   ```
   cainome --artifacts-path /path/target/dev --output-dir /tmp --rust
   ```

2. To fetch ABI from a chain, the name of the contract must be given:
   ```
   cainome --contract-address 0x1234.. --contract-name MyContract --rpc-url https://node.url --output-dir /tmp --rust
   ```
