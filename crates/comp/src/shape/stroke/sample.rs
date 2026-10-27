//! Walking a contour into points close enough that straight lines between them are the stroke.
//!
//! A point is where the contour is at some distance `s`, which way it runs there and how wide the stroke is. An interval
//! of the contour is kept as one straight step when the centre line and both edges of the stroke stay within
//! [`TOLERANCE`] of the chord at a quarter, a half and three quarters of the way; otherwise it is halved. Halving stops
//! at [`MIN_STEP`]: an interval that is still not straight there is a corner (the direction jumps), and the point that
//! ends it says so, so the join can be drawn.

use skia_safe::ContourMeasure;

use super::vec2::{add, length, lerp, scale, sub, V};

/// How far a straight step may stray from the true edge of the stroke, in pixels: a fiftieth of a pixel.
pub const TOLERANCE: f64 = 0.02;
/// The shortest step: below this a bend is a corner, and the two points either side of it are one step apart.
const MIN_STEP: f64 = TOLERANCE;
/// A contour starts as this many equal steps so a feature narrower than a step of the walk cannot slip between the probes.
const FIRST_STEPS: usize = 16;
/// Where inside a step the straightness is checked.
const PROBES: [f64; 3] = [0.25, 0.5, 0.75];

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    /// Distance from the start of the contour.
    pub s: f64,
    pub p: V,
    /// The direction the contour runs, a unit vector.
    pub t: V,
    /// Half the width of the stroke here.
    pub half: f64,
    /// The direction jumps between the point before this one and this one.
    pub corner: bool,
}

impl Sample {
    /// The unit vector across the stroke, to the side the direction turns toward when it turns by a positive angle.
    pub fn n(&self) -> V {
        [-self.t[1], self.t[0]]
    }

    fn edge(&self, side: f64) -> V {
        add(self.p, scale(self.n(), side * self.half))
    }
}

struct Walk<'a> {
    contour: &'a ContourMeasure,
    width: &'a dyn Fn(f64) -> f64,
    out: Vec<Sample>,
}

/// The points of one contour, first to last. Empty when Skia cannot place a point on it.
pub fn contour(contour: &ContourMeasure, width: &dyn Fn(f64) -> f64) -> Vec<Sample> {
    let mut walk = Walk { contour, width, out: Vec::new() };
    let length = f64::from(contour.length());
    let (Some(first), true) = (walk.at(0.0), length > 0.0) else { return Vec::new() };
    walk.out.push(first);
    let mut from = first;
    for step in 1..=FIRST_STEPS {
        let Some(to) = walk.at(length * step as f64 / FIRST_STEPS as f64) else { return Vec::new() };
        walk.refine(from, to);
        from = to;
    }
    walk.out
}

impl Walk<'_> {
    fn at(&self, s: f64) -> Option<Sample> {
        let (p, t) = self.contour.pos_tan(s as f32)?;
        let t = [f64::from(t.x), f64::from(t.y)];
        let norm = length(t);
        (norm > 0.0).then(|| Sample { s, p: [f64::from(p.x), f64::from(p.y)], t: scale(t, 1.0 / norm), half: (self.width)(s).max(0.0) / 2.0, corner: false })
    }

    /// Whether the straight step from `a` to `b` is the contour and both edges of its stroke, within the tolerance.
    fn is_straight(&self, a: &Sample, b: &Sample) -> bool {
        PROBES.iter().all(|&u| {
            let Some(m) = self.at(a.s + (b.s - a.s) * u) else { return true };
            [(m.p, a.p, b.p), (m.edge(1.0), a.edge(1.0), b.edge(1.0)), (m.edge(-1.0), a.edge(-1.0), b.edge(-1.0))].iter().all(|&(at, from, to)| length(sub(at, lerp(from, to, u))) <= TOLERANCE)
        })
    }

    fn refine(&mut self, a: Sample, b: Sample) {
        if self.is_straight(&a, &b) {
            self.out.push(b);
            return;
        }
        let mid = if b.s - a.s > MIN_STEP { self.at((a.s + b.s) / 2.0) } else { None };
        match mid {
            Some(m) => {
                self.refine(a, m);
                self.refine(m, b);
            }
            None => self.out.push(Sample { corner: true, ..b }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skia_safe::{ContourMeasureIter, Path, Rect};

    fn walk(path: &Path, width: f64) -> Vec<Sample> {
        let c = ContourMeasureIter::new(path, false, None).next().expect("one contour");
        contour(&c, &|_| width)
    }

    #[test]
    fn a_straight_line_is_16_steps_and_a_corner_is_marked_where_the_direction_jumps() {
        let line = Path::polygon(&[(0.0, 0.0).into(), (160.0, 0.0).into()], false, None, None);
        assert_eq!(walk(&line, 10.0).len(), FIRST_STEPS + 1);
        let bend = Path::polygon(&[(0.0, 0.0).into(), (100.0, 0.0).into(), (100.0, 100.0).into()], false, None, None);
        let points = walk(&bend, 10.0);
        let corners: Vec<&Sample> = points.iter().filter(|p| p.corner).collect();
        assert_eq!(corners.len(), 1, "{points:?}");
        assert!((corners[0].s - 100.0).abs() <= MIN_STEP && (corners[0].p[0] - 100.0).abs() <= MIN_STEP, "{corners:?}");
        let before = points.iter().rev().find(|p| p.s < corners[0].s).expect("a point before the corner");
        assert!(before.t[0] > 0.999 && corners[0].t[1] > 0.999, "runs right, then down");
    }

    #[test]
    fn a_curve_is_walked_until_its_edges_are_within_the_tolerance_of_the_true_circle() {
        let circle = Path::oval(Rect::from_xywh(0.0, 0.0, 200.0, 200.0), None);
        let points = walk(&circle, 30.0);
        assert!(points.iter().all(|p| !p.corner) && points.len() > 40, "{}", points.len());
        let centre = [100.0, 100.0];
        for pair in points.windows(2) {
            let mid = lerp(pair[0].p, pair[1].p, 0.5);
            assert!((length(sub(mid, centre)) - 100.0).abs() <= TOLERANCE * 1.01, "{mid:?}");
            let edge = lerp(pair[0].edge(1.0), pair[1].edge(1.0), 0.5);
            let radius = length(sub(edge, centre));
            assert!((radius - 85.0).abs() <= TOLERANCE * 1.01 || (radius - 115.0).abs() <= TOLERANCE * 1.01, "{radius}");
        }
    }
}
