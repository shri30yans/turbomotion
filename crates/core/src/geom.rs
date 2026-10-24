//! Plane geometry shared by layers and transitions: 2x3 affine and 3x3 projective matrices with OpenCV's conventions
//! (pixel centres at integer coordinates), converted to Skia's continuous coordinates on demand.

use skia_safe::Matrix;

/// Row-major 3x3 matrix mapping source pixel indices to destination pixel indices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat3(pub [[f64; 3]; 3]);

impl Mat3 {
    pub fn translate(dx: f64, dy: f64) -> Self {
        Self([[1.0, 0.0, dx], [0.0, 1.0, dy], [0.0, 0.0, 1.0]])
    }

    /// `cv2.getRotationMatrix2D(centre, angle, scale)`: counter-clockwise `angle` degrees about `centre`.
    pub fn rotation_about(cx: f64, cy: f64, angle_deg: f64, scale: f64) -> Self {
        let (s, c) = angle_deg.to_radians().sin_cos();
        let (alpha, beta) = (scale * c, scale * s);
        Self([[alpha, beta, (1.0 - alpha) * cx - beta * cy], [-beta, alpha, beta * cx + (1.0 - alpha) * cy], [0.0, 0.0, 1.0]])
    }

    pub fn then_translate(mut self, dx: f64, dy: f64) -> Self {
        self.0[0][2] += dx;
        self.0[1][2] += dy;
        self
    }

    #[cfg(test)]
    fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let m = &self.0;
        let w = m[2][0] * x + m[2][1] * y + m[2][2];
        ((m[0][0] * x + m[0][1] * y + m[0][2]) / w, (m[1][0] * x + m[1][1] * y + m[1][2]) / w)
    }

    /// Skia draws in continuous coordinates where pixel `i` spans `[i, i + 1)`; OpenCV puts its centre at `i`.
    pub fn to_skia(&self) -> Matrix {
        let shifted = Self::translate(0.5, 0.5).mul(self).mul(&Self::translate(-0.5, -0.5)).0;
        Matrix::new_all(
            shifted[0][0] as f32,
            shifted[0][1] as f32,
            shifted[0][2] as f32,
            shifted[1][0] as f32,
            shifted[1][1] as f32,
            shifted[1][2] as f32,
            shifted[2][0] as f32,
            shifted[2][1] as f32,
            shifted[2][2] as f32,
        )
    }

    pub fn mul(&self, o: &Self) -> Self {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = (0..3).map(|k| self.0[i][k] * o.0[k][j]).sum();
            }
        }
        Self(r)
    }

    /// `cv2.getPerspectiveTransform`: the homography taking the four `src` points to the four `dst` points.
    pub fn perspective(src: [(f64, f64); 4], dst: [(f64, f64); 4]) -> Option<Self> {
        let mut a = [[0.0f64; 9]; 8];
        for i in 0..4 {
            let ((x, y), (u, v)) = (src[i], dst[i]);
            a[2 * i] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y, u];
            a[2 * i + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y, v];
        }
        for col in 0..8 {
            let pivot = (col..8).max_by(|&p, &q| a[p][col].abs().total_cmp(&a[q][col].abs()))?;
            if a[pivot][col].abs() < 1e-12 {
                return None;
            }
            a.swap(col, pivot);
            let d = a[col][col];
            a[col].iter_mut().for_each(|v| *v /= d);
            for r in 0..8 {
                if r != col {
                    let f = a[r][col];
                    let pivot_row = a[col];
                    a[r].iter_mut().zip(pivot_row).for_each(|(v, p)| *v -= f * p);
                }
            }
        }
        let h = |i: usize| a[i][8];
        Some(Self([[h(0), h(1), h(2)], [h(3), h(4), h(5)], [h(6), h(7), 1.0]]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_matches_opencv() {
        // cv2.getRotationMatrix2D((50, 40), 30, 1.5) = [[1.29904, 0.75, -44.95191], [-0.75, 1.29904, 25.53848]]
        let m = Mat3::rotation_about(50.0, 40.0, 30.0, 1.5).0;
        let want = [[1.2990381056766578, 0.75, -44.95190528383289], [-0.75, 1.2990381056766578, 25.538475772933676]];
        for (row, w) in m.iter().zip(want) {
            for (v, w) in row.iter().zip(w) {
                assert!((v - w).abs() < 1e-9, "{v} vs {w}");
            }
        }
    }

    #[test]
    fn perspective_maps_the_corners() {
        let src = [(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)];
        let dst = [(10.0, 5.0), (90.0, 0.0), (110.0, 60.0), (0.0, 45.0)];
        let h = Mat3::perspective(src, dst).unwrap();
        for (s, d) in src.iter().zip(dst) {
            let (x, y) = h.apply(s.0, s.1);
            assert!((x - d.0).abs() < 1e-8 && (y - d.1).abs() < 1e-8);
        }
    }
}
