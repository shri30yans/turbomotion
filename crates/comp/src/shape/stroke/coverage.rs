//! Measuring a drawn stroke: the coverage of every pixel, and the width of what is drawn across a line. Tests only.

/// How much of each pixel is covered, 0 to 1.
pub struct Coverage {
    width: i32,
    height: i32,
    pub values: Vec<f64>,
}

impl Coverage {
    pub fn new(width: i32, height: i32, values: Vec<f64>) -> Coverage {
        assert_eq!(values.len(), (width * height) as usize, "one value for each pixel");
        Coverage { width, height, values }
    }

    fn at(&self, x: i32, y: i32) -> f64 {
        if (0..self.width).contains(&x) && (0..self.height).contains(&y) { self.values[(y * self.width + x) as usize] } else { 0.0 }
    }

    /// Coverage at a point, interpolated between the centres of the four pixels around it.
    pub fn sample(&self, x: f64, y: f64) -> f64 {
        let (fx, fy) = (x - 0.5, y - 0.5);
        let (x0, y0) = (fx.floor(), fy.floor());
        let (u, v) = (fx - x0, fy - y0);
        let (x0, y0) = (x0 as i32, y0 as i32);
        self.at(x0, y0) * (1.0 - u) * (1.0 - v) + self.at(x0 + 1, y0) * u * (1.0 - v) + self.at(x0, y0 + 1) * (1.0 - u) * v + self.at(x0 + 1, y0 + 1) * u * v
    }

    /// The width of what is drawn across the line through `p` along the unit vector `n`: the coverage summed along the
    /// line, `reach` pixels either side. Exact for an edge of any angle, since an edge's coverage is its distance.
    pub fn width_across(&self, p: (f64, f64), n: (f64, f64), reach: f64) -> f64 {
        const STEP: f64 = 0.05;
        let steps = (reach / STEP) as i32;
        (-steps..=steps).map(|k| self.sample(p.0 + n.0 * f64::from(k) * STEP, p.1 + n.1 * f64::from(k) * STEP)).sum::<f64>() * STEP
    }
}
