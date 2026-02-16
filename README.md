# agentfiles

> [!WARNING]
> This project is in early development and is **not production-ready**. APIs, CLI flags, manifest format, and behavior may change without notice between versions. Use at your own risk.

A CLI that installs agent files (skills, commands, agents) across multiple agentic coding providers from a unified `agentfiles.json` manifest.

Write your agent files once, install them everywhere -- Claude Code, OpenCode, Codex, and Cursor.

## Supported Providers

| Provider | Skills | Commands | Agents |
|---|---|---|---|
| Claude Code | Yes | Yes | Yes |
| OpenCode | Yes | Yes | Yes |
| Codex | Yes | - | - |
| Cursor | Yes | Yes | Yes |

Run `agentfiles matrix` to see this table at any time.

## Installation

### Shell script (Linux / macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/leodiegues/agentfiles/main/install.sh | sh
```

You can pin a version with `VERSION=vX.Y.Z` and change the install location with `INSTALL_DIR`:

```sh
VERSION=vX.Y.Z INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/leodiegues/agentfiles/main/install.sh | sh
```

### Cargo

```sh
cargo install agentfiles
```

### Build from source

```sh
git clone https://github.com/leodiegues/agentfiles.git
cd agentfiles
cargo build --release
# Binary is at target/release/agentfiles
```

## Quick Start

```sh
# 1. Initialize a manifest in your project
agentfiles init

# 2. Install agent files from a remote repository
agentfiles install github.com/org/shared-agents

# 3. Or install from a local directory
agentfiles install ../my-agents

# 4. Re-install all saved dependencies
agentfiles install
```

The first `install <source>` scans the source for agent files, installs them into each provider's expected directory structure, and saves the source as a dependency in `agentfiles.json`. Running `agentfiles install` with no arguments re-installs all saved dependencies.

## Commands

### `agentfiles init`

Create a new `agentfiles.json` manifest. The manifest starts empty -- add dependencies with `agentfiles install <source>`.

```
agentfiles init [PATH] [OPTIONS]
```

| Option | Description | Default |
|---|---|---|
| `PATH` | Directory to create the manifest in | `.` (current directory) |
| `-n, --name <NAME>` | Package name | Inferred from directory name |

```sh
# Initialize in the current directory
agentfiles init

# Initialize in a specific directory with a custom name
agentfiles init ./my-agents --name "my-custom-agents"
```

### `agentfiles install`

Install agent files from dependencies in `agentfiles.json`, or add a new source.

```
agentfiles install [SOURCE] [OPTIONS]
```

| Option | Description | Default |
|---|---|---|
| `SOURCE` | Local path or git URL. If omitted, installs all deps from `agentfiles.json` | |
| `-s, --scope <SCOPE>` | Installation scope: `project` or `global` | `project` |
| `-p, --providers <PROVIDERS>` | Target providers (comma-separated) | All providers |
| `--strategy <STRATEGY>` | File placement: `copy` or `link` (symlink) | Per-dependency manifest setting |
| `--pick <ITEMS>` | Cherry-pick specific items by name (comma-separated) | |
| `--no-save` | Do not save the source to `agentfiles.json` after installing | |
| `--dry-run` | Preview what would be installed without making changes | |
| `--root <ROOT>` | Project root directory | `.` |

**Install all dependencies from the manifest:**

```sh
# Install everything listed in agentfiles.json
agentfiles install

# Install to specific providers only
agentfiles install -p claude-code,cursor

# Install globally (user-wide, not project-scoped)
agentfiles install -s global
```

**Add and install from a new source:**

```sh
# Install from a local directory (auto-saves to agentfiles.json)
agentfiles install ./my-agents

# Install from a GitHub repository
agentfiles install github.com/org/repo

# Install from a specific branch or tag
agentfiles install github.com/org/repo@v1.0

# Install from a full URL
agentfiles install https://github.com/org/repo.git@main

# Use symlinks instead of copies
agentfiles install github.com/org/repo --strategy link

# Cherry-pick specific items
agentfiles install github.com/org/repo --pick skills/review,commands/deploy

# Install without saving to agentfiles.json
agentfiles install github.com/org/repo --no-save

# Preview what would be installed
agentfiles install github.com/org/repo --dry-run

# Install from a specific directory into a specific project root
agentfiles install ./my-agents --root ./my-project
```

The `--pick` flag supports kind-prefixed names (`skills/review`, `commands/deploy`) or plain names (`review`) that match any kind.

Provider names for `-p` are: `claude-code`, `opencode`, `codex`, `cursor`.

### `agentfiles scan`

Scan a local directory or remote git repository for agent files without installing them. Useful for previewing what would be discovered.

```
agentfiles scan [SOURCE]
```

| Option | Description | Default |
|---|---|---|
| `SOURCE` | Local path or git URL | `.` (current directory) |

```sh
# Scan the current directory
agentfiles scan

# Scan a remote repository
agentfiles scan github.com/org/repo@main
```

### `agentfiles list`

List dependencies from `agentfiles.json`.

```
agentfiles list [ROOT]
```

| Option | Description | Default |
|---|---|---|
| `ROOT` | Project root directory | `.` (current directory) |

```sh
agentfiles list
```

### `agentfiles remove`

Remove a dependency from `agentfiles.json`.

```
agentfiles remove <SOURCE> [OPTIONS]
```

| Option | Description | Default |
|---|---|---|
| `SOURCE` | Source to remove (matches by normalized URL) | |
| `--clean` | Also delete installed files from provider directories | |
| `-s, --scope <SCOPE>` | Installation scope used when installing (for `--clean`) | `project` |
| `-p, --providers <PROVIDERS>` | Target providers to clean (for `--clean`) | All providers |
| `--root <ROOT>` | Project root directory | `.` |

```sh
# Remove a dependency
agentfiles remove github.com/org/repo

# Remove and clean up installed files
agentfiles remove github.com/org/repo --clean
```

### `agentfiles matrix`

Display the provider compatibility matrix showing which file kinds each provider supports.

```sh
agentfiles matrix
```

## Manifest Format

The `agentfiles.json` manifest lists dependencies (remote or local sources) that provide agent files. It lives in your project root, similar to `package.json`.

```json
{
  "name": "my-project",
  "version": "0.0.1",
  "description": "My project agent files",
  "author": "Your Name",
  "repository": "https://github.com/org/my-project",
  "dependencies": [
    "github.com/org/shared-agents",
    {
      "source": "github.com/org/more-agents",
      "ref": "v2.0",
      "pick": ["skills/review", "commands/deploy"],
      "strategy": "Link",
      "paths": [
        { "path": "prompts", "kind": "skill" },
        { "path": "macros", "kind": "command" }
      ]
    }
  ]
}
```

### Fields

| Field | Required | Description |
|---|---|---|
| `name` | Yes | Package name |
| `version` | No | Package version (defaults to `"0.0.1"`) |
| `description` | No | Short description |
| `author` | No | Author name |
| `repository` | No | Source repository URL |
| `dependencies` | No | Array of dependency sources (omitted when empty) |

### Dependency formats

Dependencies can be a simple string or a detailed object:

**Simple form** -- just a URL or local path:

```json
"github.com/org/repo"
```

**Detailed form** -- with options:

| Field | Required | Description |
|---|---|---|
| `source` | Yes | URL or local path |
| `ref` | No | Git ref (branch, tag, or commit) to check out |
| `pick` | No | Cherry-pick specific items by name |
| `strategy` | No | Override placement strategy: `Copy` (default) or `Link` (symlink) |
| `paths` | No | Custom directory-to-kind mappings (replaces default convention) |

Each entry in `paths` has a `path` (relative to source) and a `kind` (`skill`, `command`, or `agent`).

## Remote Git Sources

agentfiles can install directly from git repositories. Supported URL formats:

| Format | Example |
|---|---|
| Shorthand | `github.com/org/repo` |
| Shorthand + ref | `github.com/org/repo@v1.0` |
| HTTPS | `https://github.com/org/repo.git` |
| SSH | `git@github.com:org/repo.git` |

Recognized shorthand hosts: `github.com`, `gitlab.com`, `bitbucket.org`, `codeberg.org`, `sr.ht`.

Remote repositories are cached locally. Subsequent installs from the same URL will fetch updates instead of re-cloning.

The remote repository should either contain an `agentfiles.json` manifest or use the standard directory structure so that files can be auto-discovered.

## File Conventions

agentfiles expects agent files to follow a specific directory structure:

```
skills/
  code-review/
    SKILL.md          # Each skill is a directory with a SKILL.md file
  refactor/
    SKILL.md
commands/
  deploy.md           # Each command is a .md file
  test.md
agents/
  security.md         # Each agent is a .md file
  performance.md
```

- **Skills** -- A directory containing a `SKILL.md` file (e.g., `skills/code-review/SKILL.md`).
- **Commands** -- A `.md` file in the `commands/` directory (e.g., `commands/deploy.md`).
- **Agents** -- A `.md` file in the `agents/` directory (e.g., `agents/security.md`).

This structure is used by `agentfiles scan` for discovery and `agentfiles install` for scanning sources.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, testing conventions, and how to submit changes.

## License

[MIT](LICENSE)
