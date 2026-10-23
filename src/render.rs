//! Frame rendering: a `Renderer` owns one Metal context and surface and draws any frame of a
//! film into it; a segment worker adds an encoder, the review tools read the pixels back.

use std::{
    ops::Range,
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{bail, ensure, Context, Result};
use skia_safe::{png_encoder, AlphaType, ColorType, ISize, ImageInfo, Pixmap, Surface};

use crate::{
    chrome::Chrome,
    encode::{Codec, Encoder},
    film::Film,
    gpu::Gpu,
    photo::PhotoEffect,
    shots::{self, Loader, Moment, Scene, Stage},
    text::Fonts,
};

/// A film together with where its assets live.
#[derive(Clone, Copy)]
pub struct Project<'a> {
    pub film: &'a Film,
    /// The project's `public/`, which asset paths in the film are relative to.
    pub public: &'a Path,
    /// The asset directory (`fonts/`, `grain/`, `maps/`, `paper.jpg`).
    pub assets: &'a Path,
}

/// Wall time spent in each stage, summed over a worker's frames.
#[derive(Default, Clone, Copy)]
pub struct Timings {
    pub prepare: Duration,
    pub draw: Duration,
    pub readback: Duration,
    pub encode: Duration,
}

impl std::iter::Sum for Timings {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |a, b| Self {
            prepare: a.prepare + b.prepare,
            draw: a.draw + b.draw,
            readback: a.readback + b.readback,
            encode: a.encode + b.encode,
        })
    }
}

/// Draws frames of a film. Shots are prepared once, when the first of their frames is drawn.
pub struct Renderer<'a> {
    project: Project<'a>,
    gpu: Gpu,
    photo: PhotoEffect,
    fonts: Fonts,
    chrome: Chrome,
    surface: Surface,
    info: ImageInfo,
    scene: Option<(&'a str, Box<dyn Scene>)>,
    pub timings: Timings,
}

impl<'a> Renderer<'a> {
    pub fn new(project: Project<'a>) -> Result<Self> {
        let film = project.film;
        let mut gpu = Gpu::new()?;
        let photo = PhotoEffect::compile()?;
        let fonts = Fonts::load(&project.assets.join("fonts"))?;
        let chrome = Chrome::prepare(&fonts, film, project.assets)?;
        let size = ISize::new(film.width, film.height);
        let surface = gpu.frame_surface(size)?;
        let info = ImageInfo::new(size, ColorType::BGRA8888, AlphaType::Premul, None);
        Ok(Self { project, gpu, photo, fonts, chrome, surface, info, scene: None, timings: Timings::default() })
    }

    /// Draw `frame` into the surface.
    pub fn draw(&mut self, frame: u32) -> Result<()> {
        let film = self.project.film;
        let shot = film.shot_at(frame).with_context(|| format!("no shot covers frame {frame}"))?;
        if !matches!(&self.scene, Some((id, _)) if *id == shot.id) {
            let at = Instant::now();
            let mut loader = Loader { context: &mut self.gpu.context, fonts: &self.fonts, film, public: self.project.public, assets: self.project.assets };
            let scene = shots::prepare(&mut loader, shot).with_context(|| format!("preparing {}", shot.id))?;
            self.timings.prepare += at.elapsed();
            self.scene = Some((shot.id.as_str(), scene));
        }
        let (_, scene) = self.scene.as_ref().expect("prepared above");

        let at = Instant::now();
        let stage = Stage { film, fonts: &self.fonts, photo: &self.photo };
        scene.draw(self.surface.canvas(), &stage, Moment::at(shot, frame, film.fps))?;
        self.chrome.draw(self.surface.canvas(), frame, film.fps)?;
        self.gpu.context.flush_and_submit();
        self.timings.draw += at.elapsed();
        Ok(())
    }

    /// The BGRA bytes of the last drawn frame.
    pub fn pixels(&mut self, into: &mut Vec<u8>) -> Result<()> {
        into.resize(self.info.compute_min_byte_size(), 0);
        if !self.surface.read_pixels(&self.info, into, self.info.min_row_bytes(), (0, 0)) {
            bail!("pixel readback failed");
        }
        Ok(())
    }

    /// The last drawn frame as a PNG (lightly compressed: these are review images).
    pub fn png(&mut self) -> Result<Vec<u8>> {
        let mut pixels = Vec::new();
        self.pixels(&mut pixels)?;
        let pixmap = Pixmap::new(&self.info, &mut pixels, self.info.min_row_bytes()).context("wrapping the frame pixels")?;
        let mut options = png_encoder::Options::default();
        options.z_lib_level = PNG_COMPRESSION;
        let mut png = Vec::new();
        ensure!(png_encoder::encode(&pixmap, &mut png, &options), "PNG encoding failed");
        Ok(png)
    }
}

/// zlib level for review PNGs: encoding time matters more than file size.
const PNG_COMPRESSION: i32 = 1;

pub struct Job<'a> {
    pub project: Project<'a>,
    pub codec: &'a Codec,
}

/// Render `frames` into a silent mp4 at `out`.
pub fn render_segment(job: &Job, frames: Range<u32>, out: &Path) -> Result<Timings> {
    let mut renderer = Renderer::new(job.project)?;
    let mut encoder = Encoder::spawn(out, job.project.film, job.codec)?;
    let mut pixels = Vec::new();
    let (mut readback, mut encode) = (Duration::ZERO, Duration::ZERO);

    for frame in frames {
        renderer.draw(frame)?;

        let at = Instant::now();
        renderer.pixels(&mut pixels).with_context(|| format!("frame {frame}"))?;
        readback += at.elapsed();

        let at = Instant::now();
        encoder.write(&pixels)?;
        encode += at.elapsed();
    }
    encoder.finish()?;
    Ok(Timings { readback, encode, ..renderer.timings })
}

/// Split `frames` into at most `parts` contiguous, near-equal ranges.
pub fn split(frames: Range<u32>, parts: u32) -> Vec<Range<u32>> {
    let len = frames.end - frames.start;
    let parts = parts.clamp(1, len.max(1));
    (0..parts).map(|i| frames.start + len * i / parts..frames.start + len * (i + 1) / parts).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_covers_the_range_exactly() {
        let parts = split(10..110, 3);
        assert_eq!(parts, vec![10..43, 43..76, 76..110]);
        assert_eq!(split(0..2, 8), vec![0..1, 1..2]);
    }
}
