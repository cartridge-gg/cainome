# Contributing to Cainome

Thank you for your interest in contributing to Cainome! This document provides guidelines and instructions for contributing to the project.

## Prerequisites

Before contributing, ensure you have the following tools installed:

### Required Tools

- **Rust**: Install via [rustup](https://rustup.rs/)
- **Go**: Install via [golang.org](https://golang.org/dl/) (required for full test suite)
- **Scarb**: Install via [docs.swmansion.com/scarb](https://docs.swmansion.com/scarb/) (for Cairo formatting)
- **Prettier**: Install via `npm install -g prettier` or `yarn global add prettier` (for markdown/YAML formatting)

### Verify Installation

```bash
# Check Rust installation
cargo --version
rustc --version

# Check Go installation
go version

# Check Scarb installation
scarb --version

# Check Prettier installation
prettier --version
```

## Development Setup

### 1. Clone the Repository

```bash
git clone https://github.com/cartridge-gg/cainome.git
cd cainome
```

### 2. Build the Project

```bash
# Build all crates
cargo build --workspace

# Build with all features
cargo build --workspace --all-features
```

## Testing Locally

### Running the Full Test Suite

Before pushing any changes, always run the complete test suite to ensure everything works correctly:

```bash
# Run all tests (Rust + Go examples)
./bin/test
```

This command will:

1. Run all Rust tests across the workspace
2. Execute Go examples to verify generated code works correctly
3. Test the serde implementation with various examples

### Comprehensive Local Testing

For a more thorough local test that includes all checks, you can use the comprehensive test script:

```bash
# Run comprehensive local tests
./bin/test-local
```

This script will:

1. Check if all required tools are installed (Rust, Go, Scarb, Prettier)
2. Run linting and formatting checks
3. Build all crates with all features
4. Run all Rust tests
5. Execute Go examples (if Go is installed)
6. Test the CLI functionality
7. Test individual crates
8. Check for uncommitted changes
9. Provide a summary and next steps

**Note**: Go is required for the full test suite. If Go is not installed, some tests will be skipped with a warning.

### Individual Test Commands

You can also run specific test categories:

```bash
# Rust tests only
cargo test --workspace --all-features

# Run specific examples
cargo run --example structs --all-features
cargo run --example alias_skip --all-features
cargo run --example components_events --all-features

# Test specific crates
cargo test -p cainome-parser
cargo test -p cainome-rs
cargo test -p cainome-cairo-serde
```

### Linting and Formatting

Use the provided linting scripts to ensure code quality:

```bash
# Run all linting checks
./bin/lint

# Run specific linting checks
./bin/lint --rust      # Rust formatting and clippy
./bin/lint --cairo     # Cairo formatting
./bin/lint --prettier  # Markdown/YAML formatting

# Check-only mode (doesn't fix, just reports issues)
./bin/lint --check-only

# Lint specific files
./bin/lint --files src/lib.rs crates/parser/src/lib.rs
```

## Release System

Cainome uses a release system with interdependent crates. Understanding the dependency chain is crucial for proper releases.

### Crate Dependencies

The crates have the following dependency relationships:

```
cainome-parser (independent)
    ↓
cainome-cairo-serde (independent)
    ↓
cainome-rs (depends on parser + cairo-serde)
    ↓
cainome-rs-macro (depends on rs + parser + cairo-serde)
```

### Release Process

#### 1. Single Crate Release

To release a single crate (e.g., `cainome-parser`):

1. **Update the version** in the crate's `Cargo.toml`:

   ```toml
   [package]
   name = "cainome-parser"
   version = "0.5.2"  # Increment version
   ```

2. **Update workspace dependencies** in the root `Cargo.toml`:

   ```toml
   [workspace.dependencies]
   cainome-parser = { version = "0.5.2", path = "crates/parser" }
   ```

3. **Create and push the tag**:
   ```bash
   git add .
   git commit -m "release(cainome-parser): bump to v0.5.2"
   git tag -a cainome-parser/v0.5.2 -m "cainome-parser v0.5.2"
   git push origin main --follow-tags
   ```

#### 2. Full Cainome Release

To release the entire cainome project:

1. **Ensure all crate versions are updated** in their respective `Cargo.toml` files.

2. **Update workspace version** in the root `Cargo.toml`.

   ```toml
   [package]
   name = "cainome"
   version = "0.10.0"
   ```

3. **Create and push the tag**:
   ```bash
   git add .
   git commit -m "release(cainome): bump to v0.10.0"
   git tag -a cainome/v0.10.0 -m "cainome v0.10.0"
   git push origin main --follow-tags
   ```

#### 3. StarkNet Dependency Updates

When the `starknet` dependency needs updating, you must follow the dependency chain order to avoid breaking builds:

1. **Update the workspace dependency** in the root `Cargo.toml`:

   ```toml
   [workspace.dependencies]
   starknet = "0.17"  # Update version
   ```

2. **Update and publish crates in dependency order**:
   - Update and test independent crates, then publish them (`cainome-parser`, `cainome-cairo-serde`).
   - Update and test dependent crates, then publish them (`cainome-rs`, `cainome-rs-macro`).
   - Update and test the root Cainome crate, then publish it (`cainome`).

### Release Workflow

The `.github/workflows/release.yml` workflow automatically handles:

1. **Tag parsing**: Extracts crate name and version from Git tags
2. **Branch validation**: Ensures releases are from the main branch
3. **Version verification**: Checks that the tag version matches `Cargo.toml`
4. **Multi-platform builds**: Builds on Ubuntu, Windows, and macOS
5. **Crates.io publishing**: Publishes to crates.io using the API token

To release, you must push a tag to the repository with the following format:

- **Single crate**: `cainome-parser/v0.5.2`
- **Full project**: `cainome/v0.9.1`

The changes must be done in the `main` branch.

```bash
git tag -a cainome-parser/v0.5.2 -m "cainome-parser v0.5.2"
git push origin main --follow-tags
```

### Tag Format

The release system supports two tag formats:

- **Single crate**: `cainome-parser/v0.5.2`
- **Full project**: `cainome/v0.9.1`

### Example Release Scenarios

#### Scenario 1: Bug Fix in Parser

```bash
# 1. Update version in crates/parser/Cargo.toml
# 2. Update workspace dependency in root Cargo.toml
# 3. Test changes
./bin/test

# 4. Commit and tag
git add .
git commit -m "fix: resolve parser issue with complex types"
git tag cainome-parser/v0.5.2
git push origin main
git push origin cainome-parser/v0.5.2
```

#### Scenario 2: New Feature in RS Crate

```bash
# 1. Update version in crates/rs/Cargo.toml
# 2. Update workspace dependency in root Cargo.toml
# 3. Test changes
./bin/test

# 4. Commit and tag
git add .
git commit -m "feat: add support for new Cairo types"
git tag cainome-rs/v0.4.1
git push origin main
git push origin cainome-rs/v0.4.1
```

#### Scenario 3: Major Version Release

```bash
# 1. Update all crate versions
# 2. Update workspace dependencies
# 3. Test thoroughly
./bin/test

# 4. Commit and tag
git add .
git commit -m "feat: major release with breaking changes"
git tag cainome/v1.0.0
git push origin main
git push origin cainome/v1.0.0
```
