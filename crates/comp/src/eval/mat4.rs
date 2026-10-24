//! 4x4 transforms (row-major, column vectors) for layers, cameras and projection.

use skia_safe::M44;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4(pub [f64; 16]);

impl Mat4 {
    pub const IDENTITY: Mat4 = Mat4([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);

    pub fn translate(x: f64, y: f64, z: f64) -> Mat4 {
        let mut m = Mat4::IDENTITY;
        m.0[3] = x;
        m.0[7] = y;
        m.0[11] = z;
        m
    }

    pub fn scale(x: f64, y: f64, z: f64) -> Mat4 {
        let mut m = Mat4::IDENTITY;
        m.0[0] = x;
        m.0[5] = y;
        m.0[10] = z;
        m
    }

    /// Rotation about the x axis, degrees (positive tips the top of the layer away, as in After Effects).
    pub fn rot_x(deg: f64) -> Mat4 {
        let (s, c) = deg.to_radians().sin_cos();
        let mut m = Mat4::IDENTITY;
        m.0[5] = c;
        m.0[6] = -s;
        m.0[9] = s;
        m.0[10] = c;
        m
    }

    pub fn rot_y(deg: f64) -> Mat4 {
        let (s, c) = deg.to_radians().sin_cos();
        let mut m = Mat4::IDENTITY;
        m.0[0] = c;
        m.0[2] = s;
        m.0[8] = -s;
        m.0[10] = c;
        m
    }

    /// Clockwise on screen (y points down).
    pub fn rot_z(deg: f64) -> Mat4 {
        let (s, c) = deg.to_radians().sin_cos();
        let mut m = Mat4::IDENTITY;
        m.0[0] = c;
        m.0[1] = -s;
        m.0[4] = s;
        m.0[5] = c;
        m
    }

    pub fn mul(&self, o: &Mat4) -> Mat4 {
        let mut r = [0.0; 16];
        for i in 0..4 {
            for j in 0..4 {
                r[i * 4 + j] = (0..4).map(|k| self.0[i * 4 + k] * o.0[k * 4 + j]).sum();
            }
        }
        Mat4(r)
    }

    /// Transform a point; returns `(x, y, z, w)` before the perspective divide.
    pub fn apply(&self, x: f64, y: f64, z: f64) -> [f64; 4] {
        let m = &self.0;
        [
            m[0] * x + m[1] * y + m[2] * z + m[3],
            m[4] * x + m[5] * y + m[6] * z + m[7],
            m[8] * x + m[9] * y + m[10] * z + m[11],
            m[12] * x + m[13] * y + m[14] * z + m[15],
        ]
    }

    /// Transform and divide by w. `None` when the point is behind the projection plane.
    pub fn project(&self, x: f64, y: f64, z: f64) -> Option<[f64; 3]> {
        let [px, py, pz, w] = self.apply(x, y, z);
        (w > 1e-6 || (w - 1.0).abs() < 1e-9).then(|| [px / w, py / w, pz / w])
    }

    pub fn to_m44(&self) -> M44 {
        let m = self.0.map(|v| v as f32);
        M44::row_major(&m)
    }

    pub fn inverse(&self) -> Option<Mat4> {
        let m = &self.0;
        let mut a = [[0.0f64; 8]; 4];
        for i in 0..4 {
            for j in 0..4 {
                a[i][j] = m[i * 4 + j];
            }
            a[i][4 + i] = 1.0;
        }
        for c in 0..4 {
            let p = (c..4).max_by(|x, y| a[*x][c].abs().total_cmp(&a[*y][c].abs()))?;
            if a[p][c].abs() < 1e-12 {
                return None;
            }
            a.swap(c, p);
            let d = a[c][c];
            a[c].iter_mut().for_each(|v| *v /= d);
            for r in 0..4 {
                if r != c {
                    let f = a[r][c];
                    let row = a[c];
                    a[r].iter_mut().zip(row).for_each(|(v, b)| *v -= f * b);
                }
            }
        }
        let mut out = [0.0; 16];
        for i in 0..4 {
            for j in 0..4 {
                out[i * 4 + j] = a[i][4 + j];
            }
        }
        Some(Mat4(out))
    }

    /// Camera: looks from `eye` at `target`, x right, y down, z forward (the After Effects convention), `roll` degrees.
    pub fn look_at(eye: [f64; 3], target: [f64; 3], roll: f64) -> Mat4 {
        let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let cross = |a: [f64; 3], b: [f64; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let norm = |a: [f64; 3]| {
            let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-12);
            [a[0] / l, a[1] / l, a[2] / l]
        };
        let f = norm(sub(target, eye));
        let down = if f[1].abs() > 0.999 { [0.0, 0.0, 1.0] } else { [0.0, 1.0, 0.0] };
        let x = norm(cross(down, f));
        let y = cross(f, x);
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let view = Mat4([x[0], x[1], x[2], -dot(x, eye), y[0], y[1], y[2], -dot(y, eye), f[0], f[1], f[2], -dot(f, eye), 0.0, 0.0, 0.0, 1.0]);
        Mat4::rot_z(-roll).mul(&view)
    }

    /// Pinhole projection onto a `cx, cy` centred screen where a point `zoom` px in front of the camera draws 1:1.
    pub fn pinhole(zoom: f64, cx: f64, cy: f64) -> Mat4 {
        Mat4([zoom, 0.0, cx, 0.0, 0.0, zoom, cy, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_and_projection_round_trip() {
        let m = Mat4::translate(10.0, 20.0, 30.0).mul(&Mat4::rot_y(30.0)).mul(&Mat4::scale(2.0, 2.0, 2.0));
        let id = m.mul(&m.inverse().unwrap());
        for (a, b) in id.0.iter().zip(Mat4::IDENTITY.0) {
            assert!((a - b).abs() < 1e-9);
        }
        // a point on the 1:1 plane projects to itself
        let zoom = 2000.0;
        let view = Mat4::look_at([960.0, 540.0, -zoom], [960.0, 540.0, 0.0], 0.0);
        let p = Mat4::pinhole(zoom, 960.0, 540.0).mul(&view).project(100.0, 200.0, 0.0).unwrap();
        assert!((p[0] - 100.0).abs() < 1e-6 && (p[1] - 200.0).abs() < 1e-6, "{p:?}");
        // moving the point away from the camera shrinks its offset from the centre
        let far = Mat4::pinhole(zoom, 960.0, 540.0).mul(&view).project(1960.0, 540.0, 2000.0).unwrap();
        assert!((far[0] - (960.0 + 500.0)).abs() < 1e-6, "{far:?}");
    }
}
