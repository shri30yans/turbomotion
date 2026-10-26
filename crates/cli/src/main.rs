//! `turbomotion`: the command line. It owns no rendering; each subcommand belongs to the crate that implements it.
//!
//! * `comp`  compositions: layers on a timeline, rendered by `turbomotion-comp` on the platform in `turbomotion-core`
//! * `parity` the ledger of engine features against After Effects: check, report, gaps, scaffold a new effect
//! * `docs`  the reference pages, generated from the catalogue and the SDK's doc comments
//! * `sdk`   TypeScript films that build those compositions, `turbomotion-script`

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod docs;
mod parity;

#[derive(Parser)]
#[command(about = "Native motion-graphics renderer: Skia on Metal, no browser")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compositions: layers, masks, mattes, nested comps, video and audio, text animators, particles, 3D, expressions
    Comp {
        #[command(subcommand)]
        command: turbomotion_comp::cli::Command,
    },
    /// TypeScript films: run a script on the embedded engine and build or render the composition it describes
    Sdk {
        #[command(subcommand)]
        command: turbomotion_script::cli::Command,
    },
    /// The parity ledger: what the engine can do against what After Effects can do (check, report, gaps, init-effect)
    Parity {
        #[command(subcommand)]
        command: parity::Command,
    },
    /// Write the reference pages from the code (the catalogue and the SDK's doc comments), or check they are current
    Docs {
        /// Where the pages live
        #[arg(long, default_value = "docs/reference")]
        dir: PathBuf,
        /// Rewrite the pages
        #[arg(long, conflicts_with = "check")]
        write: bool,
        /// Fail if any page is out of date (the default)
        #[arg(long)]
        check: bool,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Comp { command } => turbomotion_comp::cli::run(command),
        Command::Sdk { command } => turbomotion_script::cli::run(command),
        Command::Parity { command } => parity::run(command),
        Command::Docs { dir, write, .. } => docs::run(&dir, !write),
    }
}
