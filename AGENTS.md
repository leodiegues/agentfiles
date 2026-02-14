# AGENTS.md

Guidance for AI coding agents operating in this repository.

## Project Summary

**agentfiles** is a Rust CLI tool that installs agent files (skills, agents, commands) across multiple agentic coding providers (Claude Code, OpenCode, Codex, Cursor) from a unified `agentfiles.json` manifest. Rust 2024 edition, purely synchronous, no unsafe code.

## Build / Lint / Test Commands

```bash
cargo build                    # Build the project
cargo run                      # Build and run the binary
cargo test                     # Run ALL tests
cargo test <name>              # Run a single test by substring match
cargo test <name> -- --exact   # Run a single test by exact name
cargo clippy -- -D warnings    # Lint (CI treats warnings as errors)
cargo fmt                      # Format code
cargo fmt -- --check           # Check formatting without modifying
```

CI runs on every PR: `fmt --check`, `clippy -D warnings`, `test`, `build` across Linux/macOS/Windows. All four must pass.

## Module Structure

```
src/
  lib.rs         -- pub mod re-exports only
  types.rs       -- Core enums (FileScope, FileKind, FileStrategy, AgentProvider)
  provider.rs    -- Provider directory layout resolution, compatibility matrix
  manifest.rs    -- Manifest/FileMapping structs, JSON load/save (serde)
  scanner.rs     -- Auto-discovery of agent files from directory structures
  installer.rs   -- File installation (copy/symlink) to provider directories
  git.rs         -- Remote git: URL detection, @ref parsing, clone/cache
  cli.rs         -- CLI argument parsing (clap derive)
  main.rs        -- Binary entry point, command handlers (cmd_install, cmd_init, etc.)
```

Dependency flow: `types` <- `provider`, `manifest` <- `scanner`, `installer`. `git` and `cli` are standalone. `main` wires everything together via the lib crate.

## Code Style Guidelines

### Formatting

Default `rustfmt` settings (no `.rustfmt.toml`). No configuration overrides. Just run `cargo fmt`.

### Import Ordering

Three groups separated by blank lines:

```rust
// 1. Standard library
use std::fs;
use std::path::{Path, PathBuf};

// 2. External crates
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// 3. Crate-internal modules
use crate::manifest::FileMapping;
use crate::types::{AgentProvider, FileKind};
```

Exception: `types.rs` has no crate-internal imports so groups 1 and 2 may appear together.

### Naming Conventions

- **Functions**: `snake_case`. Public: `scan_agent_files`, `load_manifest`. Private helpers: `scan_kind_dir`, `normalize_url`. Command handlers in main: `cmd_install`, `cmd_init`.
- **Types/Structs/Enums**: `PascalCase`. Examples: `FileMapping`, `InstallResult`, `AgentProvider`.
- **Enum variants**: `PascalCase`. Examples: `FileScope::Project`, `FileKind::Skill`.
- **Constants**: `UPPER_SNAKE_CASE`. Example: `const KIND_DIRS: &[(&str, FileKind)]`.
- **Modules**: `snake_case`, flat structure (all in `src/`), no nested module directories.

### Error Handling

Uses `anyhow` exclusively. No custom error types. All `Result` types are `anyhow::Result<T>`. Three patterns:

```rust
// 1. Static context
let content = std::fs::read_to_string(path).context("failed to read manifest")?;

// 2. Dynamic context with formatting
source.canonicalize()
    .with_context(|| format!("failed to resolve: {}", source.display()))?;

// 3. Early-return errors
anyhow::bail!("source file not found: {}", path.display());
```

Error message style: lowercase, no trailing punctuation, descriptive. All `FromStr` impls use `type Err = anyhow::Error`. Never use `unwrap()` in production code -- only in tests.

### Type Conventions

- Derive set for domain types: `#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]`
- Serde: `#[serde(rename_all = "lowercase")]` on enums, `skip_serializing_if` for optional/default fields
- Builder-like pattern on `Manifest`: `Manifest::default().with_name(n).with_files(f)`
- No trait abstractions -- concrete types throughout. Standard traits (`Display`, `FromStr`, `Default`) implemented as needed.

### Platform-Specific Code

Gate with `#[cfg(unix)]` / `#[cfg(windows)]`. Used in `installer.rs` for symlink creation and in tests.

### Output

Uses `println!()` directly for user-facing output. No logging framework, no structured logging, no tracing.

### Rust 2024 Edition Features

Let-chains are used (e.g., in `scanner.rs`):
```rust
if entry_path.is_file()
    && let Some(ext) = entry_path.extension()
    && ext == "md"
{
```

## Test Conventions

### Location

Inline `#[cfg(test)] mod tests` blocks within each module. No separate `tests/` directory, no test fixture files.

### Organization

Two patterns:

```rust
// Flat (types.rs, provider.rs, scanner.rs):
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptive_name() -> Result<()> { ... }
}

// Nested sub-modules for grouping (manifest.rs, git.rs, installer.rs):
#[cfg(test)]
mod tests {
    mod load_manifest {
        use super::super::*;
        #[test]
        fn from_directory() -> Result<()> { ... }
    }
}
```

### Test Patterns

- **Return type**: Most tests return `Result<()>` and use `?` for propagation. Tests checking error cases use `assert!(result.is_err())`.
- **Filesystem**: Use `tempfile::TempDir` for all filesystem tests. Create test data inline -- no fixture files.
- **Platform gates**: Symlink tests use `#[cfg(unix)] #[test]`.
- **Naming**: Descriptive `snake_case`. No mandatory `test_` prefix. Examples: `save_and_roundtrip`, `install_command_skips_codex`, `shorthand_gets_https`.
- **Helpers**: Define local helper functions within test modules (e.g., `setup_skill`, `make_manifest`).

### Running a Single Test

```bash
cargo test save_and_roundtrip              # Match by substring
cargo test tests::load_manifest::from_dir  # Match nested module path
cargo test -p agentfiles save_and_roundtrip -- --exact  # Exact match
```

## Design Principles

1. **Flat module structure** -- all source in `src/*.rs`, no nested directories.
2. **Minimal dependencies** -- 5 runtime deps, 1 dev dep. Shell out to `git` rather than adding a git library.
3. **Single source of truth** -- `ProviderLayout` in `provider.rs` defines all provider config. Adding a provider means adding one layout entry.
4. **Compatibility matrix drives behavior** -- one manifest file, multiple providers handled automatically.
5. **No async** -- entirely synchronous. No tokio/async-std.
6. **Section separators** in larger files use comment banners:
   ```rust
   // ---------------------------------------------------------------------------
   // Internal helpers
   // ---------------------------------------------------------------------------
   ```
