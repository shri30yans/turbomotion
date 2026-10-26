//! The cubic Bezier every curve of motion shares: the `cubic(x1, y1, x2, y2)` ease, the temporal curve of a keyframe's
//! speed and influence, and the motion path in space.

/// The cubic with control values `p0..p3` at parameter `u`.
pub fn point(p: [f64; 4], u: f64) -> f64 {
    let v = 1.0 - u;
    v * v * v * p[0] + 3.0 * v * v * u * p[1] + 3.0 * v * u * u * p[2] + u * u * u * p[3]
}

/// The derivative of the cubic by `u`.
pub fn slope(p: [f64; 4], u: f64) -> f64 {
    let v = 1.0 - u;
    3.0 * v * v * (p[1] - p[0]) + 6.0 * v * u * (p[2] - p[1]) + 3.0 * u * u * (p[3] - p[2])
}

/// Stop when the bracket around the root is this narrow: far below anything a frame can show, and above what f64
/// resolves near 1, so the loop always ends.
const BRACKET: f64 = 1e-15;

/// The parameter `u` at which the curve with control values `0, x1, x2, 1` reaches `x` (0 and 1 map to themselves).
///
/// The curve rises from 0 to 1 whenever `x1` and `x2` lie in `0..=1`. Newton's steps are kept inside a bracket that
/// bisection narrows whenever a step would leave it, so the answer is as accurate as the bracket even where the slope
/// is flat (both handles at full length stall Newton at the middle of the curve).
pub fn solve(x1: f64, x2: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let curve = [0.0, x1, x2, 1.0];
    let (mut lo, mut hi) = (0.0, 1.0);
    let mut u = x;
    while hi - lo > BRACKET {
        let miss = point(curve, u) - x;
        if miss == 0.0 {
            return u;
        }
        if miss > 0.0 { hi = u } else { lo = u }
        let step = u - miss / slope(curve, u);
        u = if step > lo && step < hi { step } else { 0.5 * (lo + hi) };
    }
    u
}

/// The ease `cubic(x1, y1, x2, y2)`: `y` as a function of `x`, both curves running from 0 to 1.
pub fn ease(x1: f64, y1: f64, x2: f64, y2: f64, x: f64) -> f64 {
    point([0.0, y1, y2, 1.0], solve(x1, x2, x))
}
