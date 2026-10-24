//! `turbomotion comp …`: render a composition, look at it, and ask it questions.

use std::{fs, path::PathBuf, time::Instant};

use anyhow::{ensure, Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use turbomotion_core::encode::Codec;

use crate::{catalogue, eval::Evaluator, plan::{Plan, ROOT}, render};

#[derive(Args)]
pub struct Source {
    /// The composition file (`comp.json`); relative paths inside it are resolved from its folder
    pub file: PathBuf,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum CodecKind {
    /// Hardware H.264 (VideoToolbox), quality set by `--bitrate`
    Hardware,
    /// libx264 on the CPU, quality set by `--crf`
    Software,
}

#[derive(Subcommand)]
pub enum Command {
    /// Print everything a composition can contain, with a working example of each, as JSON
    Catalogue,
    /// Load and validate a composition; print what it contains. Fails with a path to the first mistake
    Check {
        #[command(flatten)]
        source: Source,
    },
    /// List the installed font families a text layer can name
    Fonts,
    /// What is true at a moment: every layer's position, opacity, bounds and state, as JSON
    Inspect {
        #[command(flatten)]
        source: Source,
        /// Seconds into the composition
        #[arg(long)]
        t: f64,
        /// Only this layer id
        #[arg(long)]
        layer: Option<String>,
    },
    /// Write PNGs of chosen moments
    Still {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        out: PathBuf,
        /// Times in seconds (comma separated)
        #[arg(long, value_delimiter = ',', required = true)]
        times: Vec<f64>,
    },
    /// One labelled contact sheet of several moments, for reviewing motion at a glance
    Sheet {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        out: PathBuf,
        /// Times in seconds (comma separated); default: eight evenly spread moments
        #[arg(long, value_delimiter = ',')]
        times: Vec<f64>,
        #[arg(long, default_value_t = 4)]
        cols: u32,
        /// Width of each thumbnail in pixels
        #[arg(long, default_value_t = 480)]
        thumb: u32,
    },
    /// Render the composition (or a range of frames) to an mp4
    Render {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 0)]
        from: u32,
        /// Last frame, exclusive (default: the end)
        #[arg(long)]
        to: Option<u32>,
        #[arg(long, default_value_t = 3)]
        workers: u32,
        #[arg(long, value_enum, default_value_t = CodecKind::Hardware)]
        codec: CodecKind,
        #[arg(long, default_value = "20M")]
        bitrate: String,
        #[arg(long, default_value_t = 17)]
        crf: u32,
    },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Catalogue => {
            println!("{}", serde_json::to_string_pretty(&catalogue::all())?);
            Ok(())
        }
        Command::Fonts => {
            for f in turbomotion_core::typeface::families() {
                println!("{f}");
            }
            Ok(())
        }
        Command::Check { source } => {
            let plan = Plan::load(&source.file)?;
            let root = plan.root();
            // Evaluate the timeline at the interesting moments so expression, font and reference mistakes surface now.
            let ev = Evaluator::new(&plan);
            let mut times = vec![0.0, root.dur / 2.0, (root.dur - 1.0 / f64::from(plan.fps)).max(0.0)];
            for l in &root.layers {
                times.push(l.start + 0.01);
                times.push((l.start + l.end) / 2.0);
                times.push((l.end - 0.02).max(l.start));
            }
            times.retain(|t| *t >= 0.0 && *t < root.dur);
            times.sort_by(f64::total_cmp);
            times.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
            for t in times.iter().take(96) {
                ev.frame(ROOT, *t).with_context(|| format!("evaluating the composition at {t:.2}s"))?;
            }
            println!("ok: {}x{} at {} fps, {:.2}s ({} frames), {} layers, {} nested comps", root.width, root.height, plan.fps, root.dur, plan.frames(), root.layers.len(), plan.comps.len() - 1);
            Ok(())
        }
        Command::Inspect { source, t, layer } => {
            let plan = Plan::load(&source.file)?;
            let mut frame = serde_json::to_value(Evaluator::new(&plan).frame(ROOT, t)?)?;
            if let Some(id) = layer {
                let layers = frame["layers"].as_array().cloned().unwrap_or_default();
                let found: Vec<_> = layers.into_iter().filter(|l| l["id"] == id.as_str()).collect();
                ensure!(!found.is_empty(), "no layer with id {id:?}");
                frame["layers"] = found.into();
            }
            println!("{}", serde_json::to_string_pretty(&frame)?);
            Ok(())
        }
        Command::Still { source, out, times } => {
            let plan = Plan::load(&source.file)?;
            fs::create_dir_all(&out).with_context(|| format!("creating {}", out.display()))?;
            let mut renderer = render::Renderer::new(&plan)?;
            for t in times {
                renderer.draw_at(t)?;
                let path = out.join(format!("{t:07.2}.png"));
                fs::write(&path, renderer.png()?).with_context(|| format!("writing {}", path.display()))?;
                println!("still {t:.2}s -> {}", path.display());
            }
            Ok(())
        }
        Command::Sheet { source, out, times, cols, thumb } => {
            let plan = Plan::load(&source.file)?;
            let times = if times.is_empty() { (0..8).map(|i| plan.root().dur * (f64::from(i) + 0.5) / 8.0).collect() } else { times };
            let mut renderer = render::Renderer::new(&plan)?;
            fs::write(&out, renderer.sheet(&times, cols.max(1), thumb.max(64))?).with_context(|| format!("writing {}", out.display()))?;
            println!("sheet of {} moments -> {}", times.len(), out.display());
            Ok(())
        }
        Command::Render { source, out, from, to, workers, codec, bitrate, crf } => {
            let plan = Plan::load(&source.file)?;
            let frames = from..to.unwrap_or(plan.frames());
            ensure!(frames.end <= plan.frames() && frames.start < frames.end, "frame range {frames:?} is outside the composition ({} frames)", plan.frames());
            let codec = match codec {
                CodecKind::Hardware => Codec::Hardware { bitrate },
                CodecKind::Software => Codec::Software { crf, preset: "faster".into() },
            };
            let started = Instant::now();
            let n = frames.len();
            render::render_video(&source.file, &codec, frames, workers, &out)?;
            let wall = started.elapsed().as_secs_f64();
            println!("{n} frames in {wall:.2}s = {:.1} fps ({workers} workers) -> {}", n as f64 / wall, out.display());
            Ok(())
        }
    }
}
