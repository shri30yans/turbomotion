//! Frame history for feedback effects (`@input name history=N depth=D`).
//!
//! A frame is still a pure function of time. The effect's output at time `t` with recursion level `d` is
//!
//! ```text
//! out(t, 0) = the effect's input at t
//! out(t, d) = effect(input at t, out(t - 1/fps, d-1) .. out(t - N/fps, d-1))      for d >= 1
//! ```
//!
//! and what the effect is shown at `t` is `out(t, D)`. Times before 0 repeat the input at `t`. Because nothing but `t`
//! enters, drawing frame F after a seek gives exactly the pixels playing from frame 0 gives, and the recursion is at most
//! `D` levels deep however far into the film the frame is. Every `(time, level)` is drawn once and kept in a cache that
//! only ever holds a recomputation, never a different picture.

use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use anyhow::{bail, Context, Result};
use skia_safe::Canvas;
use turbomotion_core::layer::Layer;

use super::{fx, graph, Ctx};
use crate::{
    eval::{fx::FxOpState, Frame},
    fxlib::FxOp,
};

/// What the cache may hold before it drops its oldest entries.
const BUDGET_BYTES: usize = 512 << 20;
/// A half-float RGBA pixel.
const BYTES_PER_PIXEL: usize = 8;
/// History times are keyed in whole microseconds, so `t - 1/fps - 1/fps` and `t - 2/fps` are one time.
const TICKS_PER_SECOND: f64 = 1e6;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    comp: String,
    layer: usize,
    effect: usize,
    tick: i64,
    what: What,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum What {
    Inputs,
    Output(u32),
}

/// What one effect is given at one time: the frame it was evaluated in, its input picture, and the layers it reads.
struct Inputs {
    frame: Frame,
    image: Layer,
    pictures: Vec<(usize, Layer)>,
}

enum Entry {
    Inputs(Rc<Inputs>),
    Output(Layer),
}

/// Pictures drawn for earlier times, oldest dropped first.
#[derive(Default)]
pub struct Cache {
    entries: HashMap<Key, (Entry, usize)>,
    order: VecDeque<Key>,
    bytes: usize,
}

impl Cache {
    fn put(&mut self, key: Key, entry: Entry, bytes: usize) {
        self.bytes += bytes;
        self.order.push_back(key.clone());
        self.entries.insert(key, (entry, bytes));
        while self.bytes > BUDGET_BYTES {
            let Some(old) = self.order.pop_front() else { break };
            if let Some((_, freed)) = self.entries.remove(&old) {
                self.bytes -= freed;
            }
        }
    }
}

fn tick(t: f64) -> i64 {
    (t * TICKS_PER_SECOND).round() as i64
}

fn picture_bytes(l: &Layer) -> usize {
    l.width() as usize * l.height() as usize * BYTES_PER_PIXEL
}

/// The effect being drawn: which layer, which entry of its list, and what its history input asks for.
#[derive(Clone, Copy)]
pub struct Spec {
    pub layer: usize,
    pub effect: usize,
    pub frames: u32,
    pub depth: u32,
}

/// The pictures a history input is given at the frame being drawn, which is level `spec.depth`.
pub fn previous(ctx: &Ctx, like: &Canvas, frame: &Frame, current: &Layer, spec: Spec) -> Result<Vec<Layer>> {
    lookback(ctx, like, &frame.comp, frame.t, current, spec, spec.depth)
}

/// `out(t - k/fps, level - 1)` for k = 1..=frames: what an effect at `level` is shown of its past. Before time 0 the
/// past is `current`, the effect's input at `t`.
fn lookback(ctx: &Ctx, like: &Canvas, comp: &str, t: f64, current: &Layer, spec: Spec, level: u32) -> Result<Vec<Layer>> {
    (1..=spec.frames)
        .map(|k| {
            let earlier = t - f64::from(k) / f64::from(ctx.plan.fps);
            if tick(earlier) < 0 {
                Ok(current.clone())
            } else {
                output(ctx, like, comp, earlier, spec, level - 1)
            }
        })
        .collect()
}

fn inputs(ctx: &Ctx, like: &Canvas, comp: &str, t: f64, spec: Spec) -> Result<Rc<Inputs>> {
    let key = Key { comp: comp.to_string(), layer: spec.layer, effect: spec.effect, tick: tick(t), what: What::Inputs };
    if let Some((Entry::Inputs(hit), _)) = ctx.history.borrow().entries.get(&key) {
        return Ok(hit.clone());
    }
    let frame = ctx.ev.frame(comp, t).with_context(|| format!("evaluating {comp:?} at {t:.3}s for effect history"))?;
    let (image, pictures) = graph::effect_inputs(ctx, like, &frame, spec.layer, spec.effect)?;
    let bytes = picture_bytes(&image) * (1 + pictures.len());
    let made = Rc::new(Inputs { frame, image, pictures });
    ctx.history.borrow_mut().put(key, Entry::Inputs(made.clone()), bytes);
    Ok(made)
}

/// `out(t, depth)`.
fn output(ctx: &Ctx, like: &Canvas, comp: &str, t: f64, spec: Spec, depth: u32) -> Result<Layer> {
    let key = Key { comp: comp.to_string(), layer: spec.layer, effect: spec.effect, tick: tick(t), what: What::Output(depth) };
    if let Some((Entry::Output(hit), _)) = ctx.history.borrow().entries.get(&key) {
        return Ok(hit.clone());
    }
    let at = inputs(ctx, like, comp, t, spec)?;
    let drawn = if depth == 0 {
        at.image.clone()
    } else {
        let plan_comp = &ctx.plan.comps[comp];
        let (FxOp::Plugin(plan), FxOpState::Plugin(state)) = (&plan_comp.layers[spec.layer].fx[spec.effect].op, &at.frame.layers[spec.layer].fx[spec.effect].op) else {
            bail!("effect {} of layer {} reads its own past but is not a shader plugin", spec.effect, plan_comp.layers[spec.layer].label)
        };
        let maps: Vec<(usize, &Layer)> = at.pictures.iter().map(|(l, p)| (*l, p)).collect();
        let scope = fx::Scope { ctx, like, frame: &at.frame, layer: Some(spec.layer), pictures: &maps };
        let past = lookback(ctx, like, comp, t, &at.image, spec, depth)?;
        fx::plugin_with(&scope, &at.image, plan, state, at.frame.layers[spec.layer].local_t, past)?
    };
    ctx.history.borrow_mut().put(key, Entry::Output(drawn.clone()), picture_bytes(&drawn));
    Ok(drawn)
}
