# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**agentfiles** is a CLI tool that unifies installation of agent files across multiple agentic coding providers. It reads a manifest (`agentfiles.json`) and installs skills, agents, and commands to the correct locations for each provider.

## Build Commands

```bash
cargo build              # Build the project
cargo run                # Build and run the binary
cargo test               # Run all tests
cargo test test_name     # Run a single test by name
cargo clippy             # Lint
cargo fmt                # Format code
cargo fmt -- --check     # Check formatting without modifying
```

## Architecture

### Core Concepts

- **Manifest** (`agentfiles.json`): A JSON file declaring a package of agent files with metadata (name, version, author, repository) and a list of file mappings.
- **FileKind**: Categorizes files as `Skill`, `Agent`, or `Command` — determines the target subdirectory.
- **FileScope**: `Project` (relative to project root) or `Global` (relative to `$HOME`).
- **FileStrategy**: `Copy` (default) or `Link` (symlink) — how files are placed at the target.
- **AgentProvider**: `ClaudeCode`, `OpenCode`, `Codex`, `Cursor` — each with a compatibility matrix determining which FileKinds they support.

### Provider Compatibility Matrix

| Feature  | Claude Code | OpenCode | Codex | Cursor |
|----------|:-----------:|:--------:|:-----:|:------:|
| Skills   | Yes         | Yes      | Yes   | Yes    |
| Commands | Yes         | Yes      | No    | Yes    |
| Agents   | Yes         | Yes      | No    | Yes    |

### Target Directory Convention

**Project scope** (relative to project root):

| Provider    | Skills               | Commands               | Agents               |
|-------------|----------------------|------------------------|----------------------|
| Claude Code | `.claude/skills/`    | `.claude/commands/`    | `.claude/agents/`    |
| OpenCode    | `.opencode/skills/`  | `.opencode/commands/`  | `.opencode/agents/`  |
| Codex       | `.agents/skills/`    | N/A                    | N/A                  |
| Cursor      | `.cursor/skills/`    | `.cursor/commands/`    | `.cursor/agents/`    |

**Global scope** (`$HOME`-relative):

| Provider    | Skills                       | Commands                       | Agents                       |
|-------------|------------------------------|--------------------------------|------------------------------|
| Claude Code | `~/.claude/skills/`          | `~/.claude/commands/`          | `~/.claude/agents/`          |
| OpenCode    | `~/.config/opencode/skills/` | `~/.config/opencode/commands/` | `~/.config/opencode/agents/` |
| Codex       | `~/.agents/skills/`          | N/A                            | N/A                          |
| Cursor      | `~/.cursor/skills/`          | `~/.cursor/commands/`          | `~/.cursor/agents/`          |

### Module Structure

- `types.rs` — Core enums: `FileScope`, `FileKind`, `FileStrategy`, `AgentProvider` with compatibility matrix
- `provider.rs` — Provider target directory resolution with proper `$HOME` expansion via `dirs` crate
- `manifest.rs` — `Manifest` and `FileMapping` structs, JSON loading/saving via serde
- `scanner.rs` — Auto-discovery of agent files from directory structures
- `installer.rs` — File installation (copy/symlink) to provider-specific directories
- `git.rs` — Remote git repository support: URL detection, `@ref` parsing, clone/cache management
- `cli.rs` — CLI argument parsing with clap (install, init, scan, matrix commands)
- `main.rs` — Binary entry point wiring CLI to library functions
- `lib.rs` — Module re-exports

### CLI Commands

- `agentfiles install [source]` — Install files from a manifest or remote git repo
  - `source` can be a local path, directory, or git URL (e.g., `github.com/org/repo@v1.0`)
  - `--scope project|global` — Installation scope (default: project)
  - `--providers claude-code,opencode,codex,cursor` — Target providers (default: all)
  - `--strategy copy|link` — Override file placement strategy
  - `--root <path>` — Project root directory
- `agentfiles init [path]` — Initialize a new agentfiles.json (auto-discovers existing files)
- `agentfiles scan [path]` — Scan a directory for agent files and optionally write manifest
- `agentfiles matrix` — Display the provider compatibility matrix

### Manifest Format

```json
{
  "name": "my-team-skills",
  "version": "0.1.0",
  "description": "Shared agent files",
  "author": "Team Name",
  "repository": "https://github.com/org/repo",
  "files": [
    {
      "path": "skills/review/SKILL.md",
      "kind": "Skill"
    },
    {
      "path": "commands/deploy.md",
      "kind": "Command",
      "strategy": "Link"
    }
  ]
}
```

The `strategy` field is optional and defaults to `"Copy"`. It is omitted from JSON output when set to the default.

### Design Decisions

- **Kind-only file mappings**: The manifest declares files with just their `kind` (Skill/Agent/Command). The CLI handles multi-provider routing via the compatibility matrix — one skill file gets installed to all compatible providers automatically.
- **Scanner auto-discovery**: The scanner checks known provider directory prefixes (`.claude/`, `.opencode/`, `.agents/`, `.cursor/`, `.codex/`) as well as bare `skills/`, `commands/`, `agents/` directories. It deduplicates by name+kind.
- **Home directory resolution**: Uses the `dirs` crate for proper `$HOME` resolution instead of literal `~`.
- **Remote git installation**: Shells out to the system `git` CLI (inherits user's SSH/auth config, zero new dependencies). Clones are cached in `~/.cache/agentfiles/<url-hash>/` and updated on subsequent installs. Supports `@ref` syntax for version pinning (e.g., `github.com/org/repo@v1.0`). Two modes: repo with `agentfiles.json` uses the manifest; repo without it auto-discovers files via the scanner.
