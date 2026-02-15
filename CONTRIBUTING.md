# Contributing to agentfiles

Thanks for your interest in contributing! This guide covers everything you need to get started.

## Prerequisites

- **Rust** -- nightly toolchain (the project uses the 2024 edition, which requires nightly). Install via [rustup](https://rustup.rs/).
- **Git** -- required at runtime (agentfiles shells out to `git` for remote source support).
- **just** (optional) -- a command runner for convenience recipes. Install via `cargo install just` or your system package manager.

## Getting Started

```sh
git clone https://github.com/leodiegues/agentfiles.git
cd agentfiles
cargo build
cargo test
```

If all tests pass, you're ready to go.

## Development Commands

The project uses standard Cargo commands. A `justfile` is also provided for convenience.

| Task | Cargo | Just |
|---|---|---|
| Build | `cargo build` | `just build` |
| Run | `cargo run -- <args>` | `just run <args>` |
| Test (all) | `cargo test` | `just test` |
| Test (single) | `cargo test <name>` | -- |
| Lint | `cargo clippy -- -D warnings` | `just lint` |
| Format | `cargo fmt` | `just fmt` |
| Format check | `cargo fmt -- --check` | -- |
| Type-check only | `cargo check` | `just check` |
| Full CI suite | See below | `just ci` |
| Clean | `cargo clean` | `just clean` |

### Running CI Locally

Before submitting a PR, run the full CI suite to catch issues early:

```sh
just ci
```

Or manually:

```sh
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
cargo build
```

All four checks must pass. CI runs these across Linux, macOS, and Windows.

## Project Structure

```
src/
  lib.rs         -- pub mod re-exports only
  types.rs       -- Core enums (FileScope, FileKind, FileStrategy, AgentProvider)
  provider.rs    -- Provider directory layout resolution, compatibility matrix
  manifest.rs    -- Manifest/FileMapping structs, JSON load/save
  scanner.rs     -- Auto-discovery of agent files from directory structures
  installer.rs   -- File installation (copy/symlink) to provider directories
  git.rs         -- Remote git URL detection, parsing, clone/cache
  cli.rs         -- CLI argument parsing (clap derive)
  commands.rs    -- Command handlers (cmd_install, cmd_init, etc.)
  main.rs        -- Binary entry point
```

Dependency flow: `types` <- `provider`, `manifest` <- `scanner`, `installer`. `git` and `cli` are standalone. `main` and `commands` wire everything together.

For a comprehensive reference on module internals, naming conventions, and design principles, see [AGENTS.md](AGENTS.md).

## Code Style

### Formatting

Use default `rustfmt` settings. Run `cargo fmt` before committing.

### Import ordering

Three groups, separated by blank lines:

```rust
// 1. Standard library
use std::fs;
use std::path::PathBuf;

// 2. External crates
use anyhow::{Context, Result};

// 3. Crate-internal
use crate::types::AgentProvider;
```

### Error handling

The project uses `anyhow` exclusively. No custom error types. Prefer `.context()` / `.with_context()` over `.unwrap()`. Error messages should be lowercase with no trailing punctuation.

```rust
let content = std::fs::read_to_string(path)
    .context("failed to read manifest")?;
```

### Naming

- Functions: `snake_case` (`scan_agent_files`, `cmd_install`)
- Types/Enums: `PascalCase` (`FileMapping`, `AgentProvider`)
- Constants: `UPPER_SNAKE_CASE` (`KIND_DIRS`)
- Modules: `snake_case`, flat structure (all in `src/`)

### Platform-specific code

Gate with `#[cfg(unix)]` / `#[cfg(windows)]`. See `installer.rs` for examples.

## Testing

### Where tests live

Tests are inline `#[cfg(test)] mod tests` blocks within each module. There is no separate `tests/` directory and no fixture files.

### Running tests

```sh
# All tests
cargo test

# Single test by name substring
cargo test save_and_roundtrip

# Nested module path
cargo test tests::load_manifest::from_dir

# Exact match
cargo test -p agentfiles save_and_roundtrip -- --exact
```

### Test patterns

- Most tests return `Result<()>` and use `?` for propagation.
- Use `tempfile::TempDir` for filesystem tests. Create test data inline.
- Symlink-specific tests are gated with `#[cfg(unix)]`.
- Descriptive `snake_case` names without a `test_` prefix.

## Pull Requests

1. Fork the repository and create a branch from `main`.
2. Make your changes, following the code style above.
3. Ensure `just ci` (or the equivalent manual commands) passes locally.
4. Open a pull request against `main`.

CI will automatically run formatting checks, clippy lints, and tests across Linux, macOS, and Windows. All checks must pass before merging.

Keep commits focused and write clear commit messages that explain the _why_ behind the change.
