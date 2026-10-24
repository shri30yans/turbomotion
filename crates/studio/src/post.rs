//! The final look of every frame: grade, bloom, chromatic aberration, vignette, grain and fades, in one shader pass
//! fed by a quarter-resolution highlight blur (the reference `fx.post`).

use anyhow::{Context, Result};
use skia_safe::{image_filters, Canvas, FilterMode, Image, MipmapMode, Paint, Rect, SamplingOptions, TileMode};

use super::{model::{Scene, H, W}, noise, rng::{RandomState}, shaders::{Shaders}};
use turbomotion_core::{grade, layer::{linear, Layer}};

const GRAIN_FRAMES: u32 = 8;
const GRAIN_SEED: u32 = 11;
const GRAIN_STRENGTH: f64 = 0.022;
const BLOOM_THRESHOLD: f64 = 0.70;
const BLOOM_RADIUS: f64 = 26.0;

/// The film-grain textures: eight frames of gaussian noise, drawn at half resolution and cubic-upsampled, cycled.
pub struct Grain {
    frames: Vec<Image>,
}

impl Grain {
    pub fn new(like: &Canvas) -> Result<Self> {
        let frames = (0..GRAIN_FRAMES)
            .map(|k| {
                let small = noise::gaussian(&mut RandomState::new(GRAIN_SEED + k), (W / 2) as usize, (H / 2) as usize)?;
                Ok(noise::stretch(like, &small, W, H)?.image().clone())
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { frames })
    }

    fn frame(&self, n: u32) -> &Image {
        &self.frames[(n % GRAIN_FRAMES) as usize]
    }
}

fn nearest() -> SamplingOptions {
    SamplingOptions::new(FilterMode::Nearest, MipmapMode::None)
}

/// Run `src` through the scene's look and draw the result (opaque, display-ready) onto `out`.
pub fn apply(shaders: &Shaders, grain: &Grain, out: &Canvas, src: &Image, scene: &Scene, frame: u32, fade: f64) -> Result<()> {
    let bloom_k = scene.bloom;
    let src_shader = |sampling| src.to_shader((TileMode::Clamp, TileMode::Clamp), sampling, None).context("source shader");
    let (qw, qh) = (W / 4, H / 4);

    // quarter-resolution highlights, blurred at two widths
    let bright = shaders.bright.instance(
        grade::uniforms(&scene.grade).f(BLOOM_THRESHOLD),
        &[src_shader(nearest())?],
    )?;
    let highlights = Layer::render(out, qw, qh, |c| {
        let mut paint = Paint::default();
        paint.set_shader(bright);
        c.draw_rect(Rect::from_wh(qw as f32, qh as f32), &paint);
    })?;
    let blurred = |sigma: f64| -> Result<Image> {
        let mut paint = Paint::default();
        paint.set_image_filter(image_filters::blur((sigma as f32, sigma as f32), TileMode::Clamp, None, None));
        Ok(Layer::render(out, qw, qh, |c| {
            c.draw_image(highlights.image(), (0, 0), Some(&paint));
        })?
        .image()
        .clone())
    };
    let (narrow, wide) = if bloom_k > 0.0 { (blurred(BLOOM_RADIUS / 4.0)?, blurred(BLOOM_RADIUS / 4.0 * 2.8)?) } else { (highlights.image().clone(), highlights.image().clone()) };
    let bloom_shader = |img: &Image| img.to_shader((TileMode::Clamp, TileMode::Clamp), linear(), None).context("bloom shader");

    let grain_shader = grain.frame(frame).to_shader((TileMode::Clamp, TileMode::Clamp), nearest(), None).context("grain shader")?;
    let shader = shaders.post.instance(
        grade::uniforms(&scene.grade).v2(f64::from(W), f64::from(H)).v4(bloom_k, scene.ca, scene.vig, GRAIN_STRENGTH).f(fade),
        &[src_shader(linear())?, bloom_shader(&narrow)?, bloom_shader(&wide)?, grain_shader],
    )?;
    let mut paint = Paint::default();
    paint.set_shader(shader);
    out.draw_rect(Rect::from_wh(W as f32, H as f32), &paint);
    Ok(())
}
