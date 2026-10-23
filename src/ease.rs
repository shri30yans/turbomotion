//! GSAP easing curves, as named in the Remotion shot components.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ease {
    None,
    /// GSAP `powerN`: exponent N+1.
    PowerIn(i32),
    PowerOut(i32),
    PowerInOut(i32),
    BackOut(f32),
    /// CSS-style cubic bézier through (0,0) and (1,1) with the given control points.
    Bezier([f32; 4]),
}

/// The film's `settle` ease (`CustomEase.create` in `core.ts`).
pub const SETTLE: Ease = Ease::Bezier([0.22, 1.0, 0.36, 1.0]);

impl Ease {
    /// Map progress `t` in 0..=1 to eased progress.
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::None => t,
            Self::PowerIn(n) => t.powi(n + 1),
            Self::PowerOut(n) => 1.0 - (1.0 - t).powi(n + 1),
            Self::PowerInOut(n) => {
                if t < 0.5 { 0.5 * (2.0 * t).powi(n + 1) } else { 1.0 - 0.5 * (2.0 * (1.0 - t)).powi(n + 1) }
            }
            Self::BackOut(s) => {
                let u = t - 1.0;
                u * u * ((s + 1.0) * u + s) + 1.0
            }
            Self::Bezier(p) => bezier(p, t),
        }
    }
}

/// Solve x(s) = t for the bézier parameter by bisection, then return y(s).
fn bezier([x1, y1, x2, y2]: [f32; 4], t: f32) -> f32 {
    let at = |a: f32, b: f32, s: f32| 3.0 * a * s * (1.0 - s).powi(2) + 3.0 * b * s * s * (1.0 - s) + s.powi(3);
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if at(x1, x2, mid) < t { lo = mid } else { hi = mid }
    }
    at(y1, y2, 0.5 * (lo + hi))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn every_ease_used_by_the_reference_is_pinned_at_both_ends() {
        for ease in [Ease::None, SETTLE, Ease::PowerOut(1), Ease::PowerInOut(1), Ease::PowerIn(2), Ease::PowerOut(2), Ease::PowerInOut(2), Ease::PowerOut(3), Ease::PowerInOut(3), Ease::BackOut(2.2), Ease::BackOut(2.0), Ease::BackOut(1.6)] {
            assert!(close(ease.apply(0.0), 0.0) && close(ease.apply(1.0), 1.0), "{ease:?}");
        }
    }

    #[test]
    fn matches_gsap_reference_values() {
        assert!(close(Ease::PowerOut(2).apply(0.5), 0.875));
        assert!(close(Ease::PowerInOut(2).apply(0.25), 0.0625));
        assert!(close(Ease::PowerOut(1).apply(0.5), 0.75));
        // back.out(1.7) is the canonical overshoot; GSAP's value at 0.5 is 1.0875.
        assert!(close(Ease::BackOut(1.7).apply(0.5), 1.0875));
    }

    #[test]
    fn settle_is_monotonic_and_front_loaded() {
        assert!((0..100).all(|i| SETTLE.apply(i as f32 / 100.0) <= SETTLE.apply((i + 1) as f32 / 100.0) + 1e-6));
        assert!(SETTLE.apply(0.3) > 0.7);
    }
}
