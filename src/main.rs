use clap::Parser;
use miette::IntoDiagnostic;
use sane_scan::Sane;

mod cli;
mod commands;
mod error;

fn main() -> miette::Result<()> {
    human_panic::setup_panic!();

    let args = cli::Cli::parse();

    let sane = Sane::init_1_0().into_diagnostic()?;

    match args.command {
        cli::Command::List => {
            commands::list(sane)?;
        }
        cli::Command::Options { name, command } => {
            commands::options(sane, name, command)?;
        }
        cli::Command::Scan {
            name,
            path,
            options,
        } => {
            commands::scan(sane, name, path, options)?;
        }

        cli::Command::Tui => commands::tui(sane)?,
    }

    Ok(())
}
