//! The picture a layer-space layer is drawn into (`effect_space: "layer"`): the layer's content box grown by what its
//! effects reach, rounded out to whole pixels. This is the whole of the sizing logic, so a test can check that a layer
//! costs the size of its own pixels plus its effects' growth and not the size of the frame.

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde::Serialize;
use turbomotion_core::effects::{grown, Growth, Space};

use super::fx::{FxOpState, FxState};
use crate::{fxlib::{FxOp, Library, PlanFx}, plan::LayerPlan};

/// The longest side a layer's picture may have, and the most pixels it may hold (half-float RGBA is 8 bytes a pixel, so
/// the area limit is half a gigabyte). Past either the effects ask for more than a layer can hold: lower a radius.
pub const MAX_SIDE: i32 = 16384;
pub const MAX_PIXELS: i64 = 64 * 1024 * 1024;

/// Where a layer-space layer's picture sits in the layer's own pixels, and how big it is.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct LocalBuffer {
    /// `[left, top, right, bottom]` of what the layer draws, in layer pixels.
    pub bounds_local: [f64; 4],
    /// The same box grown by every effect in the chain.
    pub bounds_post_fx: [f64; 4],
    /// Where `bounds_post_fx` lands on screen (its bounding box), when it is in front of the camera.
    pub bounds_screen: Option<[f64; 4]>,
    /// The layer pixel at the picture's top-left corner: `bounds_post_fx` rounded out.
    pub origin: [i32; 2],
    /// The picture's width and height in pixels.
    pub size: [i32; 2],
}

impl LocalBuffer {
    /// The buffer for content `content` (none: the layer draws nothing, which still gets a 1x1 picture) whose chain
    /// reaches `growth` past it.
    pub fn plan(content: Option<[f64; 4]>, growth: Growth) -> Result<LocalBuffer> {
        let bounds_local = content.unwrap_or([0.0; 4]);
        ensure!(bounds_local.iter().chain(&growth).all(|v| v.is_finite()), "the layer's box or its effects' reach is not a finite number ({bounds_local:?}, grows {growth:?})");
        let bounds_post_fx = grown(bounds_local, growth);
        let origin = [bounds_post_fx[0].floor() as i32, bounds_post_fx[1].floor() as i32];
        let far = [bounds_post_fx[2].ceil() as i32, bounds_post_fx[3].ceil() as i32];
        let size = [(far[0] - origin[0]).max(1), (far[1] - origin[1]).max(1)];
        if size.iter().any(|&s| s > MAX_SIDE) || i64::from(size[0]) * i64::from(size[1]) > MAX_PIXELS {
            bail!("the layer's picture would be {}x{} px: its box is {:.0}x{:.0} and its effects add up to {:.0} px left, {:.0} up, {:.0} right and {:.0} down; the limit is {MAX_SIDE} px a side and {} million pixels. Lower an effect's radius or distance",
                size[0], size[1], bounds_local[2] - bounds_local[0], bounds_local[3] - bounds_local[1], growth[0], growth[1], growth[2], growth[3], MAX_PIXELS / (1024 * 1024));
        }
        Ok(LocalBuffer { bounds_local, bounds_post_fx, bounds_screen: None, origin, size })
    }

    /// Where the picture sits in layer space, as the effects library wants it.
    pub fn space(&self) -> Space {
        Space { origin: [f64::from(self.origin[0]), f64::from(self.origin[1])], content: self.bounds_local }
    }

    /// Bytes of half-float RGBA the picture takes.
    pub fn bytes(&self) -> u64 {
        self.size[0] as u64 * self.size[1] as u64 * 8
    }
}

/// How far the effects in `fx` (evaluated as `states`) reach past `content` at time `t`: each grows the box the effects
/// before it left, and the sum is what the picture is made big enough for.
pub fn chain_growth(fx: &[PlanFx], states: &[FxState], lib: &Library, t: f64, content: [f64; 4]) -> Result<Growth> {
    let mut reach = content;
    for (f, state) in fx.iter().zip(states) {
        let g = match (&f.op, &state.op) {
            (FxOp::Builtin(e), FxOpState::Builtin) => e.grows(t, reach),
            (FxOp::Plugin(p), FxOpState::Plugin(values)) => {
                let def = lib.get(&p.name)?;
                let value = |name: &str| values.values.iter().find(|(n, _)| n == name).and_then(|(_, v)| v.first()).map(|&x| f64::from(x));
                def.bounds.growth(&value).map_err(|e| anyhow!("effect {:?}: {e:#}", p.name))?
            }
            _ => bail!("effect {:?} was evaluated as a different kind of effect than the plan has", f.name()),
        };
        reach = grown(reach, g);
    }
    Ok([content[0] - reach[0], content[1] - reach[1], reach[2] - content[2], reach[3] - content[3]])
}

/// The picture layer `l` is drawn into when it runs its effects in layer space (none when it does not): its content box
/// grown by its effect chain, and where that lands on screen (`project` gives a box's bounding box on screen).
pub fn for_layer(l: &LayerPlan, states: &[FxState], lib: &Library, t: f64, content: Option<[f64; 4]>, project: impl FnOnce([f64; 4]) -> Option<[f64; 4]>) -> Result<Option<LocalBuffer>> {
    if !l.in_layer_space {
        return Ok(None);
    }
    let growth = chain_growth(&l.fx, states, lib, t, content.unwrap_or([0.0; 4])).with_context(|| l.label.clone())?;
    let mut buf = LocalBuffer::plan(content, growth).with_context(|| l.label.clone())?;
    buf.bounds_screen = project(buf.bounds_post_fx);
    Ok(Some(buf))
}

#[cfg(test)]
#[path = "buffer_tests.rs"]
mod layer_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_is_the_content_box_grown_and_rounded_out() {
        let b = LocalBuffer::plan(Some([0.0, 0.0, 4.0, 4.0]), [14.0; 4]).unwrap();
        assert_eq!((b.origin, b.size), ([-14, -14], [32, 32]), "a 4x4 square with 14 px a side");
        assert_eq!(b.bounds_post_fx, [-14.0, -14.0, 18.0, 18.0]);
        let b = LocalBuffer::plan(Some([10.4, 20.6, 50.2, 60.0]), [1.5, 0.0, 0.0, 2.2]).unwrap();
        assert_eq!(b.origin, [8, 20], "rounded outward on the near sides");
        assert_eq!(b.size, [43, 43], "x 8..51 (50.2 up), y 20..63 (62.2 up)");
    }

    #[test]
    fn a_layer_costs_its_own_size_and_not_the_frames() {
        // a 40x40 layer with a 10 px blur (3 sigma = 30) in a 1920x1080 frame
        let b = LocalBuffer::plan(Some([0.0, 0.0, 40.0, 40.0]), [30.0; 4]).unwrap();
        assert_eq!(b.size, [100, 100]);
        assert_eq!(b.bytes(), 100 * 100 * 8);
        assert!(b.bytes() < 1920 * 1080 * 8 / 100, "under a hundredth of a frame-sized picture");
        let plain = LocalBuffer::plan(Some([0.0, 0.0, 40.0, 40.0]), [0.0; 4]).unwrap();
        assert_eq!((plain.origin, plain.size), ([0, 0], [40, 40]), "no effect growth: the layer's own size");
    }

    #[test]
    fn content_outside_the_frame_is_still_in_the_picture() {
        let b = LocalBuffer::plan(Some([-300.0, -200.0, 100.0, 50.0]), [0.0; 4]).unwrap();
        assert_eq!((b.origin, b.size), ([-300, -200], [400, 250]));
    }

    #[test]
    fn nothing_to_draw_is_a_one_pixel_picture() {
        let b = LocalBuffer::plan(None, [0.0; 4]).unwrap();
        assert_eq!(b.size, [1, 1]);
    }

    #[test]
    fn an_absurd_reach_is_an_error_that_says_what_to_lower() {
        let e = LocalBuffer::plan(Some([0.0, 0.0, 100.0, 100.0]), [20000.0; 4]).unwrap_err().to_string();
        assert!(e.contains("40100x40100") && e.contains("Lower an effect's radius"), "{e}");
        let e = LocalBuffer::plan(Some([0.0, 0.0, f64::NAN, 1.0]), [0.0; 4]).unwrap_err().to_string();
        assert!(e.contains("finite"), "{e}");
    }

    #[test]
    fn the_space_has_the_origin_and_the_content_box() {
        let s = LocalBuffer::plan(Some([5.0, 6.0, 25.0, 16.0]), [3.0; 4]).unwrap().space();
        assert_eq!((s.origin, s.content), ([2.0, 3.0], [5.0, 6.0, 25.0, 16.0]));
    }
}
