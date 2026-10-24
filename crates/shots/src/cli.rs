use std::{path::PathBuf, time::Instant};

use anyhow::{ensure, Result};
use clap::{Args, Subcommand, ValueEnum};
use turbomotion_core::{encode::Codec, parallel};

use crate::{
    film::Film,
    render::{render_segment, Job, Project, Timings},
    review, shots, text,
};

#[derive(Args)]
pub struct Source {
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
pub enum CodecKind {
    /// Hardware H.264 (VideoToolbox), quality set by `--bitrate`
    Hardware,
    /// libx264 on the CPU, quality set by `--crf`, speed by `--preset`
    Software,
}

#[derive(Args)]
pub struct EncoderOptions {
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
pub enum Command {
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
        /// Exit 0 even when layout issues or non-deterministic frames were found (the report still lists them)
        #[arg(long)]
        allow_issues: bool,
    },
}

pub fn run(command: Command) -> Result<()> {
    match command {
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
        Command::Qa { source, report, shots, allow_issues } => {
            let film = load(&source, &shots)?;
            let (layout, unstable) = review::qa(project(&film, &source), &report, &shots)?;
            if !allow_issues && (layout > 0 || unstable > 0) {
                anyhow::bail!("qa failed: {layout} frames with layout issues, {unstable} frames that did not redraw identically (see {})", report.display());
            }
            Ok(())
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
    let timings = parallel::render(frames.clone(), workers, out, |range, part| render_segment(&job, range, part))?;

    let wall = started.elapsed().as_secs_f64();
    let n = frames.len() as f64;
    let t: Timings = timings.iter().copied().sum();
    println!(
        "{n} frames in {wall:.2}s = {:.1} fps ({} workers) | summed over workers: prepare {:.2}s draw {:.2}s readback {:.2}s encode-write {:.2}s",
        n / wall,
        timings.len(),
        t.prepare.as_secs_f64(),
        t.draw.as_secs_f64(),
        t.readback.as_secs_f64(),
        t.encode.as_secs_f64()
    );
    Ok(())
}
