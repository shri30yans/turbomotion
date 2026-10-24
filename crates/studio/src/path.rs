//! Smooth paths through control points, sampled to polylines (the reference `path_pts`).

pub type Pt = [f64; 2];

/// `n` samples along the path through `pts`: a line for two points, a quadratic Bézier (the middle point is the control
/// point) for three, a Catmull-Rom spline through all of them otherwise.
pub fn sample(pts: &[Pt], n: usize) -> Vec<Pt> {
    match pts.len() {
        0 | 1 => pts.to_vec(),
        2 => (0..n).map(|i| lerp(pts[0], pts[1], i as f64 / (n - 1) as f64)).collect(),
        3 => (0..n)
            .map(|i| {
                let s = i as f64 / (n - 1) as f64;
                let (a, b, c) = ((1.0 - s) * (1.0 - s), 2.0 * (1.0 - s) * s, s * s);
                [a * pts[0][0] + b * pts[1][0] + c * pts[2][0], a * pts[0][1] + b * pts[1][1] + c * pts[2][1]]
            })
            .collect(),
        _ => catmull_rom(pts, n),
    }
}

fn lerp(a: Pt, b: Pt, s: f64) -> Pt {
    [a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s]
}

fn catmull_rom(pts: &[Pt], n: usize) -> Vec<Pt> {
    let segments = pts.len() - 1;
    let mut ext = Vec::with_capacity(pts.len() + 2);
    ext.push(pts[0]);
    ext.extend_from_slice(pts);
    ext.push(pts[pts.len() - 1]);
    let per = (n / segments).max(4);
    let mut out = Vec::new();
    for i in 0..segments {
        let (p0, p1, p2, p3) = (ext[i], ext[i + 1], ext[i + 2], ext[i + 3]);
        let last = i == segments - 1;
        for k in 0..per {
            // np.linspace(0, 1, per, endpoint=last): the last segment reaches 1, the others stop short of it
            let s = if last { k as f64 / (per - 1) as f64 } else { k as f64 / per as f64 };
            let point = |d: usize| {
                0.5 * ((2.0 * p1[d]) + (-p0[d] + p2[d]) * s + (2.0 * p0[d] - 5.0 * p1[d] + 4.0 * p2[d] - p3[d]) * s * s + (-p0[d] + 3.0 * p1[d] - 3.0 * p2[d] + p3[d]) * s * s * s)
            };
            out.push([point(0), point(1)]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_points_make_a_line_and_three_a_curve() {
        let line = sample(&[[0.0, 0.0], [10.0, 20.0]], 5);
        assert_eq!(line.len(), 5);
        assert_eq!(line[2], [5.0, 10.0]);
        let curve = sample(&[[0.0, 0.0], [10.0, 20.0], [20.0, 0.0]], 5);
        assert_eq!(curve[2], [10.0, 10.0]);
    }

    #[test]
    fn splines_end_on_the_last_control_point() {
        let pts = [[0.0, 0.0], [10.0, 5.0], [20.0, 0.0], [30.0, 8.0]];
        let path = sample(&pts, 60);
        assert_eq!(path.first().unwrap(), &pts[0]);
        let last = path.last().unwrap();
        assert!((last[0] - 30.0).abs() < 1e-9 && (last[1] - 8.0).abs() < 1e-9);
        assert_eq!(path.len(), 60);
        // numpy reference (float32): path_pts(..., 60)[19]
        assert!((path[19][0] - 9.48812484741211).abs() < 1e-5 && (path[19][1] - 4.969687461853027).abs() < 1e-5);
    }
}
