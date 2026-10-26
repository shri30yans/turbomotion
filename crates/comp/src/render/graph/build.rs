//! Building the graph for one frame: demand-driven, so what nothing reads is never drawn.

use std::collections::HashMap;

use super::{FxOwner, Graph, ImageId, Pass, Stage, Write};
use crate::{
    eval::{Content, Frame},
    fxlib::PlanFx,
    plan::CompPlan,
    render::{draw_order, post},
    schema::MatteMode,
};

/// Whether an effect list samples the composite below the layer (`backdrop`).
pub fn uses_backdrop(fx: &[PlanFx]) -> bool {
    fx.iter().any(|f| f.name() == "backdrop")
}

/// Where a layer's picture comes from: the frame itself or one of the shutter samples.
const MAIN: usize = 0;

pub(super) struct Builder<'a> {
    pub(super) comp: &'a CompPlan,
    pub(super) frames: Vec<&'a Frame>,
    passes: Vec<Pass>,
    images: usize,
    /// A layer's finished picture per frame, so a layer that is a matte, a clip and a map is drawn once.
    pictures: HashMap<(usize, usize), ImageId>,
    /// A layer's picture per frame as the builder first made it: in layer space it may not be placed in the frame yet.
    parts: HashMap<(usize, usize), Parts>,
}

/// A layer's picture, and whether it is still in the layer's own pixels (layer space, with nothing in the frame's space to
/// apply yet), so that compositing can place it directly.
#[derive(Clone, Copy)]
pub(super) struct Parts {
    pub image: ImageId,
    pub unplaced: bool,
}

impl<'a> Builder<'a> {
    fn new(comp: &'a CompPlan, frames: Vec<&'a Frame>) -> Self {
        Builder { comp, frames, passes: vec![], images: 0, pictures: HashMap::new(), parts: HashMap::new() }
    }

    fn image(&mut self) -> ImageId {
        self.images += 1;
        ImageId(self.images - 1)
    }

    pub(super) fn pass(&mut self, stage: Stage, frame: usize, reads: Vec<ImageId>) -> ImageId {
        let out = self.image();
        self.passes.push(Pass { stage, frame, reads, write: Write::New(out) });
        out
    }

    fn modify(&mut self, stage: Stage, frame: usize, reads: Vec<ImageId>) {
        self.passes.push(Pass { stage, frame, reads, write: Write::Composite });
    }

    fn finish(self, outputs: Vec<ImageId>) -> Graph<'a> {
        Graph { comp: self.comp, frames: self.frames, passes: self.passes, images: self.images, outputs }
    }

    /// `layers` without repeats, in order of first use.
    fn distinct(layers: impl IntoIterator<Item = usize>) -> Vec<usize> {
        let mut once: Vec<usize> = vec![];
        for index in layers {
            if !once.contains(&index) {
                once.push(index);
            }
        }
        once
    }

    /// Effects `0..upto` of `owner`'s list applied to `from`, with the pictures they read drawn first.
    pub(super) fn effects(&mut self, frame: usize, owner: FxOwner, fx: &[PlanFx], upto: usize, from: ImageId, behind: Option<ImageId>) -> ImageId {
        let layers = Self::distinct(fx[..upto].iter().flat_map(PlanFx::layers_read));
        let mut reads = vec![from];
        if let Some(b) = behind {
            reads.push(b);
        }
        for &l in &layers {
            reads.push(self.layer_picture(frame, l, None));
        }
        self.pass(Stage::Effects { owner, upto, pictures: layers, behind: behind.is_some() }, frame, reads)
    }

    /// The layer fully processed except for its opacity and blend: content, effects, masks and its own matte.
    pub(super) fn layer_picture(&mut self, frame: usize, i: usize, behind: Option<ImageId>) -> ImageId {
        if behind.is_none() {
            if let Some(&done) = self.pictures.get(&(frame, i)) {
                return done;
            }
        }
        let parts = self.layer_parts(frame, i, behind);
        let done = if parts.unplaced { self.pass(Stage::Place { layer: i }, frame, vec![parts.image]) } else { parts.image };
        if behind.is_none() {
            self.pictures.insert((frame, i), done);
        }
        done
    }

    /// As [`Self::layer_picture`], but a layer-space layer with nothing to do in the frame's space after its effects is
    /// returned still in its own pixels, for the composite to place.
    pub(super) fn layer_parts(&mut self, frame: usize, i: usize, behind: Option<ImageId>) -> Parts {
        if behind.is_none() {
            if let Some(&made) = self.parts.get(&(frame, i)) {
                return made;
            }
        }
        let (comp, f) = (self.comp, self.frames[frame]);
        let (st, lp) = (&f.layers[i], &comp.layers[i]);
        let reflect = st.reflect.zip(st.bounds);
        let wipe = st.clip.zip(st.bounds);
        let in_frame_space = lp.clip_to.is_some() || lp.matte.is_some() || reflect.is_some() || wipe.is_some() || post::needs_finish(st);
        let local = st.local.is_some();
        let mut img = if local { self.local_content(frame, i, behind) } else { self.screen_content(frame, i, behind) };
        let mut unplaced = local;
        if local && in_frame_space {
            img = self.pass(Stage::Place { layer: i }, frame, vec![img]);
            if post::needs_finish(st) {
                img = self.pass(Stage::Finish { layer: i }, frame, vec![img]);
            }
            unplaced = false;
        }
        if let Some(src) = lp.clip_to {
            img = self.through_matte(frame, img, src, MatteMode::Alpha);
        }
        if let Some((src, mode)) = lp.matte {
            img = self.through_matte(frame, img, src, mode);
        }
        if let Some((reflect, bounds)) = reflect {
            img = self.pass(Stage::Reflect { bounds, reflect }, frame, vec![img]);
        }
        if let Some((clip, bounds)) = wipe {
            img = self.pass(Stage::Wipe { bounds, clip }, frame, vec![img]);
        }
        let made = Parts { image: img, unplaced };
        if behind.is_none() {
            self.parts.insert((frame, i), made);
        }
        made
    }

    /// Content, effects and masks of a layer in the frame's space: painted frame-sized through the layer's matrix.
    fn screen_content(&mut self, frame: usize, i: usize, behind: Option<ImageId>) -> ImageId {
        let lp = &self.comp.layers[i];
        let mut img = self.pass(Stage::Paint { layer: i }, frame, vec![]);
        if !lp.fx.is_empty() {
            img = self.effects(frame, FxOwner::Layer(i), &lp.fx, lp.fx.len(), img, behind);
        }
        if !lp.masks.is_empty() {
            let cov = self.pass(Stage::MaskCoverage { layer: i }, frame, vec![]);
            img = self.pass(Stage::Multiply, frame, vec![img, cov]);
        }
        img
    }

    fn through_matte(&mut self, frame: usize, img: ImageId, src: usize, mode: MatteMode) -> ImageId {
        let source = self.layer_picture(frame, src, None);
        let opacity = self.frames[frame].layers[src].opacity;
        let cov = self.pass(Stage::MatteCoverage { mode, opacity }, frame, vec![source]);
        self.pass(Stage::Multiply, frame, vec![img, cov])
    }

    /// An adjustment layer: what is composited so far becomes `lerp(below, effects(below), coverage × opacity)`.
    fn adjustment(&mut self, i: usize) {
        let lp = &self.comp.layers[i];
        let below = self.pass(Stage::Snapshot, MAIN, vec![]);
        let changed = self.effects(MAIN, FxOwner::Adjustment(i), &lp.fx, lp.fx.len(), below, None);
        let cov = self.pass(Stage::AdjustCoverage { layer: i }, MAIN, vec![]);
        self.modify(Stage::AdjustMix, MAIN, vec![below, changed, cov]);
    }

    /// Layer `i` averaged over the shutter samples at which it is visible.
    fn shutter(&mut self, i: usize) -> ImageId {
        let mut reads = vec![];
        let mut opacities = vec![];
        for frame in 1..self.frames.len() {
            let s = &self.frames[frame].layers[i];
            if s.visible && s.opacity > 0.0 {
                reads.push(self.layer_picture(frame, i, None));
                opacities.push(s.opacity);
            }
        }
        self.pass(Stage::ShutterAverage { opacities }, MAIN, reads)
    }

    /// Lay one visible layer on the composite (adjustment, motion-blur shutter, or a normal layer).
    fn composite_layer(&mut self, frame: usize, i: usize) {
        let (comp, f) = (self.comp, self.frames[frame]);
        let st = &f.layers[i];
        if matches!(st.content, Content::Adjustment) {
            if st.active && st.opacity > 0.0 {
                self.adjustment(i);
            }
            return;
        }
        if !st.visible || st.opacity <= 0.0 {
            return;
        }
        let blend = st.blend_mode;
        if comp.layers[i].def.common.mblur && self.frames.len() > 1 && f.comp == crate::plan::ROOT {
            let averaged = self.shutter(i);
            self.modify(Stage::Composite { blend, opacity: 1.0, placed: None }, frame, vec![averaged]);
            return;
        }
        let behind = if uses_backdrop(&comp.layers[i].fx) { Some(self.pass(Stage::Snapshot, frame, vec![])) } else { None };
        let mut parts = self.layer_parts(frame, i, behind);
        // a mode that reaches past the layer's own pixels (a dissolve, a stencil) sees the layer as the frame sees it
        if parts.unplaced && blend.is_some_and(|b| b.needs_frame()) {
            parts = Parts { image: self.pass(Stage::Place { layer: i }, frame, vec![parts.image]), unplaced: false };
        }
        let placed = parts.unplaced.then_some(i);
        self.modify(Stage::Composite { blend, opacity: st.opacity, placed }, frame, vec![parts.image]);
    }

    /// Every layer composited in paint order, then the composition's look.
    fn composition(&mut self) -> ImageId {
        let (comp, frame) = (self.comp, self.frames[MAIN]);
        self.modify(Stage::Backdrop, MAIN, vec![]);
        for i in draw_order(frame) {
            self.composite_layer(MAIN, i);
        }
        let flat = self.pass(Stage::Snapshot, MAIN, vec![]);
        if comp.look.is_empty() {
            flat
        } else {
            self.effects(MAIN, FxOwner::Look, &comp.look, comp.look.len(), flat, None)
        }
    }
}

/// The graph that draws a whole composition frame; `samples` are the shutter-sample frames (empty without motion blur).
pub fn comp_graph<'a>(comp: &'a CompPlan, frame: &'a Frame, samples: &'a [Frame]) -> Graph<'a> {
    let mut b = Builder::new(comp, std::iter::once(frame).chain(samples).collect());
    let out = b.composition();
    b.finish(vec![out])
}

/// The graph that draws one layer's finished picture (before opacity and blend).
pub fn layer_graph<'a>(comp: &'a CompPlan, frame: &'a Frame, layer: usize) -> Graph<'a> {
    let mut b = Builder::new(comp, vec![frame]);
    let order = draw_order(frame);
    let pos = order.iter().position(|&l| l == layer).expect("the layer is in paint order");
    if uses_backdrop(&comp.layers[layer].fx) {
        b.modify(Stage::Backdrop, MAIN, vec![]);
        for &i in order.iter().take(pos) {
            b.composite_layer(MAIN, i);
        }
    }
    let behind = if uses_backdrop(&comp.layers[layer].fx) { Some(b.pass(Stage::Snapshot, MAIN, vec![])) } else { None };
    let out = b.layer_picture(MAIN, layer, behind);
    b.finish(vec![out])
}

/// The graph for what effect `effect` of `layer` is given: the layer's picture after the effects before it, then the
/// pictures of the layers its shader samples (not its composite mask, which a history frame does not apply); the second
/// return value names those layers.
pub fn effect_inputs_graph<'a>(comp: &'a CompPlan, frame: &'a Frame, layer: usize, effect: usize) -> (Graph<'a>, Vec<usize>) {
    let mut b = Builder::new(comp, vec![frame]);
    let fx = &comp.layers[layer].fx;
    let needs_behind = uses_backdrop(&fx[..effect]);
    if needs_behind {
        let order = draw_order(frame);
        let pos = order.iter().position(|&l| l == layer).expect("the layer is in paint order");
        b.modify(Stage::Backdrop, MAIN, vec![]);
        for &i in order.iter().take(pos) {
            b.composite_layer(MAIN, i);
        }
    }
    let behind = if needs_behind { Some(b.pass(Stage::Snapshot, MAIN, vec![])) } else { None };
    let mut img = b.pass(Stage::Paint { layer }, MAIN, vec![]);
    if effect > 0 {
        img = b.effects(MAIN, FxOwner::Layer(layer), fx, effect, img, behind);
    }
    let read = Builder::distinct(fx[effect].inputs_read());
    let mut outputs = vec![img];
    outputs.extend(read.iter().map(|&l| b.layer_picture(MAIN, l, None)));
    (b.finish(outputs), read)
}
