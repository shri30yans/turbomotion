//! How far the blurs spread light past the pixels they are given (see `bounds.rs` for the rest of the platform
//! effects). An exact blur reaches as far as the kernel reads, which the kernel states ([`crate::kernel::Blur::reach`]);
//! a legacy blur reaches three sigma, where its weight runs out. A blur that repeats the picture's edge adds nothing
//! beyond it, so it reaches nowhere.

use super::{
    bounds::{disc_reach, Growth},
    quality::{spread, Quality},
    smear::Radial,
    Effect,
};
use crate::kernel::{Blur, Edge, Shape};

fn all_sides(r: [f64; 2]) -> Growth {
    [r[0], r[1], r[0], r[1]]
}

/// What the blur family reaches past `content` at time `t`; `None` for an effect that is not in it.
pub(super) fn grows(effect: &Effect, t: f64, content: [f64; 4]) -> Option<Growth> {
    let (w, h) = (content[2] - content[0], content[3] - content[1]);
    let kernel = |shape: Shape, edge: Edge| match edge {
        Edge::Repeat => [0.0; 4],
        Edge::Transparent => all_sides(Blur { shape, edge }.reach()),
    };
    Some(match effect {
        Effect::Blur(e) => {
            let sigma = e.dimensions.along(e.radius.at(t));
            match (e.quality, e.edge) {
                (_, Edge::Repeat) => [0.0; 4],
                (Quality::Legacy, _) => all_sides(sigma.map(|s| spread(s, Quality::Legacy))),
                (Quality::Exact, _) => kernel(Shape::Gaussian { sigma }, e.edge),
            }
        }
        Effect::Glow(e) => [spread(e.radius.at(t), e.quality); 4],
        Effect::FilmHalation(e) => [spread(e.radius.at(t), e.quality); 4],
        Effect::BoxBlur(e) => kernel(Shape::Box { radius: e.dimensions.along(e.radius.at(t)), iterations: super::lens::iterations(&e.iterations, t) }, e.edge),
        Effect::LensBlur(e) => kernel(Shape::Disc(e.iris(t)), e.edge),
        Effect::Shadow(e) => {
            let (dx, dy, reach) = (e.dx.at(t), e.dy.at(t), spread(e.blur.at(t), e.quality));
            [(reach - dx).max(0.0), (reach - dy).max(0.0), (reach + dx).max(0.0), (reach + dy).max(0.0)]
        }
        Effect::DirectionalBlur(e) => {
            let d = e.distance.at(t);
            let (sin, cos) = e.angle.at(t).to_radians().sin_cos();
            let (gx, gy) = ((d * cos).abs() / 2.0, (d * sin).abs() / 2.0);
            match e.quality {
                Quality::Legacy if d.abs() < super::smear::MIN_SMEAR => [0.0; 4],
                Quality::Legacy => [gx, gy, gx, gy],
                // the line is centred on the pixel, and the bilinear read adds a pixel past its ends
                Quality::Exact if d == 0.0 => [0.0; 4],
                Quality::Exact => [gx + 1.0, gy + 1.0, gx + 1.0, gy + 1.0],
            }
        }
        Effect::RadialBlur(e) => {
            let amount = e.amount.at(t);
            let centre = (content[0] + e.x.at(t) * w, content[1] + e.y.at(t) * h);
            match (e.quality, e.mode) {
                (Quality::Exact, Radial::Spin) if amount != 0.0 => disc_reach(content, centre, f64::INFINITY).map(|g| if g > 0.0 { g + 1.0 } else { 0.0 }),
                (_, Radial::Spin) => [0.0; 4],
                (quality, Radial::Zoom) => {
                    let a = amount.max(0.0);
                    // each side moves outward by `amount` of its distance from the centre; an exact smear reads a pixel past that
                    let past = |distance: f64| match quality {
                        Quality::Legacy => distance.max(0.0) * a,
                        Quality::Exact if distance > 0.0 && a > 0.0 => distance * a + 1.0,
                        Quality::Exact => 0.0,
                    };
                    [past(centre.0 - content[0]), past(centre.1 - content[1]), past(content[2] - centre.0), past(content[3] - centre.1)]
                }
            }
        }
        _ => return None,
    })
}
