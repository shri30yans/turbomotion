//! GSAP's easing vocabulary, parsed from strings like `power3.out`, `back.inOut(1.4)`, `elastic.out(1, 0.4)`,
//! `steps(8)`, `cubic(0.25, 0.1, 0.25, 1)`, `spring(0.5)`. Every ease maps 0..1 to 0..1 (overshooting eases leave the
//! range in between) and is a pure function of its input, so any frame can be evaluated alone.

use std::f64::consts::PI;
use std::fmt;

use anyhow::{anyhow, bail, ensure, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dir {
    In,
    Out,
    InOut,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ease {
    Linear,
    /// `power1`..`power4` are quad, cubic, quart, quint: the exponent is `power + 1`.
    Pow(f64, Dir),
    Sine(Dir),
    Expo(Dir),
    Circ(Dir),
    Back(f64, Dir),
    Elastic(f64, f64, Dir),
    Bounce(Dir),
    Steps(u32),
    Cubic(f64, f64, f64, f64),
    /// Damped spring by damping ratio: 1 never bounces, 0.7 reads premium, 0.3 is a cartoon. Settles by input 1.
    Spring(f64),
    /// Smoothstep.
    Smooth,
    /// Oscillates `n` times and dies away: for shakes (`from` 0, `to` the amplitude).
    Wiggle(u32),
    /// Jumps at the end.
    Hold,
    /// Jittery path to 1 (GSAP RoughEase): `strength`, `points`, deterministic `seed`.
    Rough(f64, u32, u32),
    /// Fast, then a long linger in the middle, then fast (GSAP SlowMo): `linear_ratio`, `power`.
    Slow(f64, f64),
    /// GSAP ExpoScaleEase: a scale from `start` to `end` that feels constant in speed. `expoScale(1, 2)`.
    ExpoScale(f64, f64),
}

impl Default for Ease {
    fn default() -> Self {
        Ease::Pow(2.0, Dir::Out)
    }
}

fn dir(name: &str) -> Result<Dir> {
    match name.to_ascii_lowercase().as_str() {
        "in" => Ok(Dir::In),
        "out" => Ok(Dir::Out),
        "inout" => Ok(Dir::InOut),
        other => bail!("unknown ease direction {other:?} (use in, out or inOut)"),
    }
}

impl Ease {
    pub fn parse(text: &str) -> Result<Ease> {
        let text = text.trim();
        let (head, args) = match text.find('(') {
            Some(i) => {
                ensure!(text.ends_with(')'), "ease {text:?} is missing its closing bracket");
                let args = text[i + 1..text.len() - 1].split(',').map(|a| a.trim().parse::<f64>().map_err(|_| anyhow!("ease {text:?}: {:?} is not a number", a.trim()))).collect::<Result<Vec<_>>>()?;
                (&text[..i], args)
            }
            None => (text, vec![]),
        };
        let arg = |i: usize, default: f64| args.get(i).copied().unwrap_or(default);
        let (name, d) = match head.split_once('.') {
            Some((n, d)) => (n, Some(dir(d)?)),
            None => (head, None),
        };
        let need = || d.ok_or_else(|| anyhow!("ease {text:?} needs a direction, like {name}.out"));
        Ok(match name.to_ascii_lowercase().as_str() {
            "none" | "linear" => Ease::Linear,
            "power1" | "quad" => Ease::Pow(2.0, need()?),
            "power2" | "cubic_" | "quadratic" => Ease::Pow(3.0, need()?),
            "power3" | "quart" => Ease::Pow(4.0, need()?),
            "power4" | "quint" => Ease::Pow(5.0, need()?),
            "sine" => Ease::Sine(need()?),
            "expo" => Ease::Expo(need()?),
            "circ" => Ease::Circ(need()?),
            "back" => Ease::Back(arg(0, 1.70158), need()?),
            "elastic" => Ease::Elastic(arg(0, 1.0).max(1.0), arg(1, 0.3), need()?),
            "bounce" => Ease::Bounce(need()?),
            "steps" => Ease::Steps(arg(0, 12.0).max(1.0) as u32),
            "cubic" | "bezier" => {
                ensure!(args.len() == 4, "cubic(x1, y1, x2, y2) takes four numbers");
                Ease::Cubic(args[0], args[1], args[2], args[3])
            }
            "spring" => Ease::Spring(arg(0, 0.6).max(0.05)),
            "smooth" | "smoothstep" => Ease::Smooth,
            "wiggle" => Ease::Wiggle(arg(0, 6.0).max(1.0) as u32),
            "hold" => Ease::Hold,
            "rough" => Ease::Rough(arg(0, 1.0), arg(1, 20.0).max(2.0) as u32, arg(2, 1.0) as u32),
            "slow" | "slowmo" => Ease::Slow(arg(0, 0.7), arg(1, 0.7)),
            "exposcale" => {
                ensure!(args.len() == 2, "expoScale(start, end) takes the scale you animate from and to");
                ensure!(args[0] != 0.0 && args[1] != 0.0 && args[0].signum() == args[1].signum(), "expoScale(start, end) needs two non-zero scales of the same sign");
                Ease::ExpoScale(args[0], args[1])
            }
            other => bail!("unknown ease {other:?}. Eases: none, power1-4, sine, expo, circ, back, elastic, bounce (each .in/.out/.inOut), steps(n), cubic(x1,y1,x2,y2), spring(zeta), smooth, wiggle(n), rough, slow, hold, expoScale(from, to)"),
        })
    }

    pub fn at(self, u: f64) -> f64 {
        let u = u.clamp(0.0, 1.0);
        match self {
            Ease::Linear => u,
            Ease::Pow(p, d) => shape(d, u, |x| x.powf(p)),
            Ease::Sine(d) => shape(d, u, |x| 1.0 - (x * PI / 2.0).cos()),
            Ease::Expo(d) => shape(d, u, |x| if x <= 0.0 { 0.0 } else { 2f64.powf(10.0 * (x - 1.0)) * (1.0 - 0.001 * (1.0 - x)) }),
            Ease::Circ(d) => shape(d, u, |x| 1.0 - (1.0 - x * x).max(0.0).sqrt()),
            Ease::Back(s, d) => match d {
                Dir::In => u * u * ((s + 1.0) * u - s),
                Dir::Out => {
                    let v = u - 1.0;
                    v * v * ((s + 1.0) * v + s) + 1.0
                }
                Dir::InOut => {
                    let s = s * 1.525;
                    let v = u * 2.0;
                    if v < 1.0 { 0.5 * (v * v * ((s + 1.0) * v - s)) } else { 0.5 * ((v - 2.0) * (v - 2.0) * ((s + 1.0) * (v - 2.0) + s) + 2.0) }
                }
            },
            Ease::Elastic(a, p, d) => {
                let p2 = p / (2.0 * PI) * (1.0 / a).asin();
                let out = |x: f64| if x <= 0.0 { 0.0 } else if x >= 1.0 { 1.0 } else { a * 2f64.powf(-10.0 * x) * ((x - p2) * 2.0 * PI / p).sin() + 1.0 };
                match d {
                    Dir::Out => out(u),
                    Dir::In => 1.0 - out(1.0 - u),
                    Dir::InOut => if u < 0.5 { 0.5 * (1.0 - out(1.0 - 2.0 * u)) } else { 0.5 + 0.5 * out(2.0 * u - 1.0) },
                }
            }
            Ease::Bounce(d) => {
                let out = bounce_out;
                match d {
                    Dir::Out => out(u),
                    Dir::In => 1.0 - out(1.0 - u),
                    Dir::InOut => if u < 0.5 { 0.5 * (1.0 - out(1.0 - 2.0 * u)) } else { 0.5 + 0.5 * out(2.0 * u - 1.0) },
                }
            }
            Ease::Steps(n) => if u >= 1.0 { 1.0 } else { (u * f64::from(n)).floor() / f64::from(n) },
            Ease::Cubic(x1, y1, x2, y2) => super::bezier::ease(x1, y1, x2, y2, u),
            Ease::Spring(zeta) => spring(zeta, u),
            Ease::Smooth => u * u * (3.0 - 2.0 * u),
            Ease::Wiggle(n) => (2.0 * PI * f64::from(n) * u).sin() * (1.0 - u),
            Ease::Hold => if u >= 1.0 { 1.0 } else { 0.0 },
            Ease::Rough(strength, points, seed) => rough(u, strength, points, seed),
            Ease::Slow(ratio, power) => slow(u, ratio, power),
            Ease::ExpoScale(start, end) => expo_scale(u, start, end),
        }
    }
}

impl fmt::Display for Ease {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Apply an "in" curve in the three directions.
fn shape(d: Dir, u: f64, f_in: impl Fn(f64) -> f64) -> f64 {
    match d {
        Dir::In => f_in(u),
        Dir::Out => 1.0 - f_in(1.0 - u),
        Dir::InOut => if u < 0.5 { 0.5 * f_in(2.0 * u) } else { 1.0 - 0.5 * f_in(2.0 * (1.0 - u)) },
    }
}

fn bounce_out(u: f64) -> f64 {
    const N: f64 = 7.5625;
    const D: f64 = 2.75;
    if u < 1.0 / D {
        N * u * u
    } else if u < 2.0 / D {
        let u = u - 1.5 / D;
        N * u * u + 0.75
    } else if u < 2.5 / D {
        let u = u - 2.25 / D;
        N * u * u + 0.9375
    } else {
        let u = u - 2.625 / D;
        N * u * u + 0.984375
    }
}

fn spring(zeta: f64, u: f64) -> f64 {
    if u >= 1.0 {
        return 1.0;
    }
    if zeta >= 1.0 {
        let w = 6.5;
        1.0 - (1.0 + w * u) * (-w * u).exp()
    } else {
        let w = 4.2 / zeta;
        let wd = w * (1.0 - zeta * zeta).sqrt();
        1.0 - (-zeta * w * u).exp() * ((wd * u).cos() + zeta * w / wd * (wd * u).sin())
    }
}

fn hash01(seed: u32, i: u32) -> f64 {
    let mut x = seed.wrapping_mul(0x9E37_79B1) ^ i.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    f64::from(x) / f64::from(u32::MAX)
}

fn rough(u: f64, strength: f64, points: u32, seed: u32) -> f64 {
    let x = u * f64::from(points);
    let i = x.floor() as u32;
    let f = x - x.floor();
    let jitter = |k: u32| if k == 0 || k >= points { 0.0 } else { (hash01(seed, k) - 0.5) * strength * 0.5 };
    let base = |k: u32| f64::from(k) / f64::from(points);
    let (a, b) = (base(i) + jitter(i), base(i + 1).min(1.0) + jitter(i + 1));
    (a + (b - a) * f).clamp(-0.5, 1.5)
}

/// GSAP ExpoScaleEase: `mix(start, end, e(t)) == start * (end/start)^t`.
fn expo_scale(u: f64, start: f64, end: f64) -> f64 {
    if (end - start).abs() < 1e-12 {
        return u;
    }
    let p = start * (end / start).powf(u);
    (p - start) / (end - start)
}

/// GSAP's SlowMo: eases in, lingers near the middle for `ratio` of the time, eases out.
fn slow(u: f64, ratio: f64, power: f64) -> f64 {
    let ratio = ratio.clamp(0.0, 1.0);
    let p1 = (1.0 - ratio) / 2.0;
    let p2 = ratio + p1;
    let r = u + (0.5 - u) * power;
    if u < p1 {
        let w = 1.0 - u / p1;
        r - w.powi(4) * r
    } else if u > p2 {
        let w = (u - p2) / p1;
        r + (u - r) * w.powi(4)
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gsap_names_and_pins_both_ends() {
        for name in ["none", "power1.in", "power2.out", "power3.inOut", "power4.out", "sine.inOut", "expo.out", "expo.inOut", "circ.in", "back.out(1.4)", "back.inOut", "elastic.out(1, 0.4)", "bounce.out", "bounce.inOut", "steps(6)", "cubic(0.25, 0.1, 0.25, 1)", "spring(0.5)", "smooth", "expoScale(1, 2)"] {
            let e = Ease::parse(name).unwrap_or_else(|err| panic!("{name}: {err}"));
            assert!(e.at(0.0).abs() < 1e-6, "{name} at 0 = {}", e.at(0.0));
            assert!((e.at(1.0) - 1.0).abs() < 1e-6, "{name} at 1 = {}", e.at(1.0));
        }
    }

    #[test]
    fn power2_is_cubic_and_back_overshoots() {
        assert!((Ease::parse("power2.in").unwrap().at(0.5) - 0.125).abs() < 1e-9);
        assert!(Ease::parse("back.out(1.7)").unwrap().at(0.7) > 1.0);
        assert!(Ease::parse("elastic.out").unwrap().at(0.2) > 1.0);
        let e = Ease::parse("expoScale(1, 2)").unwrap();
        assert!((1.0 * (2.0f64).powf(0.5) - (1.0 + e.at(0.5))).abs() < 1e-9);
    }

    #[test]
    fn mistakes_explain_themselves() {
        assert!(Ease::parse("power2").unwrap_err().to_string().contains("direction"));
        assert!(Ease::parse("zoomy.out").unwrap_err().to_string().contains("Eases:"));
        assert!(Ease::parse("cubic(1,2)").is_err());
    }

    #[test]
    fn cubic_matches_css_ease_midpoint() {
        let v = Ease::parse("cubic(0.25, 0.1, 0.25, 1)").unwrap().at(0.5);
        assert!((v - 0.8024).abs() < 1e-3, "{v}");
    }
}
