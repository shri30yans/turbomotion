//! `turbomotion studio …`: render a compiled studio film, or write review frames.

use std::{fs, path::PathBuf, thread, time::Instant};

use anyhow::{ensure, Context, Result};
use clap::{Args, Subcommand, ValueEnum};

use crate::{
    encode::{self, Codec},
    render::split,
};

use super::{
    model::Film,
    render::{render_segment, Renderer, Timings},
};

#[derive(Args)]
pub struct Source {
    /// Directory holding the compiled film (`film.json`, `geo.json`, `plates/`, `chars/`), written by the studio plan compiler
    pub dir: PathBuf,
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
    /// Render the film (or a frame range) to a silent mp4
    Render {
        #[command(flatten)]
        source: Source,
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
        #[arg(long, value_enum, default_value_t = CodecKind::Hardware)]
        codec: CodecKind,
        #[arg(long, default_value = "20M")]
        bitrate: String,
        #[arg(long, default_value_t = 17)]
        crf: u32,
    },
    /// Write PNGs of chosen moments for review and parity checks
    Stills {
        #[command(flatten)]
        source: Source,
        /// Where to write the PNGs
        #[arg(long)]
        out: PathBuf,
        /// Times in seconds (comma separated)
        #[arg(long, value_delimiter = ',', required = true)]
        times: Vec<f64>,
    },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Render { source, out, from, to, workers, codec, bitrate, crf } => {
            let film = Film::load(&source.dir)?;
            let codec = match codec {
                CodecKind::Hardware => Codec::Hardware { bitrate },
                CodecKind::Software => Codec::Software { crf, preset: "faster".into() },
            };
            render(&film, &codec, &out, from, to, workers)
        }
        Command::Stills { source, out, times } => {
            let film = Film::load(&source.dir)?;
            fs::create_dir_all(&out).with_context(|| format!("creating {}", out.display()))?;
            let mut renderer = Renderer::new(&film)?;
            for t in times {
                renderer.draw((t * f64::from(film.fps)).round() as u32)?;
                let path = out.join(format!("{t:07.2}.png"));
                fs::write(&path, renderer.png()?).with_context(|| format!("writing {}", path.display()))?;
                println!("still {t:.2}s");
            }
            Ok(())
        }
    }
}

fn render(film: &Film, codec: &Codec, out: &std::path::Path, from: u32, to: Option<u32>, workers: u32) -> Result<()> {
    let frames = from..to.unwrap_or(film.frames());
    ensure!(frames.end <= film.frames() && frames.start < frames.end, "frame range {frames:?} is outside the film");
    let started = Instant::now();
    let segments = split(frames.clone(), workers);
    let parts: Vec<PathBuf> = (0..segments.len()).map(|i| out.with_extension(format!("part{i}.mp4"))).collect();
    let timings: Vec<Timings> = thread::scope(|scope| {
        let handles: Vec<_> = segments.iter().zip(&parts).map(|(range, part)| scope.spawn(|| render_segment(film, codec, range.clone(), part))).collect();
        handles.into_iter().map(|h| h.join().expect("render worker panicked")).collect::<Result<_>>()
    })?;
    encode::concat(&parts, out)?;
    parts.iter().try_for_each(|p| fs::remove_file(p).with_context(|| format!("removing {}", p.display())))?;
    let wall = started.elapsed().as_secs_f64();
    let n = frames.len() as f64;
    let t: Timings = timings.into_iter().sum();
    println!(
        "{n} frames in {wall:.2}s = {:.1} fps ({} workers) | summed over workers: draw {:.2}s readback {:.2}s encode-write {:.2}s",
        n / wall,
        segments.len(),
        t.draw.as_secs_f64(),
        t.readback.as_secs_f64(),
        t.encode.as_secs_f64()
    );
    Ok(())
}
