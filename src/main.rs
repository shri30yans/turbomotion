//! Native renderer for compiled films: reads `film.json`, draws with Skia on Metal, pipes raw
//! frames to ffmpeg. Comments citing `bench/remotion/src/…` refer to the Remotion implementation
//! of the same shots, kept as the benchmark and parity reference.

mod camera;
mod chrome;
mod css_filter;
mod ease;
mod encode;
mod film;
mod gpu;
mod layer;
mod motion;
mod image;
mod overlay;
mod paper;
mod palette;
mod photo;
mod pins;
mod probe;
mod render;
mod review;
mod shots;
mod text;

use std::{fs, path::PathBuf, thread, time::Instant};

use anyhow::{ensure, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::{
    encode::Codec,
    film::Film,
    render::{render_segment, split, Job, Project, Timings},
};

#[derive(Parser)]
#[command(about = "Render a compiled film with Skia on Metal")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Args)]
struct Source {
    /// build/film.json
    film: PathBuf,
    /// Directory the film's asset paths are relative to (the project's public/)
    #[arg(long)]
    public: PathBuf,
    /// The asset directory (fonts/, grain/, maps/, paper.jpg)
    #[arg(long)]
    assets: PathBuf,
}

#[derive(Clone, Copy, ValueEnum)]
enum CodecKind {
    /// Hardware H.264 (VideoToolbox), quality set by `--bitrate`
    Hardware,
    /// libx264 on the CPU, quality set by `--crf`, speed by `--preset`
    Software,
}

#[derive(Args)]
struct EncoderOptions {
    #[arg(long, value_enum, default_value_t = CodecKind::Hardware)]
    codec: CodecKind,
    /// Video bitrate for the hardware codec (ffmpeg syntax)
    #[arg(long, default_value = "20M")]
    bitrate: String,
    /// Constant rate factor for the software codec
    #[arg(long, default_value_t = 17)]
    crf: u32,
    /// x264 preset for the software codec
    #[arg(long, default_value = "faster")]
    preset: String,
}

impl EncoderOptions {
    fn codec(self) -> Codec {
        match self.codec {
            CodecKind::Hardware => Codec::Hardware { bitrate: self.bitrate },
            CodecKind::Software => Codec::Software { crf: self.crf, preset: self.preset },
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Render the film (or a frame range) to a silent mp4
    Render {
        #[command(flatten)]
        source: Source,
        /// Output .mp4 (no audio)
        #[arg(long)]
        out: PathBuf,
        /// First frame, inclusive
        #[arg(long, default_value_t = 0)]
        from: u32,
        /// Last frame, exclusive (default: the end of the film)
        #[arg(long)]
        to: Option<u32>,
        /// Parallel workers, each with its own GPU context and encoder
        #[arg(long, default_value_t = 3)]
        workers: u32,
        #[command(flatten)]
        encoder: EncoderOptions,
    },
    /// One PNG per shot, for review
    Stills {
        #[command(flatten)]
        source: Source,
        /// Directory for <shot id>.png
        #[arg(long)]
        dir: PathBuf,
        /// Only these shots (comma separated ids)
        #[arg(long, value_delimiter = ',')]
        shots: Vec<String>,
        /// Seconds into each shot (default: settled, after the last cue)
        #[arg(long)]
        at: Option<f32>,
    },
    /// The words a diagram step puts on screen, as JSON
    Reads {
        /// Diagram name (`water_lift`)
        diagram: String,
        /// Which step
        #[arg(long)]
        step: u32,
        /// The asset directory
        #[arg(long)]
        assets: PathBuf,
    },
    /// Check text layout at three moments per shot and that frames redraw identically
    Qa {
        #[command(flatten)]
        source: Source,
        /// Where to write the JSON report
        #[arg(long)]
        report: PathBuf,
        /// Only these shots (comma separated ids; skips the determinism check)
        #[arg(long, value_delimiter = ',')]
        shots: Vec<String>,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Render { source, out, from, to, workers, encoder } => {
            let film = load(&source, &[])?;
            let project = project(&film, &source);
            render(project, &out, from, to, workers, encoder.codec())
        }
        Command::Stills { source, dir, shots, at } => {
            let film = load(&source, &shots)?;
            review::stills(project(&film, &source), &dir, &shots, at)
        }
        Command::Reads { diagram, step, assets } => {
            let fonts = text::Fonts::load(&assets.join("fonts"))?;
            println!("{}", serde_json::to_string(&shots::diagram_reads(&fonts, &diagram, step)?)?);
            Ok(())
        }
        Command::Qa { source, report, shots } => {
            let film = load(&source, &shots)?;
            review::qa(project(&film, &source), &report, &shots).map(|_| ())
        }
    }
}

/// Load the film and reject any of `only` (default: all) shots the renderer cannot draw faithfully,
/// before any work starts.
fn load(source: &Source, only: &[String]) -> Result<Film> {
    let film = Film::load(&source.film)?;
    film.shots.iter().filter(|s| only.is_empty() || only.contains(&s.id)).try_for_each(shots::check)?;
    Ok(film)
}

fn project<'a>(film: &'a Film, source: &'a Source) -> Project<'a> {
    Project { film, public: &source.public, assets: &source.assets }
}

fn render(project: Project, out: &std::path::Path, from: u32, to: Option<u32>, workers: u32, codec: Codec) -> Result<()> {
    let film = project.film;
    let frames = from..to.unwrap_or(film.duration_in_frames);
    ensure!(frames.end <= film.duration_in_frames && frames.start < frames.end, "frame range {frames:?} is outside the film");

    let started = Instant::now();
    let job = Job { project, codec: &codec };
    let segments = split(frames.clone(), workers);
    let parts: Vec<PathBuf> = (0..segments.len()).map(|i| out.with_extension(format!("part{i}.mp4"))).collect();

    let timings: Vec<Timings> = thread::scope(|scope| {
        let workers: Vec<_> = segments
            .iter()
            .zip(&parts)
            .map(|(range, part)| scope.spawn(|| render_segment(&job, range.clone(), part)))
            .collect();
        workers.into_iter().map(|w| w.join().expect("render worker panicked")).collect::<Result<_>>()
    })?;
    encode::concat(&parts, out)?;
    parts.iter().try_for_each(|p| fs::remove_file(p).with_context(|| format!("removing {}", p.display())))?;

    let wall = started.elapsed().as_secs_f64();
    let n = frames.len() as f64;
    let t: Timings = timings.into_iter().sum();
    println!(
        "{n} frames in {wall:.2}s = {:.1} fps ({} workers) | summed over workers: prepare {:.2}s draw {:.2}s readback {:.2}s encode-write {:.2}s",
        n / wall,
        segments.len(),
        t.prepare.as_secs_f64(),
        t.draw.as_secs_f64(),
        t.readback.as_secs_f64(),
        t.encode.as_secs_f64()
    );
    Ok(())
}
