//! The studio easings: GSAP-shaped curves in f64 (camera and counter maths must not lose precision to f32).

use std::f64::consts::PI;

pub fn clamp(u: f64) -> f64 {
    u.clamp(0.0, 1.0)
}

pub fn pow_out(u: f64, p: f64) -> f64 {
    1.0 - (1.0 - clamp(u)).powf(p)
}

pub fn pow_in(u: f64, p: f64) -> f64 {
    clamp(u).powf(p)
}

pub fn pow_io(u: f64, p: f64) -> f64 {
    let u = clamp(u);
    if u < 0.5 { 0.5 * (2.0 * u).powf(p) } else { 1.0 - 0.5 * (2.0 * (1.0 - u)).powf(p) }
}

pub fn expo_out(u: f64) -> f64 {
    let u = clamp(u);
    if u >= 1.0 { 1.0 } else { 1.0 - 2f64.powf(-10.0 * u) }
}

pub fn expo_io(u: f64) -> f64 {
    let u = clamp(u);
    if u == 0.0 || u == 1.0 {
        u
    } else if u < 0.5 {
        0.5 * 2f64.powf(20.0 * u - 10.0)
    } else {
        1.0 - 0.5 * 2f64.powf(-20.0 * u + 10.0)
    }
}

pub fn back_out(u: f64, s: f64) -> f64 {
    let u = clamp(u) - 1.0;
    1.0 + (s + 1.0) * u.powi(3) + s * u.powi(2)
}

pub fn elastic_out(u: f64, a: f64, p: f64) -> f64 {
    let u = clamp(u);
    if u == 0.0 || u == 1.0 { u } else { a * 2f64.powf(-10.0 * u) * ((u - p / 4.0) * 2.0 * PI / p).sin() + 1.0 }
}

pub fn sine_io(u: f64) -> f64 {
    -((PI * clamp(u)).cos() - 1.0) / 2.0
}

pub fn smooth(u: f64) -> f64 {
    let u = clamp(u);
    u * u * (3.0 - 2.0 * u)
}

/// An easing by its plan name (`ease: 'io'`).
pub fn named(name: &str, u: f64) -> Option<f64> {
    Some(match name {
        "lin" => clamp(u),
        "out" => pow_out(u, 3.0),
        "in_" => pow_in(u, 3.0),
        "io" => pow_io(u, 3.0),
        "expo" => expo_out(u),
        "expo_io" => expo_io(u),
        "back" => back_out(u, 1.70158),
        "elastic" => elastic_out(u, 1.0, 0.35),
        "sine" => sine_io(u),
        "smooth" => smooth(u),
        _ => return None,
    })
}

/// A value animated through `(time, value)` keys: held before the first and after the last key, smoothstep between.
pub fn keyed(keys: &[(f64, f64)], t: f64) -> f64 {
    let (Some(first), Some(last)) = (keys.first(), keys.last()) else { return 0.0 };
    if t <= first.0 {
        return first.1;
    }
    keys.windows(2).find(|w| t < w[1].0).map_or(last.1, |w| w[0].1 + (w[1].1 - w[0].1) * smooth((t - w[0].0) / (w[1].0 - w[0].0).max(1e-3)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyed_values_hold_and_blend() {
        let keys = [(1.0, 10.0), (2.0, 20.0), (4.0, 0.0)];
        assert_eq!(keyed(&keys, 0.0), 10.0);
        assert_eq!(keyed(&keys, 1.5), 15.0);
        assert_eq!(keyed(&keys, 3.0), 10.0);
        assert_eq!(keyed(&keys, 9.0), 0.0);
    }

    #[test]
    fn easings_pin_both_ends() {
        for f in [expo_out, expo_io, sine_io, smooth] {
            assert!((f(0.0)).abs() < 1e-9 && (f(1.0) - 1.0).abs() < 1e-9);
        }
        assert!((back_out(1.0, 2.2) - 1.0).abs() < 1e-9);
    }
}
