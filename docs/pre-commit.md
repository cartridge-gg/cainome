# Pre-commit Hooks Setup

This document provides detailed information about the pre-commit hooks configuration for this project.

## Overview

Pre-commit hooks ensure code quality and consistency by running the same linting and formatting checks as our CI pipeline, but only on staged files for faster feedback during development.

## Quick Start

```bash
# One-command setup
make setup-pre-commit
```

This will:

1. Install pre-commit (if not already installed)
2. Install the git hooks
3. Check for required dependencies

## Manual Setup

If you prefer manual setup or the script doesn't work in your environment:

```bash
# Install pre-commit
pip install pre-commit
# or
brew install pre-commit
# or
conda install -c conda-forge pre-commit

# Install the hooks
pre-commit install
```

## Hook Configuration

Our `.pre-commit-config.yaml` includes the following hooks:

### Formatting Hooks (Auto-fix)

- **prettier**: Formats Markdown and YAML files
- **cargo fmt**: Formats Rust code
- **scarb fmt**: Formats Cairo contract code
- **trailing-whitespace**: Removes trailing whitespace
- **end-of-file-fixer**: Ensures files end with a newline

### Linting Hooks (Check-only)

- **cargo clippy**: Rust linting with CI-level strictness
- **cargo doc**: Documentation checks (manual stage only)
- **check-yaml**: YAML syntax validation
- **check-toml**: TOML syntax validation
- **check-merge-conflict**: Detects merge conflict markers

## Usage

### Automatic (Recommended)

Hooks run automatically when you commit:

```bash
git add .
git commit -m "your message"
# Hooks run automatically and may modify files
# If files are modified, you'll need to add them and commit again
```

### Manual Execution

```bash
# Run all hooks on all files
pre-commit run --all-files

# Run all hooks on staged files only
pre-commit run

# Run specific hook
pre-commit run cargo-fmt
pre-commit run prettier

# Run hooks that normally only run manually
pre-commit run --hook-stage manual
```

### Skipping Hooks

```bash
# Skip all hooks (not recommended)
git commit --no-verify

# Skip specific hooks using SKIP environment variable
SKIP=cargo-clippy git commit -m "message"
```

## Integration with Make

We provide convenient Make targets:

```bash
make setup-pre-commit  # Install and setup pre-commit
make lint              # Run all linting checks (same as CI)
make fmt               # Format all code
make check             # Run pre-commit on all files
```

## CI Integration

We have a dedicated pre-commit CI workflow (`.github/workflows/pre-commit.yml`) that:

- Runs the same pre-commit hooks in CI
- Ensures consistency between local and CI environments
- Provides detailed diff output on failures

This complements our existing lint workflow and provides an additional layer of verification.

## Troubleshooting

### Pre-commit not found

```bash
# Make sure pre-commit is in your PATH
which pre-commit

# If using a virtual environment, activate it
source .venv/bin/activate
```

### Prettier issues

```bash
# Install yarn globally if needed
npm install -g yarn
yarn global add prettier
```

### Scarb not found

```bash
# Install scarb for Cairo contract formatting
# Follow instructions at: https://docs.swmansion.com/scarb/
```

### Cargo issues

```bash
# Make sure Rust is installed and up to date
rustup update
rustup component add rustfmt clippy
```

### Hook failures

```bash
# See what failed
pre-commit run --all-files --verbose

# Update hook versions
pre-commit autoupdate

# Clear cache if needed
pre-commit clean
```

## Configuration Details

### File Patterns

- Rust files: `\.rs$`
- Markdown files: `\.md$` (excludes `CLAUDE.md`)
- YAML files: `\.(yaml|yml)$`
- Cairo files: `^contracts/.*\.cairo$`

### Hook Stages

- **commit** (default): Runs on `git commit`
- **manual**: Only runs when explicitly called

### Performance Optimizations

- Hooks only run on relevant file types
- Rust doc checks are manual-only (expensive)
- Scarb formatting only runs on Cairo files in contracts/
- Uses `pass_filenames: false` for workspace-level tools

## Benefits

1. **Consistency**: Same checks as CI, but faster feedback
2. **Automatic fixes**: Many issues are fixed automatically
3. **Selective execution**: Only runs on changed files
4. **Developer experience**: Catches issues before push
5. **CI efficiency**: Fewer CI failures due to formatting issues

## Customization

To modify the configuration:

1. Edit `.pre-commit-config.yaml`
2. Test your changes: `pre-commit run --all-files`
3. Update documentation if needed

Common customizations:

- Add new file types to existing hooks
- Add new hooks from the [pre-commit hooks registry](https://pre-commit.com/hooks.html)
- Modify arguments for existing hooks
- Change hook stages or file patterns
