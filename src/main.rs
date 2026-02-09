use agentfiles::cli;
use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Cli::parse();

    match args.command {
        cli::Command::Install { path, scope } => {
            // call into lib functions
        }
        cli::Command::Init { path } => {
            // call into lib functions
        }
    }

    Ok(())
}
