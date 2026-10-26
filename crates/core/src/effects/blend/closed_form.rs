//! The expected result of every mode, written from the definitions (the W3C Compositing and Blending Level 1 formulas
//! After Effects follows, and `parity/specs/blend_modes.md` for the modes only After Effects has), in f64 and apart from
//! the shaders: a test compares the renderer to this.

use super::Blend;

pub type Rgb = [f64; 3];

/// A straight (not premultiplied) colour with its alpha.
#[derive(Debug, Clone, Copy)]
pub struct Px {
    pub c: Rgb,
    pub a: f64,
}

impl Px {
    pub fn premul(self) -> [f64; 4] {
        [self.c[0] * self.a, self.c[1] * self.a, self.c[2] * self.a, self.a]
    }
}

fn each(f: impl Fn(f64, f64) -> f64, cb: Rgb, cs: Rgb) -> Rgb {
    [f(cb[0], cs[0]), f(cb[1], cs[1]), f(cb[2], cs[2])]
}

fn burn(cb: f64, cs: f64) -> f64 {
    if cb >= 1.0 {
        1.0
    } else if cs <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - cb) / cs).min(1.0)
    }
}

fn dodge(cb: f64, cs: f64) -> f64 {
    if cb <= 0.0 {
        0.0
    } else if cs >= 1.0 {
        1.0
    } else {
        (cb / (1.0 - cs)).min(1.0)
    }
}

fn hard_light(cb: f64, cs: f64) -> f64 {
    if cs <= 0.5 {
        cb * 2.0 * cs
    } else {
        let s = 2.0 * cs - 1.0;
        cb + s - cb * s
    }
}

fn soft_light(cb: f64, cs: f64) -> f64 {
    if cs <= 0.5 {
        cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
    } else {
        let d = if cb <= 0.25 { ((16.0 * cb - 12.0) * cb + 4.0) * cb } else { cb.sqrt() };
        cb + (2.0 * cs - 1.0) * (d - cb)
    }
}

pub fn lum(c: Rgb) -> f64 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

pub fn sat(c: Rgb) -> f64 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

fn clip_color(c: Rgb) -> Rgb {
    let (l, n, x) = (lum(c), c[0].min(c[1]).min(c[2]), c[0].max(c[1]).max(c[2]));
    let mut out = c;
    if n < 0.0 {
        out = out.map(|v| l + (v - l) * l / (l - n));
    }
    if x > 1.0 {
        out = out.map(|v| l + (v - l) * (1.0 - l) / (x - l));
    }
    out
}

fn set_lum(c: Rgb, l: f64) -> Rgb {
    let d = l - lum(c);
    clip_color(c.map(|v| v + d))
}

fn set_sat(c: Rgb, s: f64) -> Rgb {
    let (min, max) = (c[0].min(c[1]).min(c[2]), c[0].max(c[1]).max(c[2]));
    if max > min {
        c.map(|v| (v - min) * s / (max - min))
    } else {
        [0.0; 3]
    }
}

/// B(cb, cs) of the modes that mix colours, on straight colours in 0..1.
fn mixed(blend: Blend, cb: Rgb, cs: Rgb) -> Rgb {
    let sum = |c: Rgb| c[0] + c[1] + c[2];
    match blend {
        Blend::Multiply => each(|b, s| b * s, cb, cs),
        Blend::Screen => each(|b, s| b + s - b * s, cb, cs),
        Blend::Overlay => each(|b, s| hard_light(s, b), cb, cs),
        Blend::HardLight => each(hard_light, cb, cs),
        Blend::SoftLight => each(soft_light, cb, cs),
        Blend::Darken => each(f64::min, cb, cs),
        Blend::Lighten => each(f64::max, cb, cs),
        Blend::ColorDodge => each(dodge, cb, cs),
        Blend::ColorBurn => each(burn, cb, cs),
        Blend::Difference => each(|b, s| (b - s).abs(), cb, cs),
        Blend::Exclusion => each(|b, s| b + s - 2.0 * b * s, cb, cs),
        Blend::Hue => set_lum(set_sat(cs, sat(cb)), lum(cb)),
        Blend::Saturation => set_lum(set_sat(cb, sat(cs)), lum(cb)),
        Blend::Color => set_lum(cs, lum(cb)),
        Blend::Luminosity => set_lum(cb, lum(cs)),
        Blend::DarkerColor => if sum(cs) < sum(cb) { cs } else { cb },
        Blend::LighterColor => if sum(cs) > sum(cb) { cs } else { cb },
        Blend::LinearBurn => each(|b, s| (b + s - 1.0).max(0.0), cb, cs),
        Blend::LinearLight => each(|b, s| (b + 2.0 * s - 1.0).clamp(0.0, 1.0), cb, cs),
        Blend::VividLight => each(|b, s| if s <= 0.5 { burn(b, 2.0 * s) } else { dodge(b, 2.0 * (s - 0.5)) }, cb, cs),
        Blend::PinLight => each(|b, s| if s <= 0.5 { b.min(2.0 * s) } else { b.max(2.0 * s - 1.0) }, cb, cs),
        Blend::HardMix => each(|b, s| if b + s >= 1.0 { 1.0 } else { 0.0 }, cb, cs),
        Blend::Subtract => each(|b, s| (b - s).max(0.0), cb, cs),
        Blend::Divide => each(|b, s| if s <= 0.0 { f64::from(u8::from(b > 0.0)) } else { (b / s).min(1.0) }, cb, cs),
        other => unreachable!("{other:?} does not mix colours by a function of the two colours"),
    }
}

/// The classic modes' formula on one channel, with the layer already faded toward `neutral`.
fn classic(blend: Blend, cb: f64, cs: f64) -> f64 {
    match blend {
        Blend::ClassicColorBurn => if cs <= 0.0 { 0.0 } else { (1.0 - (1.0 - cb) / cs).max(0.0) },
        Blend::ClassicColorDodge => if cs >= 1.0 { 1.0 } else { (cb / (1.0 - cs)).min(1.0) },
        Blend::ClassicDifference => (cb - cs).abs(),
        other => unreachable!("{other:?} is not a classic mode"),
    }
}

/// `source` laid on `backdrop` in `blend`, as premultiplied red, green, blue, alpha. Dissolves are not a function of the
/// two colours (they depend on the pixel's place), so they are not here.
pub fn expected(blend: Blend, backdrop: Px, source: Px) -> [f64; 4] {
    let (s, d) = (source.premul(), backdrop.premul());
    let (sa, da) = (source.a, backdrop.a);
    let over_alpha = sa + da - sa * da;
    let over = |i: usize| s[i] + d[i] * (1.0 - sa);
    let luma = 0.2126 * s[0] + 0.7152 * s[1] + 0.0722 * s[2];
    match blend {
        Blend::Normal | Blend::LuminescentPremul => [over(0), over(1), over(2), over_alpha],
        Blend::Add => std::array::from_fn(|i| (s[i] + d[i]).min(1.0)),
        Blend::StencilAlpha => d.map(|v| v * sa),
        Blend::SilhouetteAlpha => d.map(|v| v * (1.0 - sa)),
        Blend::StencilLuma => d.map(|v| v * luma.clamp(0.0, 1.0)),
        Blend::SilhouetteLuma => d.map(|v| v * (1.0 - luma.clamp(0.0, 1.0))),
        Blend::AlphaAdd => {
            let alpha = (sa + da).min(1.0);
            let k = if over_alpha > 0.0 { alpha / over_alpha } else { 0.0 };
            [over(0) * k, over(1) * k, over(2) * k, alpha]
        }
        Blend::ClassicColorBurn | Blend::ClassicColorDodge | Blend::ClassicDifference if sa > 0.0 && da > 0.0 => {
            let neutral = if blend == Blend::ClassicColorBurn { 1.0 } else { 0.0 };
            let faded = source.c.map(|v| neutral + sa * (v.clamp(0.0, 1.0) - neutral));
            let b = each(|cb, cs| classic(blend, cb, cs), backdrop.c.map(|v| v.clamp(0.0, 1.0)), faded);
            std::array::from_fn(|i| if i == 3 { over_alpha } else { s[i] * (1.0 - da) + da * b[i] })
        }
        _ if sa <= 0.0 => d,
        _ if da <= 0.0 => s,
        _ => {
            let b = mixed(blend, backdrop.c.map(|v| v.clamp(0.0, 1.0)), source.c.map(|v| v.clamp(0.0, 1.0)));
            std::array::from_fn(|i| if i == 3 { over_alpha } else { s[i] * (1.0 - da) + d[i] * (1.0 - sa) + sa * da * b[i] })
        }
    }
}
