//! What a text layer's measured bounds mean to the rest of the engine: the rectangles of its `box` at one instant, and
//! After Effects' `sourceRectAtTime(t, includeExtents)` for a layer, which expressions will call.

use anyhow::{bail, ensure, Result};
use serde::Serialize;

use super::{state, CompPlan, Evaluator, LayerEnv};
use crate::{
    anim::prop::Rgba,
    eval::salt,
    plan::LayerPlan,
    schema::{text_box::TextBox, Align, Kind, P},
    text::{
        bounds::{Edges, SourceRect, TextBounds},
        boxfit::BoxFit,
    },
};

/// A text layer's `box` at one instant, in layer pixels.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct BoxState {
    /// `[left, top, right, bottom]` of each box: one per visible line or one for the block, none when no character shows.
    pub rects: Vec<Edges>,
    pub fill: Rgba,
    /// Corner radius as asked; a box is drawn with it held to half its shorter side.
    pub radius: f64,
    /// `[x, y]` as evaluated.
    pub padding: [f64; 2],
}

/// The box of `b` round `bounds`, for a text aligned `text_align`, at layer time `lt`.
pub(super) fn box_state<'a>(b: &TextBox, bounds: Option<&TextBounds>, text_align: Align, lt: f64, env: &dyn Fn(u32) -> LayerEnv<'a>) -> Result<BoxState> {
    let pad = |p: &P, salt: u32, axis: &str| -> Result<f64> {
        let v = p.0.at(lt, &env(salt))?;
        ensure!(v.is_finite() && v >= 0.0, "box padding {axis} is {v} at {lt:.3} s into the layer: padding is 0 or more pixels (an overshooting ease or an expression went below 0)");
        Ok(v)
    };
    let padding = [pad(&b.padding[0], salt::TEXT_BOX_PADDING_X, "x")?, pad(&b.padding[1], salt::TEXT_BOX_PADDING_Y, "y")?];
    let fit = BoxFit { padding, min_width: b.min_width, align: b.align.unwrap_or(text_align), per_line: b.per_line };
    Ok(BoxState { rects: bounds.map(|b| fit.rects(b)).unwrap_or_default(), fill: b.fill.0.at(lt, &env(salt::TEXT_BOX_FILL))?, radius: b.radius, padding })
}

/// After Effects' `sourceRectAtTime(t, includeExtents)`: the rectangle of the characters showing in text layer `l` at
/// composition time `t`, in the layer's own pixels (`left` and `top` from the text block's top-left). With
/// `include_extents` it is grown by half the widest stroke. `None` when no character is showing.
pub fn source_rect_at_time(ev: &Evaluator, c: &CompPlan, l: &LayerPlan, t: f64, include_extents: bool) -> Result<Option<SourceRect>> {
    let Kind::Text(tx) = &l.def.kind else {
        bail!("{}: a source rectangle is measured for text layers, and this is a {} layer", l.label, l.def.kind.name());
    };
    Ok(state(ev, c, l, tx, t)?.bounds.map(|b| b.source_rect(include_extents)))
}
