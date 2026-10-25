//! `turbomotion sdk …`: build a TypeScript film into composition JSON, or all the way to a video.

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use turbomotion_comp::cli as comp;

use crate::run::{build, summary, Options};

#[derive(Args)]
pub struct Script {
    /// The film script (`film.ts`)
    pub file: PathBuf,
    /// Load TypeScript SDK sources from this directory (must contain index.ts)
    #[arg(long)]
    pub sdk_dir: Option<PathBuf>,
    /// Build or render this shot in isolation, starting at time zero
    #[arg(long)]
    pub shot: Option<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Type check a film with an installed TypeScript compiler (never executes the film)
    Check {
        file: PathBuf,
        #[arg(long)]
        sdk_dir: Option<PathBuf>,
        /// Installed tsc executable or tsc.js; never downloads packages
        #[arg(long, default_value = "tsc")]
        tsc: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Run a script and write the composition JSON it produces
    Build {
        #[command(flatten)]
        script: Script,
        /// Where to write the composition (default: `<script>.comp.json` beside the script)
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Build a script, then render it to an mp4 (and optionally a contact sheet)
    Render {
        #[command(flatten)]
        script: Script,
        #[arg(long)]
        out: Option<PathBuf>,
        /// Also write a labelled contact sheet here
        #[arg(long)]
        sheet: Option<PathBuf>,
        /// Sheet times in seconds (comma separated); default: eight evenly spread moments
        #[arg(long, value_delimiter = ',')]
        times: Vec<f64>,
        /// With `--sheet`, skip the video
        #[arg(long, requires = "sheet")]
        sheet_only: bool,
        #[arg(long, default_value_t = 3)]
        workers: u32,
    },
}

pub fn build_to(script: &Script, out: Option<PathBuf>) -> Result<PathBuf> {
    let options = Options { sdk_dir: script.sdk_dir.clone(), ..Options::default() };
    let mut film = build(&script.file, &options)?;
    if let Some(id) = &script.shot { crate::shot::isolate(&mut film, id)?; }
    let path = out.unwrap_or_else(|| script.file.with_extension(if script.shot.is_some() { "shot.comp.json" } else { "comp.json" }));
    relativize_root(&mut film, &path)?;
    fs::write(&path, serde_json::to_string_pretty(&film)?).with_context(|| format!("writing {}", path.display()))?;
    println!("{}: {} -> {}", script.file.display(), summary(&film), path.display());
    Ok(path)
}

/// `build` resolves the film root to an absolute folder; a file on disk records it relative to itself, so a project can
/// be moved or checked out elsewhere without its compositions pointing at the old location.
fn relativize_root(film: &mut serde_json::Value, written: &std::path::Path) -> Result<()> {
    let Some(root) = film.get("root").and_then(|r| r.as_str()).map(PathBuf::from) else { return Ok(()) };
    let base = fs::canonicalize(written.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(std::path::Path::new(".")))?;
    let (mut from, mut to) = (base.components().peekable(), root.components().peekable());
    while from.peek().is_some() && from.peek() == to.peek() {
        from.next();
        to.next();
    }
    let mut rel = PathBuf::new();
    from.for_each(|_| rel.push(".."));
    to.for_each(|c| rel.push(c));
    let rel = if rel.as_os_str().is_empty() { PathBuf::from(".") } else { rel };
    film["root"] = serde_json::Value::String(rel.to_string_lossy().into_owned());
    Ok(())
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Check { file, sdk_dir, tsc, json } => {
            let report = crate::check::check(&file, sdk_dir.as_deref(), &tsc)?;
            if json {
                println!("{}", serde_json::json!({"ok":report.success,"file":file,"diagnostics":report.diagnostics}));
            } else if report.success { println!("{}: type check passed", file.display()); }
            else { eprint!("{}", report.diagnostics); }
            anyhow::ensure!(report.success, "type check failed; fix the diagnostics and run sdk check again");
            Ok(())
        }
        Command::Build { script, out } => build_to(&script, out).map(|_| ()),
        Command::Render { script, out, sheet, times, sheet_only, workers } => {
            // The recorded film root preserves asset paths independently of the output location.
            let comp_file = build_to(&script, None)?;
            let source = comp::Source { file: comp_file };
            if let Some(sheet) = sheet {
                comp::run(comp::Command::Sheet { source: comp::Source { file: source.file.clone() }, out: sheet, times, cols: 4, thumb: 480 })?;
            }
            if !sheet_only {
                let out = out.unwrap_or_else(|| script.file.with_extension(if script.shot.is_some() { "shot.mp4" } else { "mp4" }));
                comp::run(comp::Command::Render { source, out, from: 0, to: None, workers, codec: comp::CodecKind::Hardware, bitrate: None, crf: 17 })?;
            }
            Ok(())
        }
    }
}
