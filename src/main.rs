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
            dry_run,
            root,
        } => commands::cmd_install(commands::InstallOptions {
            source,
            scope,
            providers,
            strategy,
            pick,
            no_save,
            dry_run,
            root,
        }),
        cli::Command::Init { path, name } => commands::cmd_init(path, name),
        cli::Command::Scan { source } => commands::cmd_scan(source),
        cli::Command::Remove {
            source,
            clean,
            scope,
            providers,
            root,
        } => commands::cmd_remove(source, clean, scope, providers, root),
        cli::Command::List { root } => commands::cmd_list(root),
        cli::Command::Matrix => commands::cmd_matrix(),
    }
}
