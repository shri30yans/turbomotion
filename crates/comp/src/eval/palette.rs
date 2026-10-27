//! A palette cycle at one instant: the colours the copies of a repeater take.
//!
//! [`resolve`] reads the palette and the cycle's offset at the time (they are properties, so they may animate), and
//! [`Cycled::colour`] is then the colour of one copy. Both repeaters use it: the shape repeater for its copies
//! (`eval::shape::repeat`) and the layer repeater, through the `swatch` each of its copies carries ([`apply`]).
//!
//! **Gradient mode interpolates in the film's working colour space** (`Profile::mix`), the space the compositor blends
//! in. In a display-referred film that is the sRGB-encoded values, which is what an editor's gradient does; in a
//! `linear_light` film it is light, so a blend of two colours is the mix of their light and matches a Skia gradient
//! drawn in the same film.

use anyhow::{ensure, Result};
use serde::Serialize;
use turbomotion_core::color::Profile;

use super::{env::LayerEnv, salt, Content, Evaluator};
use crate::{
    anim::{
        cycle::{slot, CycleMode, NonEmpty, Slot},
        prop::Rgba,
    },
    plan::{CompPlan, LayerPlan},
    schema::{
        palette::{Cycling, Target},
        Kind,
    },
};

/// The colours of a palette and where its cycle stands.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Cycled {
    pub colours: NonEmpty<Rgba>,
    pub mode: CycleMode,
    /// Colours the palette is shifted on, read at this time.
    pub offset: f64,
    #[serde(skip)]
    working: Profile,
}

/// Read the palette and the offset of `cycling` at layer time `lt`. `env` makes the environment of a property with the
/// given salt; the colours and the offset are salted under `owner` so each is its own property.
pub fn resolve<'a>(cycling: &Cycling, lt: f64, env: &dyn Fn(u32) -> LayerEnv<'a>, owner: u32, working: Profile) -> Result<Cycled> {
    let colours = cycling.palette.try_map(|i, c| c.0.at(lt, &env(salt::named(owner, &format!("colour.{i}")))))?;
    let colours = if cycling.cycle.reverse { colours.reversed() } else { colours };
    let offset = cycling.cycle.offset.as_ref().map_or(Ok(0.0), |p| p.0.at(lt, &env(salt::named(owner, "offset"))))?;
    ensure!(offset.is_finite(), "the palette cycle's offset is {offset} at this time: it needs to be a finite number of colours");
    Ok(Cycled { colours, mode: cycling.cycle.mode, offset, working })
}

impl Cycled {
    /// The colour copy `copy` takes. A step is one of the palette's own colours, exactly; a blend is mixed in the
    /// working space, its alpha blended straight.
    pub fn colour(&self, copy: usize) -> Rgba {
        match slot(self.mode, self.colours.len(), copy, self.offset) {
            Slot::Pick(i) => *self.colours.get(i),
            Slot::Blend { from, to, by } => {
                let (a, b) = (self.colours.get(from), self.colours.get(to));
                let level = |c: &Rgba| [c[0] / 255.0, c[1] / 255.0, c[2] / 255.0];
                let [r, g, bl] = self.working.mix(level(a), level(b), by as f32);
                // Skia's curve and its inverse agree to a few parts in ten thousand, so the end of a blend can land just outside the gamut
                let level = |v: f32| (v * 255.0).clamp(0.0, 255.0);
                [level(r), level(g), level(bl), a[3] + (b[3] - a[3]) * by as f32]
            }
        }
    }
}

/// A layer's own colour replaced by the colour its `swatch` picks. Content that has nothing to colour is left alone
/// (the plan has already refused a swatch on a layer that never has any).
pub fn apply(ev: &Evaluator, c: &CompPlan, l: &LayerPlan, t: f64, content: Content) -> Result<Content> {
    let Some(swatch) = &l.def.common.swatch else { return Ok(content) };
    let env = |salt: u32| LayerEnv::new(ev, c, l.index, t, 0).with_salt(salt);
    let colour = resolve(&swatch.cycling, t - l.start, &env, salt::SWATCH, c.color.working)?.colour(swatch.index);
    let paint: Target = swatch.cycling.paint;
    Ok(match content {
        Content::Solid { size, .. } => Content::Solid { color: colour, size },
        Content::Text(mut st) => {
            // outline-only text with no `stroke` of its own draws its outline in `color`
            let outline_follows_color = matches!(&l.def.kind, Kind::Text(tx) if tx.stroke.is_none());
            if paint.fill() {
                st.color = colour;
            }
            if paint.stroke() || (paint.fill() && outline_follows_color) {
                st.stroke = st.stroke.map(|(_, width)| (colour, width));
            }
            Content::Text(st)
        }
        Content::Shape { mut items } => {
            for item in &mut items {
                if let Some(f) = item.fill.as_mut().filter(|f| paint.fill() && f.color.is_some() && f.gradient.is_none()) {
                    f.color = Some(colour);
                }
                if let Some(s) = item.stroke.as_mut().filter(|_| paint.stroke()) {
                    s.color = colour;
                }
            }
            Content::Shape { items }
        }
        other => other,
    })
}

#[cfg(test)]
mod tests;
