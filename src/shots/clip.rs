//! Moving picture: a trimmed clip of archive or drone footage under the same camera, grade and
//! overlays as a photograph.
//!
//! Every frame stays a pure function of time. `prepare` has ffmpeg lay the shot's frames out once as
//! numbered JPEGs at the film's frame rate (cached under `public/clips/.frames/`), and `draw` reads
//! the frame for the shot's local time, so stills, QA and parallel segments all agree.

use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::PathBuf,
    process::Command,
};

use anyhow::{bail, ensure, Context, Result};
use skia_safe::{Canvas, Data, Image};

use super::{Loader, Moment, Scene, Stage};
use crate::{film::Shot, overlay::Overlay, photo::Photo, pins::Pins, spot::Spot, figure::Figure};

pub const PARAMS: &[&str] = &["from", "to", "grade", "dim", "tag", "badge", "hero", "title", "note", "pins", "pinsFrom", "spot", "spotCue", "figure", "figureAt", "figureCue", "figureH", "start", "speed"];

struct ClipShot {
    photo: Photo,
    overlay: Option<Overlay>,
    pins: Option<Pins>,
    spot: Option<Spot>,
    figure: Option<Figure>,
    dir: PathBuf,
    frames: u32,
}

fn number(shot: &Shot, key: &str, default: f32) -> Result<f32> {
    shot.param(key).map(|v| v.parse().with_context(|| format!("{}: {key} is not a number", shot.id))).transpose().map(|v| v.unwrap_or(default))
}

pub fn prepare(loader: &mut Loader, shot: &Shot) -> Result<Box<dyn Scene>> {
    let id = shot.args.first().with_context(|| format!("{}: clip shot without an asset", shot.id))?;
    let asset = loader.film.assets.get(id).with_context(|| format!("{}: unknown asset {id:?}", shot.id))?;
    ensure!(asset.kind == "clip", "{}: asset {id:?} is a {}, not a clip", shot.id, asset.kind);
    let (start, speed) = (number(shot, "start", 0.0)?, number(shot, "speed", 1.0)?);
    ensure!(speed > 0.0 && start >= 0.0, "{}: start/speed out of range", shot.id);
    let film = loader.film;
    let file = loader.public.join(&asset.file);
    let frames = shot.duration_in_frames + 1;
    let meta = fs::metadata(&file).with_context(|| format!("{}: {}", shot.id, file.display()))?;
    let mut hasher = DefaultHasher::new();
    (&asset.file, meta.len(), start.to_bits(), speed.to_bits(), film.fps, film.width, film.height, frames).hash(&mut hasher);
    let dir = loader.public.join("clips/.frames").join(format!("{id}-{:016x}", hasher.finish()));
    if !dir.join("done").exists() {
        let tmp = dir.with_extension("partial");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp)?;
        let (w, h) = (film.width, film.height);
        let status = Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-ss", &start.to_string(), "-i"])
            .arg(&file)
            .args(["-an", "-vf", &format!("setpts=PTS/{speed},fps={},scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}", film.fps)])
            .args(["-frames:v", &frames.to_string(), "-q:v", "3"])
            .arg(tmp.join("%05d.jpg"))
            .status()
            .context("running ffmpeg (is it on PATH?)")?;
        ensure!(status.success(), "{}: ffmpeg could not read {}", shot.id, file.display());
        let made = fs::read_dir(&tmp)?.count() as u32;
        ensure!(made > 0, "{}: no frames came out of {} at start={start}", shot.id, file.display());
        if made < frames {
            eprintln!("warn: {}: clip {id} has {made} frames after start={start}, shot needs {frames}; the last frame is held", shot.id);
        }
        fs::write(tmp.join("done"), made.to_string())?;
        let _ = fs::remove_dir_all(&dir);
        fs::rename(&tmp, &dir)?;
    }
    let made: u32 = fs::read_to_string(dir.join("done"))?.trim().parse().unwrap_or(1);
    let first = frame(&dir, 0)?;
    Ok(Box::new(ClipShot {
        photo: Photo::for_clip(shot, first)?,
        overlay: Overlay::prepare(loader.fonts, film, shot)?,
        pins: Pins::prepare(loader.fonts, film, shot)?,
        spot: Spot::prepare(loader.fonts, film, shot)?,
        figure: Figure::prepare(loader.fonts, film, loader.public, shot)?,
        dir,
        frames: made,
    }))
}

fn frame(dir: &std::path::Path, index: u32) -> Result<Image> {
    let path = dir.join(format!("{:05}.jpg", index + 1));
    let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    match Image::from_encoded(Data::new_copy(&bytes)) {
        Some(image) => Ok(image),
        None => bail!("decoding {}", path.display()),
    }
}

impl Scene for ClipShot {
    fn draw(&self, canvas: &Canvas, stage: &Stage, at: Moment) -> Result<()> {
        let index = (at.frame.max(0.0) as u32).min(self.frames - 1);
        let image = frame(&self.dir, index)?;
        self.photo.draw_with(canvas, stage.photo, stage.film, at.seconds, &image)?;
        let view = self.photo.placement(stage.film, at.seconds).rect;
        if let Some(spot) = &self.spot {
            spot.draw(canvas, view, at.seconds);
        }
        if let Some(overlay) = &self.overlay {
            overlay.draw(canvas, at.seconds);
        }
        if let Some(pins) = &self.pins {
            pins.draw(canvas, view, at.seconds);
        }
        if let Some(figure) = &self.figure {
            figure.draw(canvas, at.seconds);
        }
        Ok(())
    }
}
