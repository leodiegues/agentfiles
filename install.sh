#!/bin/sh
# install.sh — Installs the agentfiles CLI binary.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/leodiegues/agentfiles/main/install.sh | sh
#
# Options (via environment variables):
#   INSTALL_DIR  — Where to place the binary (default: ~/.local/bin)
#   VERSION      — Specific version to install (default: latest)

set -eu

REPO="leodiegues/agentfiles"
BINARY="agentfiles"
BASE_URL="https://github.com/${REPO}/releases"

main() {
    os="$(detect_os)"
    arch="$(detect_arch)"
    target="$(resolve_target "$os" "$arch")"
    version="$(resolve_version)"
    install_dir="${INSTALL_DIR:-$HOME/.local/bin}"

    echo "Installing ${BINARY} ${version} for ${target}..."

    url="${BASE_URL}/download/${version}/${BINARY}-${version}-${target}.tar.gz"

    tmp_dir="$(mktemp -d)"
    trap 'rm -rf "$tmp_dir"' EXIT

    echo "Downloading ${url}..."
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "${tmp_dir}/archive.tar.gz"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "${tmp_dir}/archive.tar.gz" "$url"
    else
        echo "Error: curl or wget is required" >&2
        exit 1
    fi

    tar xzf "${tmp_dir}/archive.tar.gz" -C "$tmp_dir"

    mkdir -p "$install_dir"
    cp "${tmp_dir}/${BINARY}-${version}-${target}/${BINARY}" "${install_dir}/${BINARY}"
    chmod +x "${install_dir}/${BINARY}"

    echo ""
    echo "Installed ${BINARY} to ${install_dir}/${BINARY}"

    case ":${PATH}:" in
    *":${install_dir}:"*) ;;
    *)
        echo ""
        echo "WARNING: ${install_dir} is not in your PATH."
        echo "Add it by appending this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo ""
        echo "  export PATH=\"${install_dir}:\$PATH\""
        ;;
    esac

    if command -v "$BINARY" >/dev/null 2>&1; then
        echo ""
        "$BINARY" --version
    fi
}

detect_os() {
    case "$(uname -s)" in
    Linux*) echo "linux" ;;
    Darwin*) echo "macos" ;;
    *)
        echo "Error: unsupported operating system: $(uname -s)" >&2
        echo "Use 'cargo install agentfiles' instead." >&2
        exit 1
        ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
    x86_64 | amd64) echo "x86_64" ;;
    aarch64 | arm64) echo "aarch64" ;;
    *)
        echo "Error: unsupported architecture: $(uname -m)" >&2
        echo "Use 'cargo install agentfiles' instead." >&2
        exit 1
        ;;
    esac
}

resolve_target() {
    os="$1"
    arch="$2"

    case "${os}-${arch}" in
    linux-x86_64) echo "x86_64-unknown-linux-gnu" ;;
    linux-aarch64) echo "aarch64-unknown-linux-gnu" ;;
    macos-x86_64) echo "x86_64-apple-darwin" ;;
    macos-aarch64) echo "aarch64-apple-darwin" ;;
    *)
        echo "Error: unsupported platform: ${os}-${arch}" >&2
        exit 1
        ;;
    esac
}

resolve_version() {
    if [ -n "${VERSION:-}" ]; then
        echo "$VERSION"
        return
    fi

    latest_url="https://api.github.com/repos/${REPO}/releases/latest"

    if command -v curl >/dev/null 2>&1; then
        version="$(curl -fsSL "$latest_url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
    elif command -v wget >/dev/null 2>&1; then
        version="$(wget -qO- "$latest_url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
    else
        echo "Error: curl or wget is required" >&2
        exit 1
    fi

    if [ -z "$version" ]; then
        echo "Error: could not determine latest version. Set VERSION=vX.Y.Z manually." >&2
        exit 1
    fi

    echo "$version"
}

main
