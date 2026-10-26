//! Text animators: from a layout, a clock and the animators to a state for every character. Each animator groups
//! the characters into units (characters, words or lines), gives each unit a weight 0..1 and moves the unit by its
//! `props` scaled by that weight. Two ways to get the weight, because editors think in both:
//!
//! * GSAP mode (`stagger`, `at`, `dur`, `ease`): the weight of unit *i* runs 1 → 0 (in) or 0 → 1 (out) on a clock
//!   that starts `i` staggers late.
//! * After Effects mode (`selector`): the weight is how much of the unit lies inside a range that can slide along
//!   the text, with a shape.

use anyhow::{anyhow, ensure, Result};
use serde::Serialize;

use super::layout::{Layout, Span};
use super::place::{self, Placement};
use crate::{
    anim::{ease::Ease, noise},
    eval::{mat4::Mat4, pose::euler_matrix},
    schema::{Animator, Charset, Direction, Selector, SelectorShape, StaggerFrom, Unit, Wiggle},
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

    /// The x/y part of a 4x4 (what it does to a flat glyph seen from the front, ignoring perspective).
    pub fn from_mat4(m: &Mat4) -> Aff {
        let v = m.0;
        Aff([v[0], v[4], v[1], v[5], v[3], v[7]].map(|c| c as f32))
    }

    /// This map as a 4x4 (z untouched).
    pub fn to_mat4(&self) -> Mat4 {
        let [a, b, c, d, e, f] = self.0.map(f64::from);
        Mat4([a, c, 0.0, e, b, d, 0.0, f, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0])
    }

    pub fn is_identity(&self) -> bool {
        self.0.iter().zip(Aff::IDENTITY.0).all(|(a, b)| (a - b).abs() < 1e-6)
    }
}

/// How one character is drawn at one instant.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GlyphState {
    /// Which glyph of the layout this draws (a marquee draws each one several times).
    pub src: usize,
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
    /// The whole transform as a 4x4 in layout space, when the glyph has depth (per-character 3D, a surface): it
    /// replaces `m`, and already holds the text's perspective.
    pub m4: Option<[f64; 16]>,
    /// Hue shift in degrees, saturation and brightness offsets on the fill.
    pub hue: f64,
    pub sat: f64,
    pub bri: f64,
    /// Pixels added to the stroke width.
    pub stroke_w: f64,
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

fn units(layout: &Layout, by: Unit) -> Vec<(std::ops::Range<usize>, [f32; 4])> {
    match by {
        Unit::Chars => layout.glyphs.iter().enumerate().map(|(i, g)| (i..i + 1, [g.x, g.top, g.x + g.w, g.bottom])).collect(),
        Unit::Words => layout.words.iter().map(|s: &Span| (s.glyphs.clone(), s.rect)).collect(),
        Unit::Lines => layout.lines.iter().map(|s: &Span| (s.glyphs.clone(), s.rect)).collect(),
    }
}

/// Delay order of unit `i` of `n` for a stagger origin: 0 starts first.
pub(crate) fn stagger_order(from: StaggerFrom, i: usize, n: usize, seed: u32) -> f64 {
    let last = (n.max(1) - 1) as f64;
    match from {
        StaggerFrom::Start => i as f64,
        StaggerFrom::End => last - i as f64,
        StaggerFrom::Center => (i as f64 - last / 2.0).abs(),
        StaggerFrom::Edges => (i as f64).min(last - i as f64),
        StaggerFrom::Random => noise::rand01(seed, i as i32, 91) * last,
    }
}

fn selector_weight(sel: &Selector, start: f64, end: f64, offset: f64, i: usize, n: usize, t: f64) -> f64 {
    wiggle_factor(sel.wiggle.as_ref(), i, t) * range_weight(sel, start, end, offset, i, n)
}

/// The unit's own random factor in `min..=max`: fixed for `rate` 0, a smooth drift through random targets otherwise.
fn wiggle_factor(wiggle: Option<&Wiggle>, i: usize, t: f64) -> f64 {
    let Some(w) = wiggle else { return 1.0 };
    let unit = if w.rate <= 0.0 {
        noise::rand01(w.seed, i as i32, WIGGLE_SALT)
    } else {
        (noise::noise1(noise::hash(w.seed, i as i32, WIGGLE_SALT), t * w.rate) + 1.0) / 2.0
    };
    w.min + (w.max - w.min) * unit
}

fn range_weight(sel: &Selector, start: f64, end: f64, offset: f64, i: usize, n: usize) -> f64 {
    // position of the unit's centre along the text, 0..1; random reorders the units
    let slot = if sel.random { (noise::rand01(sel.seed, i as i32, 17) * n as f64).floor().min(n as f64 - 1.0) } else { i as f64 };
    let p = (slot + 0.5) / n as f64;
    let (s, e) = (start + offset, end + offset);
    let span = (e - s).max(1e-9);
    let u = (p - s) / span;
    let soft = sel.smooth.clamp(0.0, 1.0);
    let edge = |x: f64| if soft <= 0.0 { f64::from(x >= 0.0) } else { let w = soft * 0.5; ((x + w) / (2.0 * w)).clamp(0.0, 1.0).powi(2) * (3.0 - 2.0 * ((x + w) / (2.0 * w)).clamp(0.0, 1.0)) };
    let inside = (0.0..=1.0).contains(&u);
    match sel.shape {
        SelectorShape::RampUp => if inside { u } else { f64::from(u > 1.0) },
        SelectorShape::RampDown => if inside { 1.0 - u } else { f64::from(u < 0.0) },
        SelectorShape::Triangle => if inside { 1.0 - (2.0 * u - 1.0).abs() } else { 0.0 },
        SelectorShape::Round => if inside { (1.0 - (2.0 * u - 1.0).powi(2)).max(0.0).sqrt() } else { 0.0 },
        SelectorShape::Smooth => if inside { let x = 1.0 - (2.0 * u - 1.0).abs(); x * x * (3.0 - 2.0 * x) } else { 0.0 },
        SelectorShape::Square => edge(p - s) * edge(e - p),
    }
}

/// Keeps a unit's wiggle numbers apart from the other draws made with the same seed.
const WIGGLE_SALT: i32 = 53;

/// A weight this small counts as none.
const NO_WEIGHT: f64 = 1e-9;

/// A fill whose chroma or value is this small has no hue to shift.
const ACHROMATIC: f64 = 1e-9;
/// The hue circle has this many sextants, one per step between a primary and a secondary colour.
const SEXTANTS: f64 = 6.0;
const SEXTANT_DEGREES: f64 = 60.0;
const HUE_DEGREES: f64 = SEXTANTS * SEXTANT_DEGREES;
/// Colours here are 0..255 per channel.
const CHANNEL_MAX: f64 = 255.0;
/// Scramble noise changes this many times a second.
const SCRAMBLE_RATE: f64 = 30.0;

const SYMBOLS: &str = "#%&*+=?@$!/<>~^|";
const HEX_DIGITS: &str = "0123456789ABCDEF";
/// Half-width katakana, the glyphs of the falling-code look.
const KATAKANA: std::ops::RangeInclusive<char> = '\u{ff66}'..='\u{ff9d}';

/// The characters a cipher draws from for a real character `real`, that `font` can draw.
fn cipher_set(charset: &Charset, real: char, font: &skia_safe::Font) -> Result<Vec<char>> {
    let mut set: Vec<char> = match charset {
        Charset::Auto if real.is_ascii_uppercase() => ('A'..='Z').collect(),
        Charset::Auto if real.is_ascii_lowercase() => ('a'..='z').collect(),
        Charset::Auto if real.is_ascii_digit() => ('0'..='9').collect(),
        Charset::Auto | Charset::Symbols => SYMBOLS.chars().collect(),
        Charset::Upper => ('A'..='Z').collect(),
        Charset::Lower => ('a'..='z').collect(),
        Charset::Digits => ('0'..='9').collect(),
        Charset::Alnum => ('A'..='Z').chain('a'..='z').chain('0'..='9').collect(),
        Charset::Hex => HEX_DIGITS.chars().collect(),
        Charset::Binary => vec!['0', '1'],
        Charset::Matrix => KATAKANA.chain('0'..='9').collect(),
        Charset::Literal(chars) => chars.clone(),
    };
    set.retain(|ch| font.unichar_to_glyph(*ch as i32) != 0);
    ensure!(!set.is_empty(), "the cipher charset {charset:?} has no character the font can draw");
    Ok(set)
}

/// The state of every character of `layout` at layer time `t`.
pub fn glyph_states(layout: &Layout, animators: &[Resolved], t: f64, base_color: [f32; 4], inks: &[Option<[f64; 3]>], placement: &Placement) -> Result<Vec<GlyphState>> {
    let mut out: Vec<GlyphState> = layout
        .glyphs
        .iter()
        .enumerate()
        .map(|(i, g)| GlyphState { src: i, ch: g.ch, x: g.x, y: g.y, m: Aff::IDENTITY, opacity: 1.0, blur: 0.0, color: inks.get(i).copied().flatten(), clip: None, m4: None, hue: 0.0, sat: 0.0, bri: 0.0, stroke_w: 0.0 })
        .collect();
    // glyphs an animator gave depth: their running 4x4 (a glyph without one stays on the cheaper 2D map)
    let mut m4s: Vec<Option<Mat4>> = vec![None; out.len()];
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
        // a typewriter that counts spaces staggers by position in the text, not by position among the visible characters
        let slots: Vec<usize> = match (a.count_spaces, a.by) {
            (true, Unit::Chars) => list.iter().map(|(ids, _)| layout.glyphs[ids.start].slot).collect(),
            _ => (0..n).collect(),
        };
        let total = slots.last().map_or(n, |s| s + 1);
        let max_order = (0..n).map(|i| stagger_order(from, slots[i], total, seed)).fold(0.0, f64::max).max(1e-9);
        for (i, (ids, rect)) in list.iter().enumerate() {
            let w = match &a.selector {
                Some(sel) => selector_weight(sel, r.start, r.end, r.offset, i, n, t),
                None => {
                    let order = stagger_order(from, slots[i], total, seed);
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
            let cipher_live = a.cipher.as_ref().is_some_and(|c| w > c.settle + NO_WEIGHT);
            if w.abs() < NO_WEIGHT && a.scramble.is_none() && !cipher_live {
                continue;
            }
            let p = &a.props;
            let (cx, cy) = ((rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0);
            let sc = (1.0 + p.scale.unwrap_or(0.0) * w).max(0.0);
            let rz = (p.rotation.unwrap_or(0.0) + p.rotation_z.unwrap_or(0.0)) * w;
            let (rx, ry) = (p.rotation_x.unwrap_or(0.0) * w, p.rotation_y.unwrap_or(0.0) * w);
            let (dz, sz) = (p.z.unwrap_or(0.0) * w, (1.0 + p.scale_z.unwrap_or(0.0) * w).max(0.0));
            let is3d = rx != 0.0 || ry != 0.0 || dz != 0.0 || sz != 1.0;
            // translate x, y, z; then about the unit's centre turn X, Y, Z in the fixed order, stretch z, skew and scale
            let skew = Mat4::skew_x(p.skew.unwrap_or(0.0) * w);
            let about = Mat4::translate(f64::from(cx), f64::from(cy), 0.0)
                .mul(&Mat4::scale(1.0, 1.0, sz))
                .mul(&euler_matrix([rx, ry, rz]))
                .mul(&skew)
                .mul(&Mat4::scale(sc, sc, sc))
                .mul(&Mat4::translate(-f64::from(cx), -f64::from(cy), 0.0));
            let pivot = Mat4::translate(p.x.unwrap_or(0.0) * w, p.y.unwrap_or(0.0) * w, dz).mul(&about);
            let hue_shift = p.fill_hue.unwrap_or(0.0) * w;
            let sat_shift = p.fill_saturation.unwrap_or(0.0) * w;
            let bri_shift = p.fill_brightness.unwrap_or(0.0) * w;
            let stroke_add = p.stroke_width.unwrap_or(0.0) * w;
            for gi in ids.clone() {
                let g = &mut out[gi];
                if is3d || m4s[gi].is_some() {
                    m4s[gi] = Some(pivot.mul(&m4s[gi].unwrap_or_else(|| g.m.to_mat4())));
                } else {
                    g.m = Aff::from_mat4(&pivot).mul(&g.m);
                }
                g.opacity *= (1.0 + p.opacity.unwrap_or(0.0) * w).clamp(0.0, 1.0);
                g.blur = g.blur.max(p.blur.unwrap_or(0.0) * w);
                g.hue += hue_shift;
                g.sat += sat_shift;
                g.bri += bri_shift;
                g.stroke_w += stroke_add;
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
                        let k = noise::hash(seed, gi as i32, (t * SCRAMBLE_RATE).floor() as i32) as usize % chars.len();
                        g.ch = chars[k];
                    }
                }
                if let (Some(c), true) = (&a.cipher, cipher_live) {
                    let lg = &layout.glyphs[gi];
                    let set = cipher_set(&c.charset, lg.ch, &lg.font)?;
                    let tick = (t * c.rate).floor() as i32;
                    g.ch = set[noise::hash(c.seed, gi as i32, tick) as usize % set.len()];
                    // sit the noise character in the middle of the slot the real one holds
                    let dx = (lg.w - lg.font.measure_str(g.ch.to_string(), None).0) / 2.0;
                    g.m = Aff::translate(dx, 0.0).mul(&g.m);
                    if let Some(m) = &mut m4s[gi] {
                        *m = Mat4::translate(f64::from(dx), 0.0, 0.0).mul(m);
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
    for (g, m) in out.iter_mut().zip(&m4s) {
        g.m4 = m.map(|m| m.0);
    }
    Ok(place::finish(layout, out, &shift, placement))
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

/// Shift an `[r, g, b]` colour (0..255) by `hue` degrees and add `sat` and `bri` (each -1..1) in HSV.
pub fn shift_hsv(rgb: [f32; 3], hue: f64, sat: f64, bri: f64) -> [f32; 3] {
    let [r, g, b] = rgb.map(|c| f64::from(c) / CHANNEL_MAX);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let d = mx - mn;
    let h = if d < ACHROMATIC {
        0.0
    } else if mx == r {
        SEXTANT_DEGREES * ((g - b) / d).rem_euclid(SEXTANTS)
    } else if mx == g {
        SEXTANT_DEGREES * ((b - r) / d + 2.0)
    } else {
        SEXTANT_DEGREES * ((r - g) / d + 4.0)
    };
    let s = if mx <= ACHROMATIC { 0.0 } else { d / mx };
    let (h, s, v) = ((h + hue).rem_euclid(HUE_DEGREES), (s + sat).clamp(0.0, 1.0), (mx + bri).clamp(0.0, 1.0));
    let (c, hp) = (v * s, (h / SEXTANT_DEGREES) % SEXTANTS);
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [(r1 + m) * CHANNEL_MAX, (g1 + m) * CHANNEL_MAX, (b1 + m) * CHANNEL_MAX].map(|c| c as f32)
}
