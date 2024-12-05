use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use miette::IntoDiagnostic;

use super::error::ScannrsError;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// List available scanners
    List,
    /// Get all options this scanner exposes
    Options {
        /// Which scanner to operate on
        name: String,

        #[command(subcommand)]
        command: Option<OptionsCommand>,
    },
    Scan {
        /// Which scanner to operate on
        name: String,

        /// A list of options in `key=value` format to set before scanning, can be used multiple times, later options
        /// replace earlier ones.
        #[arg(short, long, value_parser = split_options)]
        options: Vec<(Vec<u8>, String)>,

        /// The path to save the scan at
        #[arg(short, long)]
        path: PathBuf,
    },
    Tui,
}

pub(crate) fn split_options(opt: &str) -> miette::Result<(Vec<u8>, String)> {
    opt.split_once('=')
        .map(|(k, v)| (k.trim().to_string().into_bytes(), v.trim().to_string()))
        .ok_or(ScannrsError::InvalidOption)
        .into_diagnostic()
}

#[derive(Default, Subcommand)]
pub(crate) enum OptionsCommand {
    #[default]
    List,
    Show {
        option: String,
    },
}
