//! Building the graph for one frame: demand-driven, so what nothing reads is never drawn.

use std::collections::HashMap;

use super::{FxOwner, Graph, ImageId, Pass, Stage, Write};
use crate::{
    eval::{Content, Frame},
    fxlib::PlanFx,
    plan::CompPlan,
    render::draw_order,
    schema::MatteMode,
};

/// Where a layer's picture comes from: the frame itself or one of the shutter samples.
const MAIN: usize = 0;

struct Builder<'a> {
    comp: &'a CompPlan,
    frames: Vec<&'a Frame>,
    passes: Vec<Pass>,
    images: usize,
    /// A layer's finished picture per frame, so a layer that is a matte, a clip and a map is drawn once.
    pictures: HashMap<(usize, usize), ImageId>,
}

impl<'a> Builder<'a> {
    fn new(comp: &'a CompPlan, frames: Vec<&'a Frame>) -> Self {
        Builder { comp, frames, passes: vec![], images: 0, pictures: HashMap::new() }
    }

    fn image(&mut self) -> ImageId {
        self.images += 1;
        ImageId(self.images - 1)
    }

    fn pass(&mut self, stage: Stage, frame: usize, reads: Vec<ImageId>) -> ImageId {
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
    fn effects(&mut self, frame: usize, owner: FxOwner, fx: &[PlanFx], upto: usize, from: ImageId) -> ImageId {
        let layers = Self::distinct(fx[..upto].iter().flat_map(PlanFx::layers_read));
        let mut reads = vec![from];
        for &l in &layers {
            reads.push(self.layer_picture(frame, l));
        }
        self.pass(Stage::Effects { owner, upto, pictures: layers }, frame, reads)
    }

    /// The layer fully processed except for its opacity and blend: content, effects, masks and its own matte.
    fn layer_picture(&mut self, frame: usize, i: usize) -> ImageId {
        if let Some(&done) = self.pictures.get(&(frame, i)) {
            return done;
        }
        let (comp, f) = (self.comp, self.frames[frame]);
        let (st, lp) = (&f.layers[i], &comp.layers[i]);
        let mut img = self.pass(Stage::Paint { layer: i }, frame, vec![]);
        if !lp.fx.is_empty() {
            img = self.effects(frame, FxOwner::Layer(i), &lp.fx, lp.fx.len(), img);
        }
        if !lp.masks.is_empty() {
            let cov = self.pass(Stage::MaskCoverage { layer: i }, frame, vec![]);
            img = self.pass(Stage::Multiply, frame, vec![img, cov]);
        }
        if let Some(src) = lp.clip_to {
            img = self.through_matte(frame, img, src, MatteMode::Alpha);
        }
        if let Some((src, mode)) = lp.matte {
            img = self.through_matte(frame, img, src, mode);
        }
        if let (Some(reflect), Some(bounds)) = (st.reflect, st.bounds) {
            img = self.pass(Stage::Reflect { bounds, reflect }, frame, vec![img]);
        }
        if let (Some(clip), Some(bounds)) = (st.clip, st.bounds) {
            img = self.pass(Stage::Wipe { bounds, clip }, frame, vec![img]);
        }
        self.pictures.insert((frame, i), img);
        img
    }

    fn through_matte(&mut self, frame: usize, img: ImageId, src: usize, mode: MatteMode) -> ImageId {
        let source = self.layer_picture(frame, src);
        let opacity = self.frames[frame].layers[src].opacity;
        let cov = self.pass(Stage::MatteCoverage { mode, opacity }, frame, vec![source]);
        self.pass(Stage::Multiply, frame, vec![img, cov])
    }

    /// An adjustment layer: what is composited so far becomes `lerp(below, effects(below), coverage × opacity)`.
    fn adjustment(&mut self, i: usize) {
        let lp = &self.comp.layers[i];
        let below = self.pass(Stage::Snapshot, MAIN, vec![]);
        let changed = self.effects(MAIN, FxOwner::Adjustment(i), &lp.fx, lp.fx.len(), below);
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
                reads.push(self.layer_picture(frame, i));
                opacities.push(s.opacity);
            }
        }
        self.pass(Stage::ShutterAverage { opacities }, MAIN, reads)
    }

    /// Every layer composited in paint order, then the composition's look.
    fn composition(&mut self) -> ImageId {
        let (comp, frame) = (self.comp, self.frames[MAIN]);
        self.modify(Stage::Backdrop, MAIN, vec![]);
        for i in draw_order(frame) {
            let st = &frame.layers[i];
            if matches!(st.content, Content::Adjustment) {
                if st.active && st.opacity > 0.0 {
                    self.adjustment(i);
                }
                continue;
            }
            if !st.visible || st.opacity <= 0.0 {
                continue;
            }
            let blend = st.blend_mode;
            if comp.layers[i].def.common.mblur && self.frames.len() > 1 && frame.comp == crate::plan::ROOT {
                let averaged = self.shutter(i);
                self.modify(Stage::Composite { blend, opacity: 1.0 }, MAIN, vec![averaged]);
            } else {
                let picture = self.layer_picture(MAIN, i);
                self.modify(Stage::Composite { blend, opacity: st.opacity }, MAIN, vec![picture]);
            }
        }
        let flat = self.pass(Stage::Snapshot, MAIN, vec![]);
        if comp.look.is_empty() {
            flat
        } else {
            self.effects(MAIN, FxOwner::Look, &comp.look, comp.look.len(), flat)
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
    let out = b.layer_picture(MAIN, layer);
    b.finish(vec![out])
}

/// The graph for what effect `effect` of `layer` is given: the layer's picture after the effects before it, then the
/// pictures of the layers its shader samples (not its composite mask, which a history frame does not apply); the second
/// return value names those layers.
pub fn effect_inputs_graph<'a>(comp: &'a CompPlan, frame: &'a Frame, layer: usize, effect: usize) -> (Graph<'a>, Vec<usize>) {
    let mut b = Builder::new(comp, vec![frame]);
    let fx = &comp.layers[layer].fx;
    let mut img = b.pass(Stage::Paint { layer }, MAIN, vec![]);
    if effect > 0 {
        img = b.effects(MAIN, FxOwner::Layer(layer), fx, effect, img);
    }
    let read = Builder::distinct(fx[effect].inputs_read());
    let mut outputs = vec![img];
    outputs.extend(read.iter().map(|&l| b.layer_picture(MAIN, l)));
    (b.finish(outputs), read)
}
