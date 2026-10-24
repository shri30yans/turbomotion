//! CSS `filter` colour functions (`contrast`, `saturate`, `sepia`) as one Skia colour filter.
//! The matrices are the ones in the Filter Effects spec; they act on sRGB values in 0..1.

use skia_safe::{color_filters, ColorFilter};

/// Row-major 4×5 colour matrix: `[r' g' b' a'] = M · [r g b a 1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix([[f32; 5]; 4]);

impl Matrix {

    pub fn contrast(amount: f32) -> Self {
        let offset = 0.5 - 0.5 * amount;
        Self([[amount, 0.0, 0.0, 0.0, offset], [0.0, amount, 0.0, 0.0, offset], [0.0, 0.0, amount, 0.0, offset], [0.0, 0.0, 0.0, 1.0, 0.0]])
    }

    pub fn saturate(s: f32) -> Self {
        Self([
            [0.213 + 0.787 * s, 0.715 - 0.715 * s, 0.072 - 0.072 * s, 0.0, 0.0],
            [0.213 - 0.213 * s, 0.715 + 0.285 * s, 0.072 - 0.072 * s, 0.0, 0.0],
            [0.213 - 0.213 * s, 0.715 - 0.715 * s, 0.072 + 0.928 * s, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0, 0.0],
        ])
    }

    pub fn sepia(amount: f32) -> Self {
        let k = 1.0 - amount;
        Self([
            [0.393 + 0.607 * k, 0.769 - 0.769 * k, 0.189 - 0.189 * k, 0.0, 0.0],
            [0.349 - 0.349 * k, 0.686 + 0.314 * k, 0.168 - 0.168 * k, 0.0, 0.0],
            [0.272 - 0.272 * k, 0.534 - 0.534 * k, 0.131 + 0.869 * k, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0, 0.0],
        ])
    }

    /// The matrix that applies `self`, then `next` (CSS applies filter functions left to right).
    pub fn then(self, next: Self) -> Self {
        let (a, b) = (self.0, next.0);
        let mut out = [[0.0; 5]; 4];
        for (r, row) in out.iter_mut().enumerate() {
            for (c, cell) in row.iter_mut().enumerate() {
                *cell = (0..4).map(|k| b[r][k] * a[k][c]).sum::<f32>() + if c == 4 { b[r][4] } else { 0.0 };
            }
        }
        Self(out)
    }

    pub fn filter(&self) -> ColorFilter {
        let mut flat = [0.0; 20];
        flat.iter_mut().zip(self.0.iter().flatten()).for_each(|(dst, src)| *dst = *src);
        color_filters::matrix_row_major(&flat, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(m: Matrix, rgb: [f32; 3]) -> [f32; 3] {
        let v = [rgb[0], rgb[1], rgb[2], 1.0, 1.0];
        std::array::from_fn(|r| m.0[r].iter().zip(v).map(|(a, b)| a * b).sum())
    }

    #[test]
    fn neutral_amounts_change_nothing() {
        let m = Matrix::contrast(1.0).then(Matrix::saturate(1.0)).then(Matrix::sepia(0.0));
        let out = apply(m, [0.2, 0.5, 0.8]);
        assert!(out.iter().zip([0.2, 0.5, 0.8]).all(|(a, b)| (a - b).abs() < 1e-5));
    }

    #[test]
    fn contrast_pivots_on_mid_grey_and_sepia_warms() {
        assert!((apply(Matrix::contrast(2.0), [0.5; 3])[0] - 0.5).abs() < 1e-6);
        let [r, _, b] = apply(Matrix::sepia(1.0), [0.5; 3]);
        assert!(r > b);
    }

    #[test]
    fn then_applies_left_to_right() {
        let m = Matrix::contrast(2.0).then(Matrix::sepia(1.0));
        let step = apply(Matrix::sepia(1.0), apply(Matrix::contrast(2.0), [0.3, 0.4, 0.5]));
        let fused = apply(m, [0.3, 0.4, 0.5]);
        assert!(step.iter().zip(fused).all(|(a, b)| (a - b).abs() < 1e-5));
    }
}
