//! Text animators: from a layout, a clock and the animators to a state for every character. Each animator groups
//! the characters into units (characters, words or lines), gives each unit a weight 0..1 and moves the unit by its
//! `props` scaled by that weight. Two ways to get the weight, because editors think in both:
//!
//! * GSAP mode (`stagger`, `at`, `dur`, `ease`): the weight of unit *i* runs 1 → 0 (in) or 0 → 1 (out) on a clock
//!   that starts `i` staggers late.
//! * After Effects mode (`selector`): the weight is how much of the unit lies inside a range that can slide along
//!   the text, with a shape.

use anyhow::{anyhow, Result};
use serde::Serialize;

use super::layout::{Layout, Span};
use crate::{
    anim::{ease::Ease, noise},
    schema::{Animator, Direction, Selector, StaggerFrom, Unit},
};

/// A 2D affine map: `x' = a x + c y + e`, `y' = b x + d y + f`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Aff(pub [f32; 6]);

impl Aff {
    pub const IDENTITY: Aff = Aff([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    pub fn translate(x: f32, y: f32) -> Aff {
        Aff([1.0, 0.0, 0.0, 1.0, x, y])
    }

    pub fn scale(x: f32, y: f32) -> Aff {
        Aff([x, 0.0, 0.0, y, 0.0, 0.0])
    }

    pub fn rotate(deg: f32) -> Aff {
        let (s, c) = deg.to_radians().sin_cos();
        Aff([c, s, -s, c, 0.0, 0.0])
    }

    pub fn skew_x(deg: f32) -> Aff {
        Aff([1.0, 0.0, deg.to_radians().tan(), 1.0, 0.0, 0.0])
    }

    /// `self` applied after `o` (matrix product `self · o`).
    pub fn mul(&self, o: &Aff) -> Aff {
        let (a, b) = (self.0, o.0);
        Aff([
            a[0] * b[0] + a[2] * b[1],
            a[1] * b[0] + a[3] * b[1],
            a[0] * b[2] + a[2] * b[3],
            a[1] * b[2] + a[3] * b[3],
            a[0] * b[4] + a[2] * b[5] + a[4],
            a[1] * b[4] + a[3] * b[5] + a[5],
        ])
    }

    /// Do `m` about the point `(cx, cy)`.
    pub fn about(cx: f32, cy: f32, m: &Aff) -> Aff {
        Aff::translate(cx, cy).mul(m).mul(&Aff::translate(-cx, -cy))
    }

    pub fn is_identity(&self) -> bool {
        self.0.iter().zip(Aff::IDENTITY.0).all(|(a, b)| (a - b).abs() < 1e-6)
    }
}

/// How one character is drawn at one instant.
#[derive(Debug, Clone, Serialize)]
pub struct GlyphState {
    pub ch: char,
    /// The rest position in layout space (left, baseline).
    pub x: f32,
    pub y: f32,
    /// Applied to the glyph in layout space (about nothing: it already includes each unit's pivot).
    pub m: Aff,
    pub opacity: f64,
    pub blur: f64,
    /// `[r, g, b]` 0..255 when an animator has recoloured the glyph.
    pub color: Option<[f64; 3]>,
    /// Clip `[l, t, r, b]` in layout space (the unit's own box) for masked reveals.
    pub clip: Option<[f32; 4]>,
}

/// An animator with the animated parts of its selector already evaluated.
pub struct Resolved<'a> {
    pub anim: &'a Animator,
    pub start: f64,
    pub end: f64,
    pub offset: f64,
    /// The colour the unit mixes toward at full weight, when the animator recolours.
    pub color: Option<[f32; 4]>,
}

fn units<'a>(layout: &'a Layout, by: Unit) -> Vec<(std::ops::Range<usize>, [f32; 4])> {
    match by {
        Unit::Chars => layout.glyphs.iter().enumerate().map(|(i, g)| (i..i + 1, [g.x, g.top, g.x + g.w, g.bottom])).collect(),
        Unit::Words => layout.words.iter().map(|s: &Span| (s.glyphs.clone(), s.rect)).collect(),
        Unit::Lines => layout.lines.iter().map(|s: &Span| (s.glyphs.clone(), s.rect)).collect(),
    }
}

/// Delay order of unit `i` of `n` for a stagger origin: 0 starts first.
fn stagger_order(from: StaggerFrom, i: usize, n: usize, seed: u32) -> f64 {
    let last = (n.max(1) - 1) as f64;
    match from {
        StaggerFrom::Start => i as f64,
        StaggerFrom::End => last - i as f64,
        StaggerFrom::Center => (i as f64 - last / 2.0).abs(),
        StaggerFrom::Edges => (i as f64).min(last - i as f64),
        StaggerFrom::Random => noise::rand01(seed, i as i32, 91) * last,
    }
}

fn selector_weight(sel: &Selector, start: f64, end: f64, offset: f64, i: usize, n: usize) -> f64 {
    // position of the unit's centre along the text, 0..1; random reorders the units
    let slot = if sel.random { (noise::rand01(sel.seed, i as i32, 17) * n as f64).floor().min(n as f64 - 1.0) } else { i as f64 };
    let p = (slot + 0.5) / n as f64;
    let (s, e) = (start + offset, end + offset);
    let span = (e - s).max(1e-9);
    let u = (p - s) / span;
    let soft = sel.smooth.clamp(0.0, 1.0);
    let edge = |x: f64| if soft <= 0.0 { f64::from(x >= 0.0) } else { let w = soft * 0.5; ((x + w) / (2.0 * w)).clamp(0.0, 1.0).powi(2) * (3.0 - 2.0 * ((x + w) / (2.0 * w)).clamp(0.0, 1.0)) };
    match sel.shape.as_str() {
        "ramp_up" => if (0.0..=1.0).contains(&u) { u } else { f64::from(u > 1.0) },
        "ramp_down" => if (0.0..=1.0).contains(&u) { 1.0 - u } else { f64::from(u < 0.0) },
        "triangle" => if (0.0..=1.0).contains(&u) { 1.0 - (2.0 * u - 1.0).abs() } else { 0.0 },
        "round" => if (0.0..=1.0).contains(&u) { (1.0 - (2.0 * u - 1.0).powi(2)).max(0.0).sqrt() } else { 0.0 },
        "smooth" => if (0.0..=1.0).contains(&u) { let x = 1.0 - (2.0 * u - 1.0).abs(); x * x * (3.0 - 2.0 * x) } else { 0.0 },
        _ => edge(p - s) * edge(e - p),
    }
}

/// The state of every character of `layout` at layer time `t`.
pub fn glyph_states(layout: &Layout, animators: &[Resolved], t: f64, base_color: [f32; 4], font_size: f64) -> Result<Vec<GlyphState>> {
    let mut out: Vec<GlyphState> = layout
        .glyphs
        .iter()
        .map(|g| GlyphState { ch: g.ch, x: g.x, y: g.y, m: Aff::IDENTITY, opacity: 1.0, blur: 0.0, color: None, clip: None })
        .collect();
    let _ = font_size;
    // cumulative tracking shift per glyph (added after the per-unit maths so later glyphs make room)
    let mut shift = vec![0.0f32; layout.glyphs.len()];
    for r in animators {
        let a = r.anim;
        let list = units(layout, a.by);
        let n = list.len();
        if n == 0 {
            continue;
        }
        let ease = match &a.ease {
            Some(e) => Ease::parse(e)?,
            None => Ease::parse("power2.out")?,
        };
        let (each, from, seed, sease) = match &a.stagger {
            Some(s) => (s.each, s.from, s.seed, s.ease.as_deref().map(Ease::parse).transpose()?),
            None => (0.0, StaggerFrom::Start, 1, None),
        };
        let max_order = (0..n).map(|i| stagger_order(from, i, n, seed)).fold(0.0, f64::max).max(1e-9);
        for (i, (ids, rect)) in list.iter().enumerate() {
            let w = match &a.selector {
                Some(sel) => selector_weight(sel, r.start, r.end, r.offset, i, n),
                None => {
                    let order = stagger_order(from, i, n, seed);
                    let order = match &sease {
                        Some(e) => e.at(order / max_order) * max_order,
                        None => order,
                    };
                    let delay = order * each;
                    let u = if a.dur <= 1e-9 { f64::from(t - a.at - delay >= 0.0) } else { ((t - a.at - delay) / a.dur).clamp(0.0, 1.0) };
                    match a.direction {
                        Direction::In => 1.0 - ease.at(u),
                        Direction::Out => ease.at(u),
                    }
                }
            };
            if w.abs() < 1e-9 && a.scramble.is_none() {
                continue;
            }
            let p = &a.props;
            let (cx, cy) = ((rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0);
            let sc = (1.0 + p.scale.unwrap_or(0.0) * w).max(0.0) as f32;
            let pivot_m = Aff::translate((p.x.unwrap_or(0.0) * w) as f32, (p.y.unwrap_or(0.0) * w) as f32)
                .mul(&Aff::about(cx, cy, &Aff::rotate((p.rotation.unwrap_or(0.0) * w) as f32).mul(&Aff::skew_x((p.skew.unwrap_or(0.0) * w) as f32)).mul(&Aff::scale(sc, sc))));
            for gi in ids.clone() {
                let g = &mut out[gi];
                g.m = pivot_m.mul(&g.m);
                g.opacity *= (1.0 + p.opacity.unwrap_or(0.0) * w).clamp(0.0, 1.0);
                g.blur = g.blur.max(p.blur.unwrap_or(0.0) * w);
                if let Some(target) = r.color {
                    let from = g.color.map_or(base_color, |c| [c[0] as f32, c[1] as f32, c[2] as f32, 1.0]);
                    g.color = Some(mix_color(from, target, w));
                }
                if a.mask {
                    g.clip = Some([rect[0] - 2.0, rect[1], rect[2] + 2.0, rect[3]]);
                }
                if let Some(set) = &a.scramble {
                    if w > 1e-6 && !set.is_empty() {
                        let chars: Vec<char> = set.chars().collect();
                        let k = noise::hash(seed, gi as i32, (t * 30.0).floor() as i32) as usize % chars.len();
                        g.ch = chars[k];
                    }
                }
                if let Some(tr) = p.tracking {
                    for s in shift.iter_mut().skip(gi) {
                        *s += (tr * w) as f32;
                    }
                }
            }
        }
    }
    for (g, s) in out.iter_mut().zip(shift) {
        if s != 0.0 {
            g.m = Aff::translate(s, 0.0).mul(&g.m);
        }
    }
    Ok(out)
}

/// Build the colour-mix of animators that recolour: `[(weight, target rgb)]` is applied by the caller; kept separate
/// because colour targets are animated properties evaluated with the layer's environment.
pub fn mix_color(base: [f32; 4], target: [f32; 4], w: f64) -> [f64; 3] {
    let k = w.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| f64::from(base[i]) + (f64::from(target[i]) - f64::from(base[i])) * k)
}

pub fn parse_ease(name: Option<&String>) -> Result<Ease> {
    name.map_or_else(|| Ok(Ease::default()), |e| Ease::parse(e)).map_err(|e| anyhow!("{e}"))
}
