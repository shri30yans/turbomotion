//! Frame rendering: a `Renderer` owns one Metal context and draws any frame of a compiled film; segment workers add an
//! encoder, the review commands read the pixels back.

use std::{
    ops::Range,
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{bail, ensure, Context, Result};
use skia_safe::{png_encoder, AlphaType, ColorType, ISize, ImageInfo, Pixmap, Surface};

use crate::{
    encode::{Codec, Encoder},
    gpu::Gpu,
};

use super::{
    ease::clamp,
    model::{Film, H, W},
    post::{self, Grain},
    scene,
    stage::Stage,
    transitions,
};

/// Seconds the film fades in from black, and out to black at the end.
const FADE_IN: f64 = 0.6;
const FADE_OUT: f64 = 0.9;

pub struct Renderer<'a> {
    film: &'a Film,
    gpu: Gpu,
    stage: Stage,
    grain: Grain,
    surface: Surface,
    info: ImageInfo,
}

impl<'a> Renderer<'a> {
    pub fn new(film: &'a Film) -> Result<Self> {
        let mut gpu = Gpu::new()?;
        let mut surface = gpu.frame_surface(ISize::new(W, H))?;
        let stage = Stage::new(film)?;
        let grain = Grain::new(surface.canvas())?;
        let info = ImageInfo::new(ISize::new(W, H), ColorType::BGRA8888, AlphaType::Premul, None);
        Ok(Self { film, gpu, stage, grain, surface, info })
    }

    /// Draw `frame` into the surface: the scene (cross-blended with its predecessor while a transition runs), then the look.
    pub fn draw(&mut self, frame: u32) -> Result<()> {
        let film = self.film;
        let t = f64::from(frame) / f64::from(film.fps);
        let i = film.scene_at(t);
        let sc = &film.scenes[i];
        let canvas = self.surface.canvas();
        let mut image = scene::render(&self.stage, canvas, sc, t)?;
        let u = t - sc.t0;
        if i > 0 && sc.trans != "cut" && u < sc.trd {
            let prev = scene::render(&self.stage, canvas, &film.scenes[i - 1], t)?;
            image = transitions::blend(&self.stage, canvas, &sc.trans, &prev, &image, clamp(u / sc.trd))?;
        }
        let mut fade = 1.0;
        if t < FADE_IN {
            fade *= t / FADE_IN;
        }
        if t > film.end - FADE_OUT {
            fade *= ((film.end - t) / FADE_OUT).max(0.0);
        }
        post::apply(&self.stage.shaders, &self.grain, canvas, &image, sc, frame, fade)?;
        self.gpu.context.flush_and_submit();
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

    pub fn png(&mut self) -> Result<Vec<u8>> {
        let mut pixels = Vec::new();
        self.pixels(&mut pixels)?;
        let pixmap = Pixmap::new(&self.info, &mut pixels, self.info.min_row_bytes()).context("wrapping the frame pixels")?;
        let mut png = Vec::new();
        ensure!(png_encoder::encode(&pixmap, &mut png, &png_encoder::Options::default()), "PNG encoding failed");
        Ok(png)
    }
}

/// Wall time a worker spent drawing, reading back and encoding.
#[derive(Default, Clone, Copy)]
pub struct Timings {
    pub draw: Duration,
    pub readback: Duration,
    pub encode: Duration,
}

impl std::iter::Sum for Timings {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |a, b| Self { draw: a.draw + b.draw, readback: a.readback + b.readback, encode: a.encode + b.encode })
    }
}

/// Render `frames` into a silent mp4 at `out`.
pub fn render_segment(film: &Film, codec: &Codec, frames: Range<u32>, out: &Path) -> Result<Timings> {
    let mut renderer = Renderer::new(film)?;
    let mut encoder = Encoder::spawn(out, crate::encode::VideoFormat { width: W, height: H, fps: film.fps }, codec)?;
    let (mut pixels, mut times) = (Vec::new(), Timings::default());
    for frame in frames {
        let at = Instant::now();
        renderer.draw(frame).with_context(|| format!("frame {frame}"))?;
        times.draw += at.elapsed();
        let at = Instant::now();
        renderer.pixels(&mut pixels)?;
        times.readback += at.elapsed();
        let at = Instant::now();
        encoder.write(&pixels)?;
        times.encode += at.elapsed();
    }
    encoder.finish()?;
    Ok(times)
}
