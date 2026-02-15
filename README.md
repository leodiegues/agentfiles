# agentfiles

> **Warning:** This project is in early development (v0.0.1) and is **not production-ready**.
> APIs, CLI flags, manifest format, and behavior may change without notice between versions.
> Use at your own risk.

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

You can pin a version with `VERSION=v0.0.1` and change the install location with `INSTALL_DIR`:

```sh
VERSION=v0.0.1 INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/leodiegues/agentfiles/main/install.sh | sh
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
# 1. Create a directory with some agent files
mkdir -p my-agents/skills/code-review
cat > my-agents/skills/code-review/SKILL.md << 'EOF'
# Code Review
Review code changes for bugs, style issues, and improvements.
EOF

mkdir -p my-agents/commands
cat > my-agents/commands/deploy.md << 'EOF'
# Deploy
Run the deployment pipeline for the current project.
EOF

# 2. Initialize a manifest
cd my-agents
agentfiles init

# 3. Install into the current project for all providers
agentfiles install
```

This creates an `agentfiles.json` manifest from the discovered files and installs them into each provider's expected directory structure.

## Commands

### `agentfiles init`

Initialize a new `agentfiles.json` manifest by auto-discovering agent files in the target directory.

```
agentfiles init [PATH] [OPTIONS]
```

| Option | Description | Default |
|---|---|---|
| `PATH` | Directory to initialize | `.` (current directory) |
| `-n, --name <NAME>` | Package name | Inferred from directory name |

The command scans for `skills/`, `commands/`, and `agents/` directories and generates a manifest with all discovered files.

```sh
# Initialize in the current directory
agentfiles init

# Initialize in a specific directory with a custom name
agentfiles init ./my-agents --name "my-custom-agents"
```

### `agentfiles install`

Install agent files from a manifest or remote git repository into provider directories.

```
agentfiles install [SOURCE] [OPTIONS]
```

| Option | Description | Default |
|---|---|---|
| `SOURCE` | Local path, directory, or git URL | `.` (current directory) |
| `-s, --scope <SCOPE>` | Installation scope: `project` or `global` | `project` |
| `-p, --providers <PROVIDERS>` | Target providers (comma-separated) | All providers |
| `--strategy <STRATEGY>` | File placement: `copy` or `link` (symlink) | Per-file manifest setting |
| `--root <ROOT>` | Project root directory | `.` |

**Install from local directory:**

```sh
# Install all files from the current directory to all providers
agentfiles install

# Install to specific providers only
agentfiles install -p claude-code,cursor

# Install globally (user-wide, not project-scoped)
agentfiles install -s global

# Use symlinks instead of copies
agentfiles install --strategy link

# Install from a specific directory into a specific project root
agentfiles install ./my-agents --root ./my-project
```

**Install from a remote git repository:**

```sh
# Install from a GitHub repository
agentfiles install github.com/org/repo

# Install from a specific branch or tag
agentfiles install github.com/org/repo@v1.0

# Install from a full URL
agentfiles install https://github.com/org/repo.git@main
```

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

### `agentfiles matrix`

Display the provider compatibility matrix showing which file kinds each provider supports.

```sh
agentfiles matrix
```

## Manifest Format

The `agentfiles.json` manifest describes a collection of agent files:

```json
{
  "name": "my-agents",
  "version": "0.1.0",
  "description": "A collection of useful agent skills and commands",
  "author": "Your Name",
  "repository": "https://github.com/org/repo",
  "files": [
    {
      "path": "skills/code-review/SKILL.md",
      "kind": "Skill"
    },
    {
      "path": "commands/deploy.md",
      "kind": "Command"
    },
    {
      "path": "agents/security.md",
      "kind": "Agent",
      "strategy": "link"
    }
  ]
}
```

### Fields

| Field | Required | Description |
|---|---|---|
| `name` | Yes | Package name |
| `version` | Yes | Package version |
| `description` | No | Short description |
| `author` | No | Author name |
| `repository` | No | Source repository URL |
| `files` | Yes | Array of file mappings |

### File mapping fields

| Field | Required | Description |
|---|---|---|
| `path` | Yes | Relative path to the file from the manifest directory |
| `kind` | Yes | File type: `Skill`, `Command`, or `Agent` |
| `strategy` | No | Placement strategy: `copy` (default) or `link` (symlink) |

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

This structure is used by both `agentfiles init` (for auto-discovery) and `agentfiles scan`.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, testing conventions, and how to submit changes.

## License

[MIT](LICENSE)
