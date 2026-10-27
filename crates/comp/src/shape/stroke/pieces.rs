//! The stroke as a union of small polygons: one quadrilateral for each step between two points, a wedge or fan where the
//! outline turns a corner, and a cap at each end of an open contour.
//!
//! Every polygon is added with the same orientation, so filling the whole path with the nonzero rule is the union: where
//! pieces overlap (the inside of a bend) the winding only goes up, never to zero, so a tight bend cannot punch a hole the way a
//! single offset curve that folds back over itself does.

use std::f64::consts::PI;

use skia_safe::{Path, Point};

use super::{
    sample::{Sample, TOLERANCE},
    vec2::{add, cross, dot, lerp, scale, turn, V},
    Cap, Join, Style,
};

/// Skia's own default miter limit, which a stroke without a taper uses: a corner sharper than this is bevelled.
const MITER_LIMIT: f64 = 4.0;

/// Add the stroke of one contour, given its points, to `out`.
pub fn stroke(out: &mut Path, points: &[Sample], closed: bool, style: Style) {
    let mut pieces = Pieces { out };
    for pair in points.windows(2) {
        pieces.step(&pair[0], &pair[1]);
        if pair[1].corner {
            pieces.join(&pair[0], &pair[1], style.join);
        }
    }
    let (Some(first), Some(last)) = (points.first(), points.last()) else { return };
    if closed {
        pieces.join(last, first, style.join);
    } else {
        pieces.cap(first, scale(first.t, -1.0), style.cap);
        pieces.cap(last, last.t, style.cap);
    }
}

struct Pieces<'a> {
    out: &'a mut Path,
}

impl Pieces<'_> {
    fn polygon(&mut self, corners: &[V]) {
        let area: f64 = corners.iter().zip(corners.iter().cycle().skip(1)).map(|(a, b)| cross(*a, *b)).sum();
        let point = |c: &V| Point::new(c[0] as f32, c[1] as f32);
        let ordered: Vec<Point> = if area < 0.0 { corners.iter().rev().map(point).collect() } else { corners.iter().map(point).collect() };
        self.out.add_poly(&ordered, true);
    }

    /// The part of the stroke between two neighbouring points: the quadrilateral their edges make.
    fn step(&mut self, a: &Sample, b: &Sample) {
        let edge = |s: &Sample, side: f64| add(s.p, scale(s.n(), side * s.half));
        self.polygon(&[edge(a, 1.0), edge(b, 1.0), edge(b, -1.0), edge(a, -1.0)]);
    }

    /// Fill the outside of a turn from the direction of `a` to the direction of `b`, which the two quadrilaterals leave open.
    fn join(&mut self, a: &Sample, b: &Sample, join: Join) {
        let angle = cross(a.t, b.t).atan2(dot(a.t, b.t));
        if angle == 0.0 {
            return;
        }
        // the edges that lie on the outside of the turn
        let side = if angle > 0.0 { -1.0 } else { 1.0 };
        let (da, db) = (scale(a.n(), side), scale(b.n(), side));
        let (oa, ob) = (add(a.p, scale(da, a.half)), add(b.p, scale(db, b.half)));
        let mut wedge = vec![a.p, oa];
        match join {
            Join::Bevel => {}
            Join::Miter => {
                let reach = 1.0 / (angle.abs() / 2.0).cos();
                if reach <= MITER_LIMIT {
                    let half = (a.half + b.half) / 2.0;
                    let bisector = scale(add(da, db), 1.0 / (1.0 + dot(da, db)));
                    wedge.push(add(lerp(a.p, b.p, 0.5), scale(bisector, half)));
                }
            }
            Join::Round => {
                let steps = arc_steps(angle.abs(), a.half.max(b.half));
                wedge.extend((1..steps).map(|k| {
                    let u = k as f64 / steps as f64;
                    add(lerp(a.p, b.p, u), scale(turn(da, angle * u), a.half + (b.half - a.half) * u))
                }));
            }
        }
        wedge.extend([ob, b.p]);
        self.polygon(&wedge);
    }

    /// The end of an open contour at `at`, where the contour leaves in direction `outward`.
    fn cap(&mut self, at: &Sample, outward: V, cap: Cap) {
        let (across, h) = (at.n(), at.half);
        let side = |k: f64| add(at.p, scale(across, k * h));
        match cap {
            Cap::Butt => {}
            Cap::Square => self.polygon(&[side(1.0), add(side(1.0), scale(outward, h)), add(side(-1.0), scale(outward, h)), side(-1.0)]),
            Cap::Round => {
                let steps = arc_steps(PI, h);
                let arc = (0..=steps).map(|k| {
                    let phi = PI * k as f64 / steps as f64;
                    add(at.p, scale(add(scale(across, phi.cos()), scale(outward, phi.sin())), h))
                });
                self.polygon(&arc.collect::<Vec<_>>());
            }
        }
    }
}

/// How many straight pieces an arc of `angle` radians and `radius` pixels needs to stay within the tolerance of the circle.
fn arc_steps(angle: f64, radius: f64) -> usize {
    let per_step = if radius > TOLERANCE { 2.0 * (1.0 - TOLERANCE / radius).acos() } else { PI };
    ((angle / per_step).ceil() as usize).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(points: &[Point]) -> f32 {
        points.iter().zip(points.iter().cycle().skip(1)).map(|(a, b)| a.x * b.y - a.y * b.x).sum()
    }

    #[test]
    fn a_polygon_is_added_the_same_way_round_whichever_way_it_is_given() {
        let mut path = Path::new();
        let mut pieces = Pieces { out: &mut path };
        pieces.polygon(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]]);
        pieces.polygon(&[[0.0, 0.0], [0.0, 4.0], [4.0, 4.0], [4.0, 0.0]]);
        let mut pts = vec![Point::default(); path.count_points()];
        path.get_points(&mut pts);
        assert_eq!((area(&pts[..4]), area(&pts[4..])), (32.0, 32.0), "both squares came out in one orientation");
    }

    #[test]
    fn an_arc_gets_more_steps_as_the_radius_grows_and_never_fewer_than_one() {
        assert_eq!(arc_steps(PI, 0.01), 1);
        assert!(arc_steps(PI, 10.0) < arc_steps(PI, 1000.0));
    }
}
