//! One keyframe: a value at a time, and how the motion leaves and arrives.

use super::value::Value;
use crate::anim::ease::Ease;

/// A point in space: a position is `[x, y, 0]` for a 2D vector, `[x, y, z]` for a 3D one.
pub type Point = [f64; 3];

/// A temporal handle as After Effects shows it in the Keyframe Velocity dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct Handle {
    /// Units per second, one per dimension (or one for all). Along a motion path: pixels per second along the path.
    pub speed: Vec<f64>,
    /// How far across the segment's time the handle reaches, as a fraction `0..=1` (After Effects shows percent).
    pub influence: f64,
}

/// How the value leaves (`ease_out`) or arrives at (`ease_in`) a key.
#[derive(Debug, Clone, PartialEq)]
pub enum Side {
    /// A constant speed: the side of a key in a plain straight segment.
    Linear,
    /// The value stays at the key until the next one. Only a leaving side can hold.
    Hold,
    /// Auto Bezier: the speed of the straight line between the neighbouring keys.
    Auto,
    Bezier(Handle),
}

/// The motion path's handles at a key (spatial properties).
#[derive(Debug, Clone, PartialEq)]
pub enum Tangents {
    /// Auto Bezier: a smooth curve through the key, whatever the times (see `spatial::auto`).
    Auto,
    /// No handles: the path is straight into and out of the key.
    Linear,
    /// Handles as offsets from the key: `into` points back along the path, `out` forward.
    Bezier { into: Point, out: Point },
}

/// A roving keyframe: a point on the path whose time is not set but found, so that the motion between the fixed keys
/// around it keeps one smooth speed.
#[derive(Debug, Clone, PartialEq)]
pub struct Rove {
    pub at: Point,
    pub tangents: Tangents,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    pub t: f64,
    pub v: Value,
    /// A named ease for the move INTO this key. Left out, the default ease shapes it; it cannot be combined with
    /// `ease_in` here or `ease_out` on the key before.
    pub ease: Option<Ease>,
    pub ease_in: Option<Side>,
    pub ease_out: Option<Side>,
    pub tangents: Tangents,
    /// Roving keys between this key and the next one.
    pub rove: Vec<Rove>,
}

impl Key {
    pub fn new(t: f64, v: Value) -> Key {
        Key { t, v, ease: None, ease_in: None, ease_out: None, tangents: Tangents::Auto, rove: Vec::new() }
    }

    /// The same key, the move into it shaped by `ease`.
    pub fn eased(mut self, ease: Ease) -> Key {
        self.ease = Some(ease);
        self
    }
}
