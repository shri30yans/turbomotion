//! A taper: the stroke is thinner toward either end of the outline.
//!
//! Each end has a `length` over which the stroke grows from the width at its tip to the full width, and the tip's `width`.
//! The two are numbers in pixels or fractions (of the outline's length, of the full width). A tip can be at most half the
//! contour long, so the two never overlap: a longer one is held to half. How the width rises from a tip is the `ease`,
//! measured from the tip at both ends, so one ease gives two ends that mirror each other.

use serde::Serialize;

use super::Profile;
use crate::anim::ease::Ease;

/// A number that is pixels, or a fraction of the whole it is measured against.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Dim {
    pub value: f64,
    pub fraction: bool,
}

impl Dim {
    pub fn px(self, whole: f64) -> f64 {
        if self.fraction { self.value * whole } else { self.value }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Tip {
    /// How far from the end the stroke takes to reach its full width; a fraction is of the contour's length.
    pub length: Dim,
    /// How wide the stroke is at the end itself; a fraction is of the full width.
    pub width: Dim,
}

/// A taper at one instant.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Taper {
    pub start: Option<Tip>,
    pub end: Option<Tip>,
    #[serde(skip)]
    pub ease: Ease,
}

impl Taper {
    /// The widest the stroke gets under this taper, for the bounds of the layer.
    pub fn widest(&self, full: f64) -> f64 {
        [self.start, self.end].into_iter().flatten().map(|t| t.width.px(full)).fold(full, f64::max)
    }

    /// The width along a contour `length` pixels long whose stroke is `full` wide between the tapers.
    pub fn profile(&self, length: f64, full: f64) -> Tapered {
        let ramp = |t: &Tip| Ramp { length: t.length.px(length).max(0.0).min(length / 2.0), tip: t.width.px(full) };
        Tapered { length, full, start: self.start.as_ref().map(ramp), end: self.end.as_ref().map(ramp), ease: self.ease }
    }
}

#[derive(Debug, Clone, Copy)]
struct Ramp {
    length: f64,
    tip: f64,
}

/// A [`Taper`] on one contour.
#[derive(Debug, Clone)]
pub struct Tapered {
    length: f64,
    full: f64,
    start: Option<Ramp>,
    end: Option<Ramp>,
    ease: Ease,
}

impl Tapered {
    /// The width `from_tip` pixels from an end whose ramp is `ramp`, or `None` beyond it.
    fn within(&self, ramp: Option<Ramp>, from_tip: f64) -> Option<f64> {
        let r = ramp.filter(|r| from_tip < r.length)?;
        Some(r.tip + (self.full - r.tip) * self.ease.at(from_tip / r.length))
    }
}

impl Profile for Tapered {
    fn width(&self, s: f64) -> f64 {
        self.within(self.start, s).or_else(|| self.within(self.end, self.length - s)).unwrap_or(self.full)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dim(value: f64) -> Dim {
        Dim { value, fraction: false }
    }

    fn tip(length: f64, width: f64) -> Option<Tip> {
        Some(Tip { length: dim(length), width: dim(width) })
    }

    #[test]
    fn the_width_rises_linearly_from_each_tip_and_is_full_between() {
        let p = Taper { start: tip(100.0, 0.0), end: tip(50.0, 10.0), ease: Ease::Linear }.profile(400.0, 20.0);
        let w = |s| p.width(s);
        assert_eq!([w(0.0), w(50.0), w(100.0), w(250.0)], [0.0, 10.0, 20.0, 20.0]);
        assert_eq!([w(350.0), w(375.0), w(400.0)], [20.0, 15.0, 10.0]);
    }

    #[test]
    fn fractions_are_of_the_contour_length_and_of_the_full_width() {
        let half = Dim { value: 0.25, fraction: true };
        let p = Taper { start: Some(Tip { length: half, width: half }), end: None, ease: Ease::Linear }.profile(200.0, 40.0);
        assert_eq!([p.width(0.0), p.width(25.0), p.width(50.0), p.width(199.0)], [10.0, 25.0, 40.0, 40.0]);
    }

    #[test]
    fn a_taper_longer_than_half_the_contour_is_held_to_half_so_the_two_meet_at_full_width() {
        let p = Taper { start: tip(900.0, 0.0), end: tip(900.0, 0.0), ease: Ease::Linear }.profile(100.0, 8.0);
        assert_eq!([p.width(0.0), p.width(25.0), p.width(50.0), p.width(75.0), p.width(100.0)], [0.0, 4.0, 8.0, 4.0, 0.0]);
        let none = Taper { start: tip(0.0, 3.0), end: None, ease: Ease::Linear }.profile(100.0, 8.0);
        assert_eq!([none.width(0.0), none.width(50.0)], [8.0, 8.0], "a taper of no length does nothing");
    }

    #[test]
    fn the_ease_is_measured_from_the_tip_at_both_ends() {
        let ease = Ease::parse("power2.in").expect("a GSAP ease");
        let p = Taper { start: tip(100.0, 0.0), end: tip(100.0, 0.0), ease }.profile(300.0, 10.0);
        for from_tip in [0.0, 10.0, 40.0, 99.0] {
            assert_eq!(p.width(from_tip), p.width(300.0 - from_tip), "{from_tip}");
            assert!((p.width(from_tip) - 10.0 * ease.at(from_tip / 100.0)).abs() < 1e-12);
        }
    }

    #[test]
    fn the_widest_is_the_full_width_unless_a_tip_is_wider() {
        assert_eq!(Taper { start: tip(10.0, 0.0), end: None, ease: Ease::Linear }.widest(12.0), 12.0);
        assert_eq!(Taper { start: tip(10.0, 30.0), end: tip(10.0, 5.0), ease: Ease::Linear }.widest(12.0), 30.0);
    }
}
