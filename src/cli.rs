use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::types::{AgentProvider, FileScope, FileStrategy};

#[derive(Parser)]
#[command(
    name = "agentfiles",
    about = "Unified agent file installer for Claude Code, OpenCode, Codex, and Cursor",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Install agent files from a manifest or remote git repository
    Install {
        /// Source: local path, directory, or git URL (e.g., github.com/org/repo@v1.0)
        #[arg(default_value = ".")]
        source: String,

        /// Installation scope: project or global
        #[arg(short, long, default_value = "project")]
        scope: FileScope,

        /// Target providers (comma-separated). Defaults to all compatible providers.
        /// Options: claude-code, opencode, codex, cursor
        #[arg(short, long, value_delimiter = ',')]
        providers: Option<Vec<AgentProvider>>,

        /// File placement strategy: copy or link (symlink). Can be overridden per-file in the manifest.
        #[arg(long)]
        strategy: Option<FileStrategy>,

        /// Project root directory (for project scope installations)
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },

    /// Initialize a new agentfiles.json manifest
    Init {
        /// Directory to create the manifest in
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Package name (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Scan a local directory or remote git repository for agent files
    Scan {
        /// Source: local path or git URL (e.g., github.com/org/repo@v1.0)
        #[arg(default_value = ".")]
        source: String,
    },

    /// Show the provider compatibility matrix
    Matrix,
}
