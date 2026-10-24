//! Element timing and the shared in/out animation: every element is built as a layer and landed with `blit`, which
//! applies the named entrance (`rise`, `pop`, `tilt`...) and the fade-out exactly as the reference does.

use anyhow::Result;
use skia_safe::Canvas;

use super::{fx, model::{Common}};
use turbomotion_core::{ease::{back_out, clamp, expo_out, pow_in, pow_out}, layer::{place, Layer, Place, Shadow}};

/// An element's clock: its animation settings and the time being drawn.
#[derive(Clone, Copy)]
pub struct Timing<'a> {
    pub common: &'a Common,
    pub t: f64,
}

impl<'a> Timing<'a> {
    pub fn new(common: &'a Common, t: f64) -> Self {
        Self { common, t }
    }

    pub fn t0(&self) -> f64 {
        self.common.t0
    }

    /// Entrance progress 0..1 over `d` seconds (default: the element's own `d`, else 0.45).
    pub fn prog(&self, d: Option<f64>) -> f64 {
        let d = d.or(self.common.d).unwrap_or(0.45);
        clamp((self.t - self.common.t0) / d.max(1e-3))
    }

    /// Exit progress 0..1; elements without an end time never leave.
    pub fn out_prog(&self) -> f64 {
        match self.common.t1 {
            None => 0.0,
            Some(t1) => {
                let d = self.common.dout.unwrap_or(0.3);
                clamp((self.t - (t1 - d)) / d)
            }
        }
    }

    pub fn opacity(&self) -> f64 {
        self.common.op.unwrap_or(1.0)
    }
}

/// Opacity, scale, offsets and tilt an entrance contributes at progress `u`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entrance {
    pub op: f64,
    pub sc: f64,
    pub dx: f64,
    pub dy: f64,
    pub rot: f64,
    pub rx: f64,
}

impl Entrance {
    const REST: Self = Self { op: 1.0, sc: 1.0, dx: 0.0, dy: 0.0, rot: 0.0, rx: 0.0 };

    pub fn of(name: &str, u: f64) -> Self {
        let rest = Self::REST;
        match name {
            "none" => Self { op: if u > 0.0 { 1.0 } else { 0.0 }, ..rest },
            "fade" => Self { op: pow_out(u, 2.0), ..rest },
            "rise" => Self { op: clamp(u * 2.5), dy: (1.0 - expo_out(u)) * 44.0, ..rest },
            "drop" => Self { op: clamp(u * 3.0), dy: -(1.0 - expo_out(u)) * 90.0, ..rest },
            "pop" => Self { op: clamp(u * 4.0), sc: 0.65 + 0.35 * back_out(u, 2.2), ..rest },
            "zoom" => Self { op: clamp(u * 2.5), sc: 1.3 - 0.3 * expo_out(u), ..rest },
            "slide_l" => Self { op: clamp(u * 3.0), dx: -(1.0 - expo_out(u)) * 520.0, ..rest },
            "slide_r" => Self { op: clamp(u * 3.0), dx: (1.0 - expo_out(u)) * 520.0, ..rest },
            "tilt" => Self { op: clamp(u * 3.0), rx: -62.0 * (1.0 - expo_out(u)), dy: (1.0 - expo_out(u)) * 30.0, ..rest },
            _ => rest,
        }
    }
}

/// How `blit` lands a layer.
#[derive(Default, Clone, Copy)]
pub struct Blit<'a> {
    /// Entrance used when the element names none.
    pub default_in: &'a str,
    /// Use this entrance instead of the element's own (text wipes finish their own reveal, then land with `none`).
    pub force_in: Option<&'a str>,
    pub rot: f64,
    pub scale: f64,
    pub shadow: Option<Shadow>,
}

impl<'a> Blit<'a> {
    pub fn new(default_in: &'a str) -> Self {
        Self { default_in, scale: 1.0, ..Self::default() }
    }
    pub fn rot(mut self, r: f64) -> Self {
        self.rot = r;
        self
    }
    pub fn scale(mut self, s: f64) -> Self {
        self.scale = s;
        self
    }
    pub fn shadow(mut self, s: Shadow) -> Self {
        self.shadow = Some(s);
        self
    }
    pub fn force_in(mut self, name: &'a str) -> Self {
        self.force_in = Some(name);
        self
    }
}

/// Land `layer` centred on (`cx`, `cy`) with the element's entrance and exit.
pub fn blit(cv: &Canvas, layer: &Layer, cx: f64, cy: f64, tm: &Timing, b: Blit) -> Result<()> {
    let u = tm.prog(None);
    if u <= 0.0 {
        return Ok(());
    }
    let name = b.force_in.or(tm.common.enter.as_deref()).unwrap_or(b.default_in);
    let mut ap = Entrance::of(name, u);
    let uo = tm.out_prog();
    if uo >= 1.0 {
        return Ok(());
    }
    let op = ap.op * (1.0 - pow_in(uo, 2.0)) * tm.opacity();
    if uo > 0.0 && tm.common.out.as_deref().unwrap_or("fade") == "rise" {
        ap.dy -= uo * 30.0;
    }
    let tilted;
    let mut lay = layer;
    if ap.rx != 0.0 {
        tilted = fx::tilt(cv, lay, ap.rx, 0.0, 0.0011)?;
        lay = &tilted;
    }
    let shadowed;
    if let Some(s) = &b.shadow {
        shadowed = lay.with_shadow(cv, s)?;
        lay = &shadowed;
    }
    place(cv, lay, &Place::at(cx + ap.dx, cy + ap.dy).scale(b.scale * ap.sc).rot(b.rot + ap.rot).opacity(op));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entrances_rest_at_full_progress() {
        for name in ["fade", "rise", "drop", "pop", "zoom", "slide_l", "slide_r", "tilt"] {
            let e = Entrance::of(name, 1.0);
            assert!((e.op - 1.0).abs() < 1e-9 && (e.sc - 1.0).abs() < 1e-6 && e.dx.abs() < 1e-3 && e.dy.abs() < 1e-3 && e.rx.abs() < 1e-3, "{name}: {e:?}");
        }
    }

    #[test]
    fn unfinished_elements_never_exit() {
        let c = Common::default();
        assert_eq!(Timing::new(&c, 10.0).out_prog(), 0.0);
    }
}
