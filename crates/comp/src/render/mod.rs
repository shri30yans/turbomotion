//! Rendering: `Frame → pixels`. A layer is painted into its own comp-sized half-float image through its screen matrix
//! (so vectors and text stay sharp at any scale and 3D layers are projected by the GPU), then takes its effects,
//! masks and matte, and lands on the composite with its opacity and blend mode. With `effect_space: "layer"` the
//! effects and masks come first, on the layer's own pixels (`local`), and the matrix is applied after them.

mod blend;
mod composite;
mod content;
mod fx;
pub mod graph;
mod history;
mod layout_masks;
mod light;
mod local;
mod media;
mod particles;
mod passgeo;
mod post;
mod programs;
mod res;
mod shape;
mod sheet;
mod text;

#[cfg(test)]
mod backdrop_tests;
#[cfg(test)]
mod blend_tests;
#[cfg(test)]
mod color_media_tests;
#[cfg(test)]
mod color_tests;
#[cfg(test)]
mod composite_tests;
#[cfg(test)]
mod fx_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod layer_space_plugin_tests;
#[cfg(test)]
mod layer_space_tests;
#[cfg(test)]
mod testkit;
#[cfg(test)]
mod tests;

pub use layout_masks::LayoutMask;

use anyhow::{Context, Result};
use skia_safe::{png_encoder, AlphaType, Color4f, ColorType, ISize, ImageInfo, Pixmap, Surface};
use turbomotion_core::{
    encode::{Codec, Encoder, VideoFormat},
    color::Pipeline,
    gpu::Gpu,
    layer::working_surface,
    parallel,
    shaders::Shaders,
};

use crate::{
    eval::{Evaluator, Frame},
    plan::{Plan, ROOT},
};

pub struct Renderer<'a> {
    plan: &'a Plan,
    gpu: Gpu,
    /// The frame as it leaves: tagged with the output profile when the film is colour-managed.
    surface: Surface,
    info: ImageInfo,
    /// The canvas every picture of the root composition is made beside: it carries the working profile.
    work: Surface,
    pipeline: Pipeline,
    shaders: Shaders,
    programs: programs::Programs,
    res: res::Resources,
    packs: Vec<Box<dyn crate::pack::Pack>>,
    ev: Evaluator<'a>,
    history: std::cell::RefCell<history::Cache>,
}

/// Everything painting needs besides the canvas.
pub struct Ctx<'a> {
    pub plan: &'a Plan,
    pub shaders: &'a Shaders,
    pub programs: &'a programs::Programs,
    pub res: &'a res::Resources,
    pub packs: &'a [Box<dyn crate::pack::Pack>],
    /// The root frame re-evaluated across the shutter, for layers with motion blur (empty when there is none).
    pub subs: &'a [Frame],
    /// Evaluates frames at other times, for effects that read their own past.
    pub ev: &'a Evaluator<'a>,
    pub history: &'a std::cell::RefCell<history::Cache>,
}

impl<'a> Renderer<'a> {
    pub fn new(plan: &'a Plan) -> Result<Self> {
        let mut gpu = Gpu::new()?;
        let root = plan.root();
        let size = ISize::new(root.width as i32, root.height as i32);
        let pipeline = root.color;
        let mut surface = gpu.frame_surface(size, pipeline.output_space())?;
        let info = ImageInfo::new(size, ColorType::BGRA8888, AlphaType::Premul, pipeline.output_space());
        let work = working_surface(surface.canvas(), pipeline.working_space())?;
        Ok(Renderer { plan, gpu, surface, info, work, pipeline, shaders: Shaders::compile().context("compiling the platform shaders")?, programs: programs::Programs::compile()?, res: res::Resources::new(pipeline.is_managed()), packs: crate::pack::instances(), ev: Evaluator::new(plan), history: Default::default() })
    }

    /// Draw frame number `frame` of the root composition into the surface.
    pub fn draw(&mut self, frame: u32) -> Result<()> {
        let t = f64::from(frame) / f64::from(self.plan.fps);
        let state = self.ev.frame(ROOT, t).with_context(|| format!("evaluating frame {frame}"))?;
        let subs = shutter_frames(self.plan, &self.ev, t).with_context(|| format!("evaluating the shutter of frame {frame}"))?;
        self.draw_state(&state, &subs)
    }

    /// Draw the frame nearest to `t` seconds.
    pub fn draw_at(&mut self, t: f64) -> Result<()> {
        self.draw((t * f64::from(self.plan.fps)).round().max(0.0) as u32)
    }

    pub fn draw_state(&mut self, state: &Frame, subs: &[Frame]) -> Result<()> {
        let ctx = Ctx { plan: self.plan, shaders: &self.shaders, programs: &self.programs, res: &self.res, packs: &self.packs, subs, ev: &self.ev, history: &self.history };
        let like = self.work.canvas();
        let mut layer = graph::comp_picture(&ctx, like, state).with_context(|| format!("drawing t={:.3}s", state.t))?;
        if self.pipeline.converts_at_output() {
            layer = post::over_black(like, &layer)?;
        }
        let canvas = self.surface.canvas();
        canvas.clear(skia_safe::Color::BLACK);
        canvas.draw_image(layer.image(), (0, 0), None);
        self.gpu.context.flush_and_submit();
        Ok(())
    }

    /// Where each foreground layer of every `layout` composition really is at `t`, as a coarse coverage mask (a quarter of the
    /// composition size). This is what `check --layout` compares: pixels, so charts, maps and text from any source count.
    pub fn layout_masks(&mut self, t: f64) -> Result<Vec<LayoutMask>> {
        let state = self.ev.frame(ROOT, t)?;
        let ctx = Ctx { plan: self.plan, shaders: &self.shaders, programs: &self.programs, res: &self.res, packs: &self.packs, subs: &[], ev: &self.ev, history: &self.history };
        let mut out = Vec::new();
        layout_masks::collect_masks(&ctx, self.work.canvas(), &state, &mut out)?;
        Ok(out)
    }

    pub fn pixels(&mut self, into: &mut Vec<u8>) -> Result<()> {
        into.resize(self.info.compute_min_byte_size(), 0);
        anyhow::ensure!(self.surface.read_pixels(&self.info, into, self.info.min_row_bytes(), (0, 0)), "pixel readback failed");
        Ok(())
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

/// The root composition at evenly spread moments while the shutter is open, when any layer asks for motion blur.
pub(crate) fn shutter_frames(plan: &Plan, ev: &Evaluator, t: f64) -> Result<Vec<Frame>> {
    let root = plan.root();
    if !root.layers.iter().any(|l| l.def.common.mblur) {
        return Ok(vec![]);
    }
    let mb = root.mblur.unwrap_or_default();
    if mb.samples < 2 || mb.shutter <= 0.0 {
        return Ok(vec![]);
    }
    let open = mb.shutter / 360.0 / f64::from(plan.fps);
    (0..mb.samples).map(|i| ev.frame(ROOT, t + ((f64::from(i) + 0.5) / f64::from(mb.samples) - 0.5) * open)).collect()
}

/// Render `frames` of `plan_file` into `out` with `workers` threads; each worker loads the plan itself because Skia
/// objects stay on their thread.
pub fn render_video(plan_file: &std::path::Path, codec: &Codec, frames: std::ops::Range<u32>, workers: u32, out: &std::path::Path) -> Result<()> {
    let plan = Plan::load(plan_file)?;
    let format = VideoFormat { width: plan.root().width as i32, height: plan.root().height as i32, fps: plan.fps, profile: plan.root().color.is_managed().then_some(plan.root().color.output) };
    let fps = f64::from(plan.fps);
    let silent = out.with_extension(format!("silent.{}", out.extension().and_then(|e| e.to_str()).unwrap_or("mp4")));
    let wav = out.with_extension("mix.wav");
    let has_sound = crate::audio::mix_range(&plan, frames.start as f64 / fps..frames.end as f64 / fps, &wav)?.is_some();
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
            if codec.has_alpha() {
                turbomotion_core::encode::unpremultiply(&mut pixels);
            }
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

/// Paint order: list order, except that a run of layers standing in the 3D space is sorted far to near.
///
/// A 2D layer between two 3D layers is a wall: the layers on either side of it cannot sort against each other,
/// which is After Effects' rule. A camera or a light has no place in the scene, so it neither breaks a run nor
/// takes part in the sorting; it stays in the slot the timeline gave it.
pub(crate) fn draw_order(frame: &Frame) -> Vec<usize> {
    let mut order = Vec::with_capacity(frame.layers.len());
    let mut i = 0;
    while i < frame.layers.len() {
        if frame.layers[i].in_3d && frame.camera.is_some() {
            let mut j = i;
            while j < frame.layers.len() && (frame.layers[j].in_3d || sceneless(&frame.layers[j])) {
                j += 1;
            }
            let mut run: Vec<usize> = (i..j).filter(|k| frame.layers[*k].in_3d).collect();
            run.sort_by(|a, b| frame.layers[*b].depth.total_cmp(&frame.layers[*a].depth));
            let mut far = run.into_iter();
            order.extend((i..j).map(|k| {
                if frame.layers[k].in_3d {
                    far.next().expect("one sorted layer per 3D layer in the run")
                } else {
                    k
                }
            }));
            i = j;
        } else {
            order.push(i);
            i += 1;
        }
    }
    order
}

/// A layer that is not part of the scene: a camera or a light.
fn sceneless(layer: &crate::eval::LayerState) -> bool {
    matches!(layer.kind, "camera" | "light")
}
