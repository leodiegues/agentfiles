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
            pick,
            no_save,
            root,
        } => commands::cmd_install(source, scope, providers, strategy, pick, no_save, root),
        cli::Command::Init { path, name } => commands::cmd_init(path, name),
        cli::Command::Scan { source } => commands::cmd_scan(source),
        cli::Command::Matrix => commands::cmd_matrix(),
    }
}
