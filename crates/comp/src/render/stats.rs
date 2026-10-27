//! Measuring rendered pictures: what the grain and dither tests share. A channel of a BGRA frame becomes a plane of
//! numbers (levels 0..255) from which means, deviations, correlations and autocorrelations are read.

/// One channel of a frame as a plane `w` wide: `c` is the byte of a BGRA pixel (0 blue, 1 green, 2 red).
pub struct Plane {
    pub w: usize,
    pub h: usize,
    pub v: Vec<f64>,
}

impl Plane {
    pub fn of(px: &[u8], w: usize, h: usize, c: usize) -> Self {
        Self { w, h, v: px.chunks_exact(4).map(|p| f64::from(p[c])).collect() }
    }

    pub fn mean(&self) -> f64 {
        self.v.iter().sum::<f64>() / self.v.len() as f64
    }

    /// Population standard deviation.
    pub fn std(&self) -> f64 {
        let m = self.mean();
        (self.v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / self.v.len() as f64).sqrt()
    }

    /// The autocorrelation at a lag of (`dx`, `dy`) pixels: the covariance of each pixel with the one that far away, over
    /// the variance, from every pair that lies inside the picture.
    pub fn autocorrelation(&self, dx: usize, dy: usize) -> f64 {
        let m = self.mean();
        let (mut cov, mut var) = (0.0, 0.0);
        for y in 0..self.h - dy {
            for x in 0..self.w - dx {
                cov += (self.v[y * self.w + x] - m) * (self.v[(y + dy) * self.w + x + dx] - m);
            }
        }
        for x in &self.v {
            var += (x - m).powi(2);
        }
        cov / (var * ((self.w - dx) * (self.h - dy)) as f64 / self.v.len() as f64)
    }

    /// The correlation coefficient between this plane and `other` (pixel for pixel).
    pub fn correlation(&self, other: &Plane) -> f64 {
        let (a, b) = (self.mean(), other.mean());
        let cov: f64 = self.v.iter().zip(&other.v).map(|(x, y)| (x - a) * (y - b)).sum();
        cov / (self.v.len() as f64 * self.std() * other.std())
    }
}
