# Cainome Go Wrapper for `go generate`

This Go wrapper allows you to use cainome with Go's `go generate` command, providing seamless integration into your Go development workflow.

## Usage

Add a `//go:generate` directive to your Go source file:

```go
//go:generate go run -mod=mod github.com/cartridge-gg/cainome/src/bin/cli/plugins/builtins/golang --golang --golang-package mycontract --output-dir ./bindings  --execution-version v3 --artifacts-path ./contracts

package main
```

Then run:

```bash
go generate ./...
```

## Features

- **Automatic Installation**: If the cainome binary is not found, it will be automatically installed via cargo
- **Local Development Support**: Automatically detects and uses local development binaries when running within the cainome repository
- **Flexible Binary Location**: Searches multiple common installation paths
- **Environment Variable Configuration**: Extensive configuration options via environment variables

## Environment Variables

### Binary Location

- `CAINOME_BINARY`: Explicitly set the path to the cainome binary
  ```bash
  CAINOME_BINARY=/path/to/cainome go generate ./...
  ```

### Installation Control

- `CAINOME_NO_AUTO_INSTALL`: Disable automatic installation of the binary

  ```bash
  CAINOME_NO_AUTO_INSTALL=1 go generate ./...
  ```

- `CAINOME_VERSION`: Specify which version to install (default: "latest")

  ```bash
  CAINOME_VERSION=v0.3.0 go generate ./...
  ```

- `CAINOME_INSTALL_SOURCE`: Control where to install from
  - `"github"` or `"git"`: Install from GitHub (default)
  - `"crates.io"`: Install from crates.io
  - `/path/to/local/cainome`: Install from a local path
  ```bash
  CAINOME_INSTALL_SOURCE=crates.io go generate ./...
  ```

### Debugging

- `CAINOME_DEBUG`: Enable debug output
  ```bash
  CAINOME_DEBUG=1 go generate ./...
  ```

## Binary Search Order

The wrapper searches for the cainome binary in the following order:

1.  Path specified in `CAINOME_BINARY` environment variable
2.  Local development binary (when running within cainome repository)
    - `target/release/cainome`
    - `target/debug/cainome`
3.  System PATH
4.  Common installation locations:
    - `~/.cargo/bin/cainome`
    - `~/.local/bin/cainome`
    - `/usr/local/bin/cainome`
    - `/opt/homebrew/bin/cainome` (macOS ARM)
    - `/usr/local/homebrew/bin/cainome` (macOS Intel)

## Examples

### Basic Usage

```go
//go:generate go run -mod=mod github.com/cartridge-gg/cainome/cmd/cainome --golang --golang-package mycontract --output-dir ./generated ./contracts/my_contract.json
```

### Multiple Contracts

```go
//go:generate go run -mod=mod github.com/cartridge-gg/cainome/cmd/cainome --golang --golang-package token --output-dir ./generated/token ./contracts/token.json
//go:generate go run -mod=mod github.com/cartridge-gg/cainome/cmd/cainome --golang --golang-package nft --output-dir ./generated/nft ./contracts/nft.json
```

### With Specific Version

```bash
CAINOME_VERSION=v0.3.0 go generate ./...
```

### Using Local Development Binary

```bash
# When developing cainome locally
cd /path/to/cainome
cargo build --release

# In your project
CAINOME_BINARY=/path/to/cainome/target/release/cainome go generate ./...
```

### Debugging Issues

```bash
# Enable debug output to see which binary is being used
CAINOME_DEBUG=1 go generate ./...
```

## Troubleshooting

### Binary Not Found

If you get an error about the binary not being found:

1.  Ensure Rust and Cargo are installed: <https://rustup.rs/>
2.  Try installing manually: `cargo install --git https://github.com/cartridge-gg/cainome --bin cainome`
3.  Set the binary path explicitly: `CAINOME_BINARY=/path/to/cainome go generate ./...`

### Permission Denied

If you get permission errors during installation:

1.  Ensure you have write permissions to `~/.cargo/bin`
2.  Try using a different installation directory
3.  Install manually and set `CAINOME_BINARY`

### Version Conflicts

If you need different versions for different projects:

1.  Use `CAINOME_VERSION` environment variable per project
2.  Install specific versions to different locations and use `CAINOME_BINARY`
3.  Use direnv or similar tools to manage per-project environment variables
