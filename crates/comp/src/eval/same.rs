//! Whether a layer is drawn the same at two instants, so a renderer can draw it once instead of once per instant.
//!
//! A [`LayerState`] holds everything the painters read except the clock: `local_t` is the time it was evaluated at, and a
//! few things read it while drawing (effects, a texture, a pack, the dissolve blend). A layer that none of those touch is
//! *clockless*, and two instants of it are the same picture exactly when the rest of the state is equal.

use super::{Content, Frame, LayerState};

impl LayerState {
    /// No painter reads the time this state was evaluated at: no effect (a platform effect's numbers are read at draw time,
    /// not held here), no transition blur or smear, no texture or pack, no blend that is seeded by the frame number, and
    /// nothing inside a nested composition that does.
    pub fn is_clockless(&self) -> bool {
        self.fx.is_empty()
            && self.extra_blur == 0.0
            && self.extra_smear == 0.0
            && !self.blend_mode.is_some_and(|b| b.needs_frame())
            && match &self.content {
                Content::Texture { .. } | Content::Pack { .. } => false,
                Content::Precomp { frame: Some(inner) } => inner.look.is_empty() && inner.layers.iter().all(LayerState::is_clockless),
                _ => true,
            }
    }

    /// Everything but the time the two states were evaluated at is equal. Every field is named here, so a field added to
    /// the state cannot be left out of the comparison: it will not compile until it is compared or set aside.
    pub fn equals_apart_from_clock(&self, other: &LayerState) -> bool {
        let LayerState { index, id, label, kind, active, visible, local_t: _, opacity, origin, bounds, blend, depth, blend_mode, screen, world, in_3d, extra_blur, extra_smear, clip, masks, fx, local, lit, reflect, content } = self;
        *index == other.index
            && *id == other.id
            && *label == other.label
            && *kind == other.kind
            && *active == other.active
            && *visible == other.visible
            && *opacity == other.opacity
            && *origin == other.origin
            && *bounds == other.bounds
            && *blend == other.blend
            && *depth == other.depth
            && *blend_mode == other.blend_mode
            && *screen == other.screen
            && *world == other.world
            && *in_3d == other.in_3d
            && *extra_blur == other.extra_blur
            && *extra_smear == other.extra_smear
            && *clip == other.clip
            && *masks == other.masks
            && *fx == other.fx
            && *local == other.local
            && *lit == other.lit
            && *reflect == other.reflect
            && content_equals(content, &other.content)
    }
}

impl Frame {
    /// The frames are the same scene, apart from the time they were evaluated at.
    pub fn equals_apart_from_clock(&self, other: &Frame) -> bool {
        let Frame { comp, t: _, width, height, bg, camera, lights, look, layers } = self;
        *comp == other.comp
            && *width == other.width
            && *height == other.height
            && *bg == other.bg
            && *camera == other.camera
            && *lights == other.lights
            && *look == other.look
            && layers.len() == other.layers.len()
            && layers.iter().zip(&other.layers).all(|(a, b)| a.equals_apart_from_clock(b))
    }
}

/// Content is compared as it is, except that a nested composition's frame is compared apart from its clock.
fn content_equals(a: &Content, b: &Content) -> bool {
    match (a, b) {
        (Content::Precomp { frame: x }, Content::Precomp { frame: y }) => match (x, y) {
            (Some(x), Some(y)) => x.equals_apart_from_clock(y),
            (None, None) => true,
            _ => false,
        },
        _ => a == b,
    }
}
