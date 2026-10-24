//! `turbomotion`: the command line. It owns no rendering; each subcommand belongs to the crate that implements it.
//!
//! * (top level) the original shot renderer, `turbomotion-shots`
//! * `studio`    compiled motion-graphics films, `turbomotion-studio`
//! * `comp`      After Effects-style compositions, `turbomotion-comp`

use anyhow::Result;
use clap::{Parser, Subcommand};

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
    /// Compiled motion-graphics films (see crates/studio/README.md)
    Studio {
        #[command(subcommand)]
        command: turbomotion_studio::cli::Command,
    },
    #[command(flatten)]
    Shots(turbomotion_shots::cli::Command),
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Comp { command } => turbomotion_comp::cli::run(command),
        Command::Studio { command } => turbomotion_studio::cli::run(command),
        Command::Shots(command) => turbomotion_shots::cli::run(command),
    }
}
