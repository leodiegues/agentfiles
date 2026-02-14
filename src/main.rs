use agentfiles::{cli, commands};
use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Cli::parse();

    match args.command {
        cli::Command::Install {
            source,
            scope,
            providers,
            strategy,
            root,
        } => commands::cmd_install(source, scope, providers, strategy, root),
        cli::Command::Init { path, name } => commands::cmd_init(path, name),
        cli::Command::Scan { path, write } => commands::cmd_scan(path, write),
        cli::Command::Matrix => commands::cmd_matrix(),
    }
}
