use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::types::FileScope;

#[derive(Parser)]
#[command(name = "agentfiles", about = "Unified agent file installer")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Install {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(default_value = "project")]
        scope: String,
    },
    /// Initialize a new agentfiles.json
    Init {
        /// Directory to create the manifest in
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}
