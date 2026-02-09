# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**agentfiles** is a CLI tool that unifies installation of agent files across multiple agentic coding providers (e.g., Claude Code). It reads a manifest (`agentfiles.json`) and installs skills, agents, and commands to the correct locations for each provider.

The project is being **migrated from Python to Rust** as a learning exercise. The original Python implementation is preserved in `src-deprecated/` as a reference. All new development happens in Rust under `src/`.

## Build Commands

```bash
cargo build              # Build the project
cargo run                # Build and run the binary
cargo test               # Run all tests
cargo test test_name     # Run a single test by name
cargo clippy             # Lint (if clippy is installed)
cargo fmt                # Format code
cargo fmt -- --check     # Check formatting without modifying
```

## Architecture

### Core Concepts

- **Manifest** (`agentfiles.json`): A JSON file declaring a package of agent files with metadata (name, version, author) and a list of file mappings.
- **FileKind**: Categorizes files as `Skill`, `Agent`, or `Command` — determines the target subdirectory (e.g., `.claude/skills/`).
- **FileScope**: `Project` (relative to project root) or `Global` (relative to `$HOME`).
- **FileStrategy** (not yet ported): `Copy` or `Link` — how files are placed at the target.
- **Installers**: Provider-specific implementations that resolve target directories and perform file installation. Each installer maps `(FileScope, FileKind)` to a filesystem path.

### Module Structure (Rust)

- `types.rs` — Enums: `FileScope`, `FileKind`
- `manifest.rs` — `Manifest` and `FileMapping` structs, JSON loading via serde
- `lib.rs` — Module re-exports
- `main.rs` — Binary entry point (stub)

### Python Reference (`src-deprecated/`)

The deprecated Python code contains the complete design intent, including parts not yet ported:

- `installers/protocol.py` — `AgentFileInstaller` trait/protocol with `get_target_dir()` and `install()` methods
- `installers/claude_code.py` — Claude Code installer mapping FileKind to `.claude/{skills,commands,agents}` paths
- `types.py` — Includes `FileStrategy` enum (COPY/LINK) not yet in Rust
- `manifest.py` — Includes `repository` field not yet in Rust's `Manifest` struct

### Target Directory Convention (Claude Code)

```
Global: $HOME/.claude/{skills,commands,agents}/
Project: <project_root>/.claude/{skills,commands,agents}/
```

## Known Issues

- `Cargo.toml` uses `edition = "2024"` which is non-standard; Rust editions are `2015`, `2018`, `2021`, `2024` (stabilized in Rust 1.85+, requires recent toolchain).
- serde dependency needs `features = ["derive"]` for `#[derive(Serialize, Deserialize)]` to compile on `Manifest`.
- `FileMapping` derives `Clone, Debug` but not `Serialize, Deserialize`, so `Manifest` deserialization will fail since `files: Vec<FileMapping>`.
