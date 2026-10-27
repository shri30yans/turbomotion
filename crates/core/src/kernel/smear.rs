//! Smears: the mean of a picture along a path, a line integral and not a row of copies. A directional blur is a path
//! that shifts, a radial blur one that turns (spin) or scales (zoom) about a centre; all three are one shader.

use anyhow::{ensure, Result};
use skia_safe::Canvas;

use super::{
    run::{run, Reading},
    Edge,
    taps::MAX_SMEAR_TAPS,
};
use crate::{layer::Layer, shader::Uniforms, shaders::Shaders};

/// A path each pixel's colour is carried along, as `shift`, `turn` and `zoom` of its whole length, from `span.0` to
/// `span.1` of it (so `(-0.5, 0.5)` is centred on the pixel and `(0, 1)` starts at it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Smear {
    pub centre: [f64; 2],
    /// Pixels the path moves by.
    pub shift: [f64; 2],
    /// Degrees the path turns about the centre.
    pub turn: f64,
    /// Extra scale about the centre at the path's end (0.1: a tenth bigger).
    pub zoom: f64,
    pub span: (f64, f64),
}

impl Smear {
    /// A straight smear of `distance` pixels along `angle` degrees, centred on each pixel.
    pub fn line(distance: f64, angle: f64) -> Self {
        let (sin, cos) = angle.to_radians().sin_cos();
        Self { centre: [0.0; 2], shift: [cos * distance, sin * distance], turn: 0.0, zoom: 0.0, span: (-0.5, 0.5) }
    }

    /// A spin of `degrees` about `centre`, centred on each pixel.
    pub fn spin(centre: [f64; 2], degrees: f64) -> Self {
        Self { centre, shift: [0.0; 2], turn: degrees, zoom: 0.0, span: (-0.5, 0.5) }
    }

    /// A zoom from `centre` that scales the picture up to `1 + amount` times.
    pub fn zoom(centre: [f64; 2], amount: f64) -> Self {
        Self { centre, shift: [0.0; 2], turn: 0.0, zoom: amount, span: (0.0, 1.0) }
    }

    /// The most pixels the path is long at any pixel of a `w` x `h` picture: one sample is taken per pixel of it.
    fn longest(&self, w: i32, h: i32) -> f64 {
        let far = [(0.0, 0.0), (f64::from(w), 0.0), (0.0, f64::from(h)), (f64::from(w), f64::from(h))].iter().map(|c| (c.0 - self.centre[0]).hypot(c.1 - self.centre[1])).fold(0.0, f64::max);
        (self.shift[0].hypot(self.shift[1]) + far * (self.turn.to_radians().abs() + self.zoom.abs())) * (self.span.1 - self.span.0)
    }

    pub fn check(&self, w: i32, h: i32) -> Result<()> {
        ensure!([self.centre[0], self.centre[1], self.shift[0], self.shift[1], self.turn, self.zoom].iter().all(|v| v.is_finite()), "a smear needs finite numbers: {self:?}");
        let longest = self.longest(w, h);
        ensure!(longest <= f64::from(MAX_SMEAR_TAPS), "a smear {longest:.0} pixels long at its worst pixel needs a sample per pixel; the pass takes at most {MAX_SMEAR_TAPS}");
        Ok(())
    }

    /// Whether the path has any length at all (a picture is returned as it is when it does not).
    pub fn is_still(&self) -> bool {
        self.shift == [0.0; 2] && self.turn == 0.0 && self.zoom == 0.0
    }

    pub fn apply(&self, shaders: &Shaders, like: &Canvas, layer: &Layer) -> Result<Layer> {
        self.check(layer.width(), layer.height())?;
        if self.is_still() {
            return Ok(layer.clone());
        }
        let uniforms = Uniforms::new().v2(self.centre[0], self.centre[1]).v2(self.shift[0], self.shift[1]).f(self.turn.to_radians()).f(self.zoom).v2(self.span.0, self.span.1);
        run(like, layer, &shaders.kernel.smear, uniforms, Edge::Transparent, Reading::Smooth)
    }
}
