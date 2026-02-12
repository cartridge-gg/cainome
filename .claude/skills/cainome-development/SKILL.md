---
name: cainome-development
description: Contributor workflow for cartridge-gg/cainome. Use when implementing or reviewing Cairo ABI parsing, Rust codegen, CLI, or workspace-wide lint/test updates in this repository.
---

# Cainome Development

Use this skill to work on `cartridge-gg/cainome` with full-feature checks across parser, serde, rs-macro, and CLI components.

## Core Workflow

1. Build baseline targets:
   - `cargo build --workspace`
   - `cargo build --workspace --all-features`
2. Run full tests for final validation:
   - `./bin/test`
3. Run lint/format checks:
   - `./bin/lint --all --check-only`
   - `cargo fmt`
4. Use targeted runs while iterating:
   - `cargo test -p cainome-parser`
   - `cargo test -p cainome-rs`
   - `cargo run --example <example_name> --all-features`

## Dependency Update Caution

- Follow crate dependency order when touching `starknet`-related versions.
- Validate dependent crates before bumping root crate versions.

## PR Checklist

- Include both workspace and targeted validation commands in PR notes.
- Mention feature flags used when reproducing issues.
- Keep generated outputs and formatter changes intentional.
