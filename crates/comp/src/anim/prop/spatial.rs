//! Motion paths: the curve a position follows in space, measured by distance travelled.
//!
//! Between two keys the path is one cubic Bezier from the first key to the second, its inner control points the keys'
//! handles. Position is looked up by arc length, so a speed along the path (pixels per second) is a speed in space.

use super::key::{Point, Tangents};
use crate::anim::bezier;

/// Cells of the arc-length table of each piece; inside one cell the table is refined by Newton's method.
const CELLS: usize = 64;

/// Gauss-Legendre nodes and weights on `[-1, 1]`, five points: exact for polynomials to degree 9.
const GAUSS: [(f64, f64); 5] = [
    (0.0, 0.568_888_888_888_888_9),
    (-0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
    (0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
    (-0.906_179_845_938_664, 0.236_926_885_056_189_1),
    (0.906_179_845_938_664, 0.236_926_885_056_189_1),
];

/// A key as the path sees it: its place, with the handles resolved into offsets.
#[derive(Debug, Clone, Copy)]
pub(super) struct Stop {
    pub at: Point,
    pub into: Point,
    pub out: Point,
}

/// One cubic piece from stop to stop, with its length measured.
#[derive(Debug, Clone)]
struct Piece {
    c: [Point; 4],
    /// Length of the piece up to parameter `i / CELLS`.
    len_to: Vec<f64>,
}

fn at(c: &[Point; 4], u: f64) -> Point {
    std::array::from_fn(|k| bezier::point([c[0][k], c[1][k], c[2][k], c[3][k]], u))
}

fn velocity(c: &[Point; 4], u: f64) -> Point {
    std::array::from_fn(|k| bezier::slope([c[0][k], c[1][k], c[2][k], c[3][k]], u))
}

fn norm(p: Point) -> f64 {
    p.iter().map(|x| x * x).sum::<f64>().sqrt()
}

impl Piece {
    fn new(a: &Stop, b: &Stop) -> Piece {
        let add = |p: Point, d: Point| -> Point { std::array::from_fn(|k| p[k] + d[k]) };
        let c = [a.at, add(a.at, a.out), add(b.at, b.into), b.at];
        let mut piece = Piece { c, len_to: vec![0.0] };
        for i in 1..=CELLS {
            let (from, to) = ((i - 1) as f64 / CELLS as f64, i as f64 / CELLS as f64);
            let total = piece.len_to[i - 1] + piece.arc(from, to);
            piece.len_to.push(total);
        }
        piece
    }

    /// Length of the curve between parameters `from` and `to`.
    fn arc(&self, from: f64, to: f64) -> f64 {
        let (mid, half) = ((from + to) / 2.0, (to - from) / 2.0);
        half * GAUSS.iter().map(|(x, w)| w * norm(velocity(&self.c, mid + half * x))).sum::<f64>()
    }

    fn length(&self) -> f64 {
        self.len_to[CELLS]
    }

    /// The unit direction of travel at parameter `u`. Where the curve stands still for an instant (a handle of length
    /// zero at a key) it is the direction of the chord of the piece; zero for a piece that is a single point.
    fn heading(&self, u: f64) -> Point {
        let v = unit(velocity(&self.c, u));
        if v == [0.0; 3] { unit(std::array::from_fn(|k| self.c[3][k] - self.c[0][k])) } else { v }
    }

    /// The parameter at distance `s` along the piece (`0 <= s <= length`).
    fn param(&self, s: f64) -> f64 {
        let i = self.len_to.partition_point(|l| *l < s).clamp(1, CELLS);
        let (from, to) = ((i - 1) as f64 / CELLS as f64, i as f64 / CELLS as f64);
        let (l0, l1) = (self.len_to[i - 1], self.len_to[i]);
        if l1 <= l0 {
            return from;
        }
        let mut u = from + (s - l0) / (l1 - l0) * (to - from);
        for _ in 0..4 {
            let miss = l0 + self.arc(from, u) - s;
            let speed = norm(velocity(&self.c, u));
            if miss.abs() < 1e-12 * self.length().max(1.0) || speed < 1e-12 {
                break;
            }
            u = (u - miss / speed).clamp(from, to);
        }
        u
    }
}

/// A motion path through several stops, a piece between each two.
#[derive(Debug, Clone)]
pub(super) struct Path {
    pieces: Vec<Piece>,
    /// Distance along the path at the end of each piece.
    ends: Vec<f64>,
}

impl Path {
    /// The path through `stops` (at least two).
    pub fn through(stops: &[Stop]) -> Path {
        let pieces: Vec<Piece> = stops.windows(2).map(|w| Piece::new(&w[0], &w[1])).collect();
        let ends = pieces
            .iter()
            .scan(0.0, |total, p| {
                *total += p.length();
                Some(*total)
            })
            .collect();
        Path { pieces, ends }
    }

    pub fn length(&self) -> f64 {
        self.ends.last().copied().unwrap_or(0.0)
    }

    /// The piece that distance `s` falls in, and the distance along that piece. Past either end it is the first or
    /// last piece, extended: positions beyond the path continue straight along the end tangent.
    fn locate(&self, s: f64) -> (&Piece, f64) {
        let i = self.ends.partition_point(|e| *e < s).min(self.pieces.len() - 1);
        let start = if i == 0 { 0.0 } else { self.ends[i - 1] };
        (&self.pieces[i], s - start)
    }

    pub fn point(&self, s: f64) -> Point {
        let (piece, d) = self.locate(s);
        if (0.0..=piece.length()).contains(&d) {
            return at(&piece.c, piece.param(d));
        }
        let (edge, over) = if d < 0.0 { (0.0, d) } else { (1.0, d - piece.length()) };
        let (p, dir) = (at(&piece.c, edge), piece.heading(edge));
        std::array::from_fn(|k| p[k] + dir[k] * over)
    }

    /// The unit direction of travel at distance `s` (see `Piece::heading`).
    pub fn direction(&self, s: f64) -> Point {
        let (piece, d) = self.locate(s);
        piece.heading(piece.param(d.clamp(0.0, piece.length())))
    }
}

fn unit(v: Point) -> Point {
    let n = norm(v);
    if n < 1e-12 { [0.0; 3] } else { v.map(|x| x / n) }
}

/// The handles every stop gets: given ones as written, Auto Bezier ones from the neighbours.
///
/// Auto Bezier follows After Effects: the out handle is a sixth of the way from the previous point to the next, the in
/// handle its mirror; the first and last use their one neighbour. A side that faces a point in the same place has no
/// handle.
pub(super) fn stops(points: &[(Point, &Tangents)]) -> Vec<Stop> {
    let last = points.len().saturating_sub(1);
    points
        .iter()
        .enumerate()
        .map(|(i, (at, tangents))| match tangents {
            Tangents::Linear => Stop { at: *at, into: [0.0; 3], out: [0.0; 3] },
            Tangents::Bezier { into, out } => Stop { at: *at, into: *into, out: *out },
            Tangents::Auto => {
                let (prev, next) = (points[i.saturating_sub(1)].0, points[(i + 1).min(last)].0);
                let out: Point = std::array::from_fn(|k| (next[k] - prev[k]) / 6.0);
                let into = out.map(|x| -x);
                Stop {
                    at: *at,
                    into: if i > 0 && prev == *at { [0.0; 3] } else { into },
                    out: if i < last && next == *at { [0.0; 3] } else { out },
                }
            }
        })
        .collect()
}
