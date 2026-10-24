//! Rendering: `Frame → pixels`. A layer is painted into its own comp-sized half-float image through its screen matrix
//! (so vectors and text stay sharp at any scale and 3D layers are projected by the GPU), then takes its effects,
//! masks and matte, and lands on the composite with its opacity and blend mode.

mod content;
mod fx;
mod media;
mod particles;
mod post;
mod programs;
mod res;
mod shape;
mod text;

use anyhow::{Context, Result};
use skia_safe::{png_encoder, AlphaType, Canvas, Color4f, ColorType, ISize, ImageInfo, Paint, Pixmap, Surface};
use turbomotion_core::{
    encode::{Codec, Encoder, VideoFormat},
    gpu::Gpu,
    layer::Layer,
    parallel,
    shaders::Shaders,
};

use crate::{
    eval::{Content, Evaluator, Frame, LayerState},
    plan::{Plan, ROOT},
};

pub struct Renderer<'a> {
    plan: &'a Plan,
    gpu: Gpu,
    surface: Surface,
    info: ImageInfo,
    shaders: Shaders,
    programs: programs::Programs,
    res: res::Resources,
    ev: Evaluator<'a>,
}

/// Everything painting needs besides the canvas.
pub struct Ctx<'a> {
    pub plan: &'a Plan,
    pub shaders: &'a Shaders,
    pub programs: &'a programs::Programs,
    pub res: &'a res::Resources,
    /// The root frame re-evaluated across the shutter, for layers with motion blur (empty when there is none).
    pub subs: &'a [Frame],
}

impl<'a> Renderer<'a> {
    pub fn new(plan: &'a Plan) -> Result<Self> {
        let mut gpu = Gpu::new()?;
        let root = plan.root();
        let size = ISize::new(root.width as i32, root.height as i32);
        let surface = gpu.frame_surface(size)?;
        let info = ImageInfo::new(size, ColorType::BGRA8888, AlphaType::Premul, None);
        Ok(Renderer { plan, gpu, surface, info, shaders: Shaders::compile().context("compiling the platform shaders")?, programs: programs::Programs::compile()?, res: res::Resources::default(), ev: Evaluator::new(plan) })
    }

    /// Draw frame number `frame` of the root composition into the surface.
    pub fn draw(&mut self, frame: u32) -> Result<()> {
        let t = f64::from(frame) / f64::from(self.plan.fps);
        let state = self.ev.frame(ROOT, t).with_context(|| format!("evaluating frame {frame}"))?;
        let subs = self.shutter_frames(t).with_context(|| format!("evaluating the shutter of frame {frame}"))?;
        self.draw_state(&state, &subs)
    }

    /// Draw the frame nearest to `t` seconds.
    pub fn draw_at(&mut self, t: f64) -> Result<()> {
        self.draw((t * f64::from(self.plan.fps)).round().max(0.0) as u32)
    }

    /// The root composition at evenly spread moments while the shutter is open, when any layer asks for motion blur.
    fn shutter_frames(&self, t: f64) -> Result<Vec<Frame>> {
        let root = self.plan.root();
        if !root.layers.iter().any(|l| l.def.common.mblur) {
            return Ok(vec![]);
        }
        let mb = root.mblur.unwrap_or_default();
        if mb.samples < 2 || mb.shutter <= 0.0 {
            return Ok(vec![]);
        }
        let open = mb.shutter / 360.0 / f64::from(self.plan.fps);
        (0..mb.samples).map(|i| self.ev.frame(ROOT, t + ((f64::from(i) + 0.5) / f64::from(mb.samples) - 0.5) * open)).collect()
    }

    pub fn draw_state(&mut self, state: &Frame, subs: &[Frame]) -> Result<()> {
        let ctx = Ctx { plan: self.plan, shaders: &self.shaders, programs: &self.programs, res: &self.res, subs };
        let canvas = self.surface.canvas();
        canvas.clear(skia_safe::Color::BLACK);
        let layer = comp_layer(&ctx, canvas, state).with_context(|| format!("drawing t={:.3}s", state.t))?;
        canvas.draw_image(layer.image(), (0, 0), None);
        self.gpu.context.flush_and_submit();
        Ok(())
    }

    pub fn pixels(&mut self, into: &mut Vec<u8>) -> Result<()> {
        into.resize(self.info.compute_min_byte_size(), 0);
        anyhow::ensure!(self.surface.read_pixels(&self.info, into, self.info.min_row_bytes(), (0, 0)), "pixel readback failed");
        Ok(())
    }

    /// A labelled contact sheet of the frames at `times`.
    pub fn sheet(&mut self, times: &[f64], cols: u32, thumb: u32) -> Result<Vec<u8>> {
        let root = self.plan.root();
        let tw = thumb as i32;
        let th = (f64::from(thumb) * f64::from(root.height) / f64::from(root.width)).round() as i32;
        let cols = cols.min(times.len() as u32).max(1) as i32;
        let rows = (times.len() as i32 + cols - 1) / cols;
        let gap = 10;
        let label = 22;
        let (sw, sh) = (cols * tw + (cols + 1) * gap, rows * (th + label) + (rows + 1) * gap);
        let mut sheet = self.gpu.frame_surface(ISize::new(sw, sh))?;
        sheet.canvas().clear(skia_safe::Color::from_rgb(24, 24, 28));
        let font = skia_safe::Font::from_typeface(skia_safe::FontMgr::new().legacy_make_typeface(None, skia_safe::FontStyle::normal()).context("no default font")?, 14.0);
        let mut ink = Paint::default();
        ink.set_color(skia_safe::Color::from_rgb(220, 220, 225)).set_anti_alias(true);
        for (i, t) in times.iter().enumerate() {
            self.draw_at(*t)?;
            let (c, r) = (i as i32 % cols, i as i32 / cols);
            let (x, y) = (gap + c * (tw + gap), gap + r * (th + label + gap));
            let image = self.surface.image_snapshot();
            let mut p = Paint::default();
            p.set_anti_alias(true);
            sheet.canvas().draw_image_rect_with_sampling_options(&image, None, skia_safe::Rect::from_xywh(x as f32, (y + label) as f32, tw as f32, th as f32), skia_safe::SamplingOptions::new(skia_safe::FilterMode::Linear, skia_safe::MipmapMode::Linear), &p);
            sheet.canvas().draw_str(format!("{t:.2}s  (frame {})", (t * f64::from(self.plan.fps)).round() as u32), (x as f32, (y + 15) as f32), &font, &ink);
        }
        self.gpu.context.flush_and_submit();
        let info = ImageInfo::new(ISize::new(sw, sh), ColorType::BGRA8888, AlphaType::Premul, None);
        let mut pixels = vec![0u8; info.compute_min_byte_size()];
        anyhow::ensure!(sheet.read_pixels(&info, &mut pixels, info.min_row_bytes(), (0, 0)), "pixel readback failed");
        let pixmap = Pixmap::new(&info, &mut pixels, info.min_row_bytes()).context("wrapping the sheet pixels")?;
        let mut png = Vec::new();
        anyhow::ensure!(png_encoder::encode(&pixmap, &mut png, &png_encoder::Options::default()), "PNG encoding failed");
        Ok(png)
    }

    pub fn png(&mut self) -> Result<Vec<u8>> {
        let mut pixels = Vec::new();
        self.pixels(&mut pixels)?;
        let pixmap = Pixmap::new(&self.info, &mut pixels, self.info.min_row_bytes()).context("wrapping the frame pixels")?;
        let mut png = Vec::new();
        anyhow::ensure!(png_encoder::encode(&pixmap, &mut png, &png_encoder::Options::default()), "PNG encoding failed");
        Ok(png)
    }
}

/// Render `frames` of `plan_file` into `out` with `workers` threads; each worker loads the plan itself because Skia
/// objects stay on their thread.
pub fn render_video(plan_file: &std::path::Path, codec: &Codec, frames: std::ops::Range<u32>, workers: u32, out: &std::path::Path) -> Result<()> {
    let plan = Plan::load(plan_file)?;
    let format = VideoFormat { width: plan.root().width as i32, height: plan.root().height as i32, fps: plan.fps };
    let fps = f64::from(plan.fps);
    let silent = out.with_extension("silent.mp4");
    let wav = out.with_extension("mix.wav");
    let has_sound = crate::audio::mix(&plan, frames.end as f64 / fps, &wav)?.is_some();
    drop(plan);
    let video_out: &std::path::Path = if has_sound { &silent } else { out };
    parallel::render(frames, workers, video_out, |range, part| {
        let plan = Plan::load(plan_file)?;
        let mut renderer = Renderer::new(&plan)?;
        let mut encoder = Encoder::spawn(part, format, codec)?;
        let mut pixels = Vec::new();
        for f in range {
            renderer.draw(f).with_context(|| format!("frame {f}"))?;
            renderer.pixels(&mut pixels)?;
            encoder.write(&pixels)?;
        }
        encoder.finish()
    })?;
    if has_sound {
        crate::audio::mux(&silent, &wav, out)?;
        std::fs::remove_file(&silent).ok();
        std::fs::remove_file(&wav).ok();
    }
    Ok(())
}

fn color4f(c: [f32; 4]) -> Color4f {
    Color4f::new(c[0] / 255.0, c[1] / 255.0, c[2] / 255.0, c[3])
}

/// Composite one composition frame into a transparent-or-backgrounded layer of the composition's size.
pub fn comp_layer(ctx: &Ctx, like: &Canvas, frame: &Frame) -> Result<Layer> {
    let comp = &ctx.plan.comps[&frame.comp];
    let (w, h) = (frame.width as i32, frame.height as i32);
    let info = ImageInfo::new(ISize::new(w, h), ColorType::RGBAF16, AlphaType::Premul, None);
    let mut acc = like.new_surface(&info, None).with_context(|| format!("no GPU memory for a {w}x{h} composite"))?;
    match frame.bg {
        Some(bg) => acc.canvas().clear(premul(color4f(bg))),
        None => acc.canvas().clear(skia_safe::Color::TRANSPARENT),
    };
    for i in draw_order(frame) {
        let st = &frame.layers[i];
        if matches!(st.content, Content::Adjustment) {
            if st.active && st.visible_adjustment() {
                post::adjust(ctx, like, &mut acc, frame, comp, st).with_context(|| comp.layers[i].label.clone())?;
            }
            continue;
        }
        if !st.visible || st.opacity <= 0.0 {
            continue;
        }
        let mut paint = Paint::default();
        if let Some(b) = st.blend_mode {
            paint.set_blend_mode(b.skia());
        }
        if comp.layers[i].def.common.mblur && !ctx.subs.is_empty() && frame.comp == crate::plan::ROOT {
            // average the layer over the open shutter, each sample with its own opacity
            let n = ctx.subs.len() as f32;
            let mut sum = like.new_surface(&info, None).with_context(|| format!("no GPU memory for a {w}x{h} blur accumulator"))?;
            sum.canvas().clear(skia_safe::Color::TRANSPARENT);
            let mut plus = Paint::default();
            plus.set_blend_mode(skia_safe::BlendMode::Plus);
            for sub in ctx.subs {
                let s = &sub.layers[i];
                if !s.visible || s.opacity <= 0.0 {
                    continue;
                }
                let img = post::layer_image(ctx, like, sub, comp, i).with_context(|| comp.layers[i].label.clone())?;
                plus.set_alpha_f(s.opacity as f32 / n);
                sum.canvas().draw_image(img.image(), (0, 0), Some(&plus));
            }
            acc.canvas().draw_image(sum.image_snapshot(), (0, 0), Some(&paint));
            continue;
        }
        let img = post::layer_image(ctx, like, frame, comp, i).with_context(|| comp.layers[i].label.clone())?;
        paint.set_alpha_f(st.opacity as f32);
        acc.canvas().draw_image(img.image(), (0, 0), Some(&paint));
    }
    let out = Layer::from_image(acc.image_snapshot());
    if comp.look.is_empty() {
        return Ok(out);
    }
    fx::chain(ctx, like, out, &comp.look, &frame.look, frame.t).context("look")
}

fn premul(c: Color4f) -> Color4f {
    Color4f::new(c.r * c.a, c.g * c.a, c.b * c.a, c.a)
}

/// Paint order: list order, except that a run of consecutive 3D layers is sorted far to near.
fn draw_order(frame: &Frame) -> Vec<usize> {
    let mut order = Vec::with_capacity(frame.layers.len());
    let mut i = 0;
    while i < frame.layers.len() {
        if frame.layers[i].three_d && frame.camera.is_some() {
            let mut j = i;
            while j < frame.layers.len() && frame.layers[j].three_d {
                j += 1;
            }
            let mut run: Vec<usize> = (i..j).collect();
            run.sort_by(|a, b| frame.layers[*b].depth.total_cmp(&frame.layers[*a].depth));
            order.extend(run);
            i = j;
        } else {
            order.push(i);
            i += 1;
        }
    }
    order
}

impl LayerState {
    fn visible_adjustment(&self) -> bool {
        self.active && self.opacity > 0.0
    }
}

