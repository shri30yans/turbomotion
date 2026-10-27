//! Motion blur in the frame graph. A frame is a pure function of time, so a blurred frame is the same frame drawn at
//! several moments and averaged: [`frames`] evaluates the root composition at each moment the shutter is open, and a layer
//! with the `mblur` switch is drawn from each of them, effects and masks included, then averaged (`ShutterAverage`).
//! Nothing here knows what moves; the evaluator already says where everything is at each moment.

use anyhow::Result;

use super::{build::{Builder, MAIN}, ImageId, Stage};
use crate::{
    eval::{Evaluator, Frame},
    plan::{Plan, ROOT},
    schema::Blur,
};

/// The root composition at each moment the shutter is open around the frame at `t`: empty when no layer asks for blur or
/// the shutter averages nothing.
pub fn frames(plan: &Plan, ev: &Evaluator, t: f64) -> Result<Vec<Frame>> {
    let root = plan.root();
    if !root.layers.iter().any(|l| l.def.common.mblur) {
        return Ok(vec![]);
    }
    let blur = root.blur.unwrap_or_default();
    blur.shutter().moments(t, f64::from(plan.fps)).into_iter().map(|moment| ev.frame(ROOT, moment)).collect()
}

impl Builder<'_> {
    /// Lay layer `i` on the composite as an average over the shutter, when it asks for blur and there is a shutter. False when
    /// the layer is to be drawn the usual way: its switch is off, there are no moments, or (with `motion_blur`) it is still.
    pub(super) fn composite_blurred(&mut self, i: usize) -> bool {
        let (comp, main) = (self.comp, self.frames[MAIN]);
        if !comp.layers[i].def.common.mblur || self.frames.len() < 2 || main.comp != ROOT {
            return false;
        }
        let state = &main.layers[i];
        let after_effects = matches!(comp.blur, Some(Blur::AfterEffects(_)));
        if after_effects && self.is_still(i) {
            return false;
        }
        // `mblur` draws only what shows at the frame time; `motion_blur` draws whatever shows at any moment of the shutter
        if !after_effects && !(state.visible && state.opacity > 0.0) {
            return false;
        }
        let showing: Vec<(usize, f64)> = (1..self.frames.len()).map(|k| (k, &self.frames[k].layers[i])).filter(|(_, s)| s.visible && s.opacity > 0.0).map(|(k, s)| (k, s.opacity)).collect();
        if showing.is_empty() && after_effects {
            return true;
        }
        let averaged = self.shutter(i, &showing);
        self.modify(Stage::Composite { blend: state.blend_mode, opacity: 1.0, placed: None }, MAIN, vec![averaged]);
        true
    }

    /// Layer `i` averaged over the moments at which it shows (`showing`: the moment and the layer's opacity then).
    fn shutter(&mut self, i: usize, showing: &[(usize, f64)]) -> ImageId {
        let reads = showing.iter().map(|&(frame, _)| self.layer_picture(frame, i, None)).collect();
        self.pass(Stage::ShutterAverage { opacities: showing.iter().map(|&(_, opacity)| opacity).collect() }, MAIN, reads)
    }

    /// Layer `i` is drawn the same at every moment of the shutter and at the frame time, so averaging its pictures would
    /// only draw it again and again (and the average of equal pictures is not always the picture, to the last bit).
    ///
    /// Certain, not guessed: the layer reads no other layer (no matte, no clip), nothing in it reads the clock, and its
    /// evaluated state, with the camera and lights it is seen by, is equal at all moments.
    fn is_still(&self, i: usize) -> bool {
        let (lp, main) = (&self.comp.layers[i], self.frames[MAIN]);
        let state = &main.layers[i];
        lp.matte.is_none()
            && lp.clip_to.is_none()
            && state.is_clockless()
            && self.frames[1..].iter().all(|f| f.camera == main.camera && f.lights == main.lights && f.layers[i].equals_apart_from_clock(state))
    }
}
