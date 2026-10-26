//! `turbomotion sdk …`: build a TypeScript film into composition JSON, or all the way to a video.

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use turbomotion_comp::cli as comp;

use crate::{run::{build_full, Options}, summary::summary};

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
        /// Installed tsc executable or tsc.js (default: `tsc` on PATH); never downloads packages
        #[arg(long)]
        tsc: Option<PathBuf>,
        /// Skip the compiler: run the film and validate every layer it emits against the engine
        #[arg(long)]
        schema: bool,
        #[arg(long)]
        json: bool,
    },
    /// Evaluate a film at a time and print one layer's state (`--layer shot/layer` reaches inside shots)
    Inspect {
        #[command(flatten)]
        script: Script,
        #[arg(long)]
        t: f64,
        #[arg(long)]
        layer: Option<String>,
    },
    /// Re-render a contact sheet whenever the script or anything it imports changes
    Watch {
        #[command(flatten)]
        script: Script,
        #[arg(long)]
        sheet: PathBuf,
        /// Sheet times in seconds (shot-local with `--shot`); default: eight evenly spread moments
        #[arg(long, value_delimiter = ',')]
        times: Vec<f64>,
        /// Milliseconds between checks for edits
        #[arg(long, default_value_t = 400)]
        interval_ms: u64,
        /// Render once and exit
        #[arg(long)]
        once: bool,
    },
    /// Write a starting point: a typed scene folder (`new scene NAME`)
    New {
        #[command(subcommand)]
        what: New,
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

#[derive(Subcommand)]
pub enum New {
    /// `DIR/NAME/{index.ts,types.ts}`: a typed options interface and a function that returns a shot
    Scene {
        /// Folder-safe name: lowercase letters, digits, - and _
        name: String,
        /// Where the scene folder goes
        #[arg(long, default_value = ".")]
        dir: PathBuf,
        /// Also write `DIR/probe_NAME.ts`, a film that renders only this scene
        #[arg(long)]
        probe: bool,
    },
}

pub fn build_to(script: &Script, out: Option<PathBuf>) -> Result<PathBuf> {
    build_written(script, out).map(|(path, _)| path)
}

/// Write the composition JSON; also return every source file the build read.
pub fn build_written(script: &Script, out: Option<PathBuf>) -> Result<(PathBuf, Vec<PathBuf>)> {
    let options = Options { sdk_dir: script.sdk_dir.clone(), ..Options::default() };
    let built = build_full(&script.file, &options, false)?;
    let (mut film, files) = (built.film, built.files);
    if let Some(id) = &script.shot { crate::shot::isolate(&mut film, id)?; }
    let path = out.unwrap_or_else(|| script.file.with_extension(if script.shot.is_some() { "shot.comp.json" } else { "comp.json" }));
    relativize_root(&mut film, &path)?;
    fs::write(&path, serde_json::to_string_pretty(&film)?).with_context(|| format!("writing {}", path.display()))?;
    println!("{}: {} -> {}", script.file.display(), summary(&film), path.display());
    Ok((path, files))
}

/// A contact sheet of `times` (default: eight even moments), drawn through the library.
pub fn write_sheet(comp_file: &std::path::Path, out: &std::path::Path, times: &[f64]) -> Result<()> {
    let plan = turbomotion_comp::plan::Plan::load(comp_file)?;
    let times: Vec<f64> = if times.is_empty() { (0..8).map(|i| plan.root().dur * (f64::from(i) + 0.5) / 8.0).collect() } else { times.to_vec() };
    let mut renderer = turbomotion_comp::render::Renderer::new(&plan)?;
    fs::write(out, renderer.sheet(&times, 4, 480)?).with_context(|| format!("writing {}", out.display()))?;
    println!("sheet of {} moments -> {}", times.len(), out.display());
    Ok(())
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
        Command::Check { file, sdk_dir, tsc, schema, json } => {
            let report = crate::check::auto(&file, sdk_dir.as_deref(), tsc.as_deref(), schema)?;
            if json {
                println!("{}", serde_json::json!({"ok":report.success,"file":file,"mode":report.mode,"diagnostics":report.diagnostics}));
            } else {
                println!("sdk check mode: {}", report.mode);
                if report.success { println!("{}: check passed", file.display()); if !report.diagnostics.is_empty() { print!("{}", report.diagnostics); } }
                else { eprint!("{}", report.diagnostics); }
            }
            anyhow::ensure!(report.success, "check failed; fix the diagnostics and run sdk check again");
            Ok(())
        }
        Command::Inspect { script, t, layer } => {
            let options = Options { sdk_dir: script.sdk_dir.clone(), ..Options::default() };
            let mut film = build_full(&script.file, &options, false)?.film;
            if let Some(id) = &script.shot { crate::shot::isolate(&mut film, id)?; }
            let dir = fs::canonicalize(&script.file)?.parent().map(PathBuf::from).unwrap_or_default();
            println!("{}", serde_json::to_string_pretty(&turbomotion_comp::api::inspect(film, &dir, t, layer.as_deref())?)?);
            Ok(())
        }
        Command::Watch { script, sheet, times, interval_ms, once } => crate::watch::run(&crate::watch::Watch {
            script: &script, sheet: &sheet, times: &times, interval: std::time::Duration::from_millis(interval_ms.max(50)), max_renders: once.then_some(1),
        }),
        Command::New { what: New::Scene { name, dir, probe } } => {
            let created = crate::scaffold::scene(&name, &dir, probe)?;
            created.files.iter().for_each(|f| println!("created {}", f.display()));
            println!("render it with:\n  {}", created.render);
            Ok(())
        }
        Command::Build { script, out } => build_to(&script, out).map(|_| ()),
        Command::Render { script, out, sheet, times, sheet_only, workers } => {
            // The recorded film root preserves asset paths independently of the output location.
            let comp_file = build_to(&script, None)?;
            let source = comp::Source { file: comp_file };
            if let Some(sheet) = sheet {
                write_sheet(&source.file, &sheet, &times)?;
            }
            if !sheet_only {
                let out = out.unwrap_or_else(|| script.file.with_extension(if script.shot.is_some() { "shot.mp4" } else { "mp4" }));
                comp::run(comp::Command::Render { source, out, from: 0, to: None, workers, codec: comp::CodecKind::Hardware, bitrate: None, crf: 17 })?;
            }
            Ok(())
        }
    }
}
