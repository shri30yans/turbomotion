//! The film's reveal vocabulary (`motion()` in `core.ts`), as pure functions of time.
//!
//! GSAP's `fromTo(..., immediateRender: true)` holds the "from" state until the tween
//! starts and the "to" state after it ends, so every function here clamps.

use crate::ease::{Ease, SETTLE};

/// One tween's timing and easing.
#[derive(Debug, Clone, Copy)]
pub struct Tween {
    pub at: f32,
    pub duration: f32,
    pub ease: Ease,
}

impl Tween {
    pub const fn new(at: f32, duration: f32, ease: Ease) -> Self {
        Self { at, duration, ease }
    }

    /// Eased progress at `seconds`; 0 before the tween, 1 after.
    pub fn progress(&self, seconds: f32) -> f32 {
        self.ease.apply((seconds - self.at) / self.duration)
    }
}

/// Opacity and vertical offset of an element at some moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Presence {
    pub opacity: f32,
    pub dy: f32,
}

/// `fadeUp`: rise `y` pixels while fading in.
pub fn fade_up(seconds: f32, at: f32, y: f32, duration: f32) -> Presence {
    let p = Tween::new(at, duration, SETTLE).progress(seconds);
    Presence { opacity: p, dy: y * (1.0 - p) }
}

/// How far Remotion's GSAP leaves a `pop` element from where it belongs once the tween is over.
///
/// A scale tween from `from` to 1 about "50% 50%" of an SVG element comes out permanently
/// translated by -(1 - from) / 2 of the element's own width and height, during the tween and
/// after it (measured on the old renderer's output: a 1100 x 600 group popping from 0.6 sat
/// exactly (-220, -120) off). Every film made so far carries the offset, so it is reproduced:
/// pin dots, diagram badges and markers land where they did.
pub fn pop_residual(from: f32, size: (f32, f32)) -> (f32, f32) {
    let k = -(1.0 - from) / 2.0;
    (k * size.0, k * size.1)
}

/// `show`: plain fade-in.
pub fn show(seconds: f32, at: f32, duration: f32) -> f32 {
    Tween::new(at, duration, Ease::PowerOut(1)).progress(seconds)
}

/// `draw`: fraction of an SVG stroke revealed.
pub fn draw(seconds: f32, at: f32, duration: f32, ease: Ease) -> f32 {
    Tween::new(at, duration, ease).progress(seconds)
}

/// Mask reveal of the `index`th item in a staggered group: remaining slide-in as a
/// percentage of the item's own height (`from_percent` at the start, 0 when done).
pub fn mask_slide(seconds: f32, at: f32, stagger: f32, index: usize, duration: f32, from_percent: f32) -> f32 {
    from_percent * (1.0 - Tween::new(at + stagger * index as f32, duration, SETTLE).progress(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_the_from_state_before_the_tween_and_the_to_state_after() {
        let held = fade_up(0.0, 1.0, 18.0, 0.6);
        assert!(held.opacity.abs() < 1e-6 && held.dy == 18.0);
        let done = fade_up(5.0, 1.0, 18.0, 0.6);
        assert_eq!((done.opacity, done.dy), (1.0, 0.0));
        assert_eq!(mask_slide(0.0, 1.0, 0.1, 3, 0.7, 110.0), 110.0);
        assert!(mask_slide(9.0, 1.0, 0.1, 3, 0.7, 110.0).abs() < 1e-4);
    }

    #[test]
    fn a_pop_leaves_the_element_shifted_by_a_share_of_its_size() {
        let near = |(x, y): (f32, f32), (ex, ey): (f32, f32)| (x - ex).abs() < 1e-3 && (y - ey).abs() < 1e-3;
        assert!(near(pop_residual(0.6, (1100.0, 600.0)), (-220.0, -120.0)));
        assert!(near(pop_residual(0.0, (180.0, 90.0)), (-90.0, -45.0)));
        assert!(near(pop_residual(1.0, (50.0, 50.0)), (0.0, 0.0)));
    }

    #[test]
    fn stagger_delays_later_items() {
        let first = mask_slide(1.3, 1.0, 0.1, 0, 0.7, 110.0);
        let third = mask_slide(1.3, 1.0, 0.1, 2, 0.7, 110.0);
        assert!(third > first);
    }
}
