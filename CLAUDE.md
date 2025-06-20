# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Core Development

- `cargo build` - Builds the entire workspace and all crates
- `cargo build --lib` - Builds only the library components
- `cargo build --bin cainome --features build-binary` - Builds the CLI binary
- `cargo run --bin cainome --features build-binary` - Runs the CLI tool

### Testing

- `cargo test` - Runs basic test suite
- `cargo test --workspace --all-features` - Runs all tests across the workspace with all features
- `./scripts/test_all.sh` - Runs comprehensive test suite including examples
- `cargo run --example <example_name> --all-features` - Runs specific examples

### Code Quality

- `cargo clippy` - Runs basic linting
- `./scripts/clippy.sh` - Runs comprehensive linting with all targets and features
- `cargo fmt` - Formats code
- `./scripts/fmt.sh` - Checks formatting without applying changes
- `./scripts/prettier.sh` - Formats non-Rust files (JSON, etc.)

### Documentation

- `cargo doc` - Generates documentation
- `./scripts/docs.sh` - Generates comprehensive documentation

### Package Management

- `cargo check` - Quick syntax and type checking
- `cargo clean` - Cleans build artifacts

## Architecture Overview

### Project Structure

This is a **Rust workspace** implementing a comprehensive **Cairo ABI binding generation system**. The project provides both library components and a CLI tool for generating Rust bindings from Cairo contract ABIs.

### Core Crates (`/crates/`)

**Core Libraries:**

- **`parser/`** - Runtime library for parsing ABI files into structured tokens. Handles both current and legacy Cairo ABI formats.
- **`cairo-serde/`** - Compile-time library implementing serialization/deserialization for native Rust types from Felt buffers. Core component for type safety.
- **`cairo-serde-derive/`** - Procedural macros for automatic derive implementation of Cairo serialization traits.

**Code Generation:**

- **`rs/`** - Runtime library for generated Rust bindings. Provides the foundation for contract interaction.
- **`rs-macro/`** - Compile-time library backend for the `abigen` macro. Generates Rust bindings from parsed ABI tokens.

### CLI Tool (`/src/bin/cli/`)

- **Purpose**: Command-line interface for generating bindings from Cairo ABI files
- **Plugin System**: Supports built-in plugins (currently Rust, with TypeScript planned)
- **Features**: ABI parsing, code generation, file organization

### Technology Stack

- **Language**: Rust (Edition 2021)
- **Framework**: Cargo workspace with feature flags
- **Dependencies**: StarkNet types, serde, tokio (for async CLI), clap (for CLI)
- **Testing**: Standard Rust test framework + examples
- **Code Quality**: clippy, rustfmt

### Cairo Integration

The project provides seamless integration with **Cairo and StarkNet**:

- **ABI Parsing**: Handles Cairo contract ABI JSON files
- **Type Mapping**: Maps Cairo types to native Rust types
- **Contract Interaction**: Generates type-safe Rust bindings for contract calls
- **Serialization**: Automatic Cairo serialization/deserialization

### Development Workflow

1. **Library Development**: Modify core functionality in individual crates
2. **CLI Development**: Update CLI interface and plugin system
3. **Testing**: Run comprehensive tests including practical examples
4. **Code Generation**: Test binding generation with various Cairo ABI formats

### Key Integration Points

- **StarkNet SDK**: Integration with starknet-rs for contract interaction
- **Cairo Contracts**: Direct ABI consumption from Cairo compiler output
- **Type Safety**: Compile-time guarantees for contract interaction
- **Code Generation**: Macro-based and CLI-based binding generation

### Plugin Architecture

Cainome uses an extensible plugin system:

- **Built-in Plugins**: Written in Rust, compiled into the binary
- **Current Support**: Rust bindings generation
- **Future Plans**: TypeScript, potentially protobuf-style external plugins
- **Plugin Input**: Parsed ABI tokens from the parser crate

## Claude Code Workflow Guidelines

### Code Quality Requirements

- **Always run linting** before committing: `./scripts/clippy.sh` for comprehensive checks
- **Format code** using: `./scripts/fmt.sh` to verify formatting
- **Test coverage** - Run comprehensive tests: `./scripts/test_all.sh`
- **Feature compliance** - Ensure all features build: `cargo build --all-features`

### Common Development Tasks

**Working with Core Libraries:**

- Modify parser logic in `crates/parser/src/`
- Update serialization in `crates/cairo-serde/src/`
- Test changes with: `cargo test --workspace --all-features`
- Verify with examples: `cargo run --example structs --all-features`

**Working with Code Generation:**

- Update binding generation in `crates/rs/src/expand/`
- Modify macro logic in `crates/rs-macro/src/`
- Test macro expansion with: `cargo expand` (if cargo-expand is installed)
- Validate with realistic ABI examples

**Working with CLI:**

- Modify CLI interface in `src/bin/cli/`
- Update plugin system and options
- Test CLI build: `cargo build --bin cainome --features build-binary`
- Run CLI: `cargo run --bin cainome --features build-binary -- --help`

**Adding New Features:**

- Consider impact across workspace crates
- Update relevant README files in crates
- Add examples demonstrating new functionality
- Ensure backward compatibility for library users

### Testing Strategy

- **Unit Tests**: Individual crate testing with `cargo test`
- **Integration Tests**: Workspace-wide testing with all features
- **Example Testing**: Practical examples in `/examples/` directory
- **ABI Compatibility**: Test with various Cairo ABI formats

### Debugging Integration Issues

- **ABI Parsing**: Check `crates/parser/test_data/` for reference formats
- **Type Mapping**: Verify serialization in `crates/cairo-serde/src/types/`
- **Code Generation**: Inspect generated code output
- **CLI Issues**: Use debug logging with `RUST_LOG=debug`

### Key Files to Check When Making Changes

- `Cargo.toml` - Workspace configuration and feature flags
- `crates/parser/src/abi/parser.rs` - Core ABI parsing logic
- `crates/rs-macro/src/lib.rs` - Main macro entry point
- `crates/rs/src/expand/` - Code generation templates
- `src/bin/cli/` - CLI interface and plugin system

### Cainome-Specific Considerations

- **ABI Compatibility**: Support both current and legacy Cairo ABI formats
- **Type Safety**: Maintain compile-time guarantees for contract interaction
- **Performance**: Efficient parsing and code generation for large ABIs
- **Usability**: Clear error messages and intuitive API design
- **Cairo Evolution**: Stay compatible with Cairo language updates
- **StarkNet Integration**: Maintain compatibility with starknet-rs ecosystem

### Example Usage Patterns

```rust
use cainome::rs::abigen;

// Generate bindings from ABI file
abigen!(MyContract, "/path/to/contract.abi.json");

// Use generated bindings
let contract = MyContract::new(contract_address, provider);
let result = contract.my_function(param).call().await?;
```

### Feature Flags

- **`default`**: Basic library functionality
- **`abigen-rs`**: Enables the abigen macro (requires rs-macro crate)
- **`build-binary`**: Enables CLI binary build (requires tokio)