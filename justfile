# agentfiles — development task runner
# Run `just` with no arguments to see available recipes.

# ---------------------------------------------------------------------------
# Settings
# ---------------------------------------------------------------------------

set dotenv-load := false

# ---------------------------------------------------------------------------
# Default
# ---------------------------------------------------------------------------

# List available recipes
default:
    @just --list

# ---------------------------------------------------------------------------
# Development
# ---------------------------------------------------------------------------

# Type-check without codegen (fast feedback loop)
check:
    cargo check

# Format code
fmt:
    cargo fmt

# Lint with clippy (warnings are errors, matching CI)
lint:
    cargo clippy -- -D warnings

# Run all tests
test:
    cargo test

# Build debug binary
build:
    cargo build

# Run the binary (pass arguments after --)
run *args:
    cargo run -- {{ args }}

# Remove build artifacts
clean:
    cargo clean

# ---------------------------------------------------------------------------
# CI
# ---------------------------------------------------------------------------

# Run the full CI suite locally (fmt-check, lint, test, build)
ci:
    cargo fmt -- --check
    cargo clippy -- -D warnings
    cargo test
    cargo build

# ---------------------------------------------------------------------------
# Release
# ---------------------------------------------------------------------------

# Prepare a release: validate, run CI, bump version, commit, and tag
[doc("Validate, run CI, bump version, commit, and tag (usage: just release X.Y.Z)")]
release version:
    #!/usr/bin/env bash
    set -euo pipefail

    version="{{ version }}"

    # -- Validate semver format (X.Y.Z) --
    if ! echo "$version" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
        echo "error: version must be semver (X.Y.Z), got: $version" >&2
        exit 1
    fi

    # -- Guard: must be on main branch --
    branch="$(git rev-parse --abbrev-ref HEAD)"
    if [ "$branch" != "main" ]; then
        echo "error: releases must be created from main, currently on: $branch" >&2
        exit 1
    fi

    # -- Guard: working tree must be clean --
    if ! git diff --quiet || ! git diff --cached --quiet; then
        echo "error: working tree is dirty — commit or stash changes first" >&2
        exit 1
    fi

    # -- Guard: tag must not already exist --
    if git rev-parse "v${version}" >/dev/null 2>&1; then
        echo "error: tag v${version} already exists" >&2
        exit 1
    fi

    # -- Run full CI suite --
    echo "Running CI checks..."
    just ci

    # -- Bump version in Cargo.toml --
    echo "Bumping version to ${version}..."
    sed -i.bak -E "s/^version = \"[^\"]+\"/version = \"${version}\"/" Cargo.toml
    rm -f Cargo.toml.bak

    # -- Regenerate Cargo.lock --
    cargo check --quiet

    # -- Commit and tag --
    git add Cargo.toml Cargo.lock
    git commit -m "release: v${version}"
    git tag "v${version}"

    echo ""
    echo "Release v${version} prepared. To publish, run:"
    echo ""
    echo "  git push && git push --tags"
