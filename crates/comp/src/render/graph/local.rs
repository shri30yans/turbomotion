//! The layer-space half of the builder: a layer's content, effects and masks in its own pixels.

use super::{build::Builder, FxOwner, ImageId, Stage};

impl Builder<'_> {
    /// A layer-space layer's content, effects and masks, in the picture the evaluator sized for it. Its effects and masks
    /// come before its matrix, which is applied by `Place` (or by the composite).
    pub(super) fn local_content(&mut self, frame: usize, i: usize, behind: Option<ImageId>) -> ImageId {
        let lp = &self.comp.layers[i];
        let mut img = self.pass(Stage::PaintLocal { layer: i }, frame, vec![]);
        if !lp.fx.is_empty() {
            img = self.effects(frame, FxOwner::Layer(i), &lp.fx, lp.fx.len(), img, behind);
        }
        if !lp.masks.is_empty() {
            let cov = self.pass(Stage::MaskCoverageLocal { layer: i }, frame, vec![]);
            img = self.pass(Stage::Multiply, frame, vec![img, cov]);
        }
        img
    }
}
