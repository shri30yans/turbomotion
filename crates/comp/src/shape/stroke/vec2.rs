//! Two-component vectors for the stroke geometry, in `f64` (the sampler hands over Skia's `f32` once).

pub type V = [f64; 2];

pub fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1]]
}

pub fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1]]
}

pub fn scale(a: V, k: f64) -> V {
    [a[0] * k, a[1] * k]
}

pub fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

pub fn cross(a: V, b: V) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

pub fn length(a: V) -> f64 {
    a[0].hypot(a[1])
}

pub fn lerp(a: V, b: V, u: f64) -> V {
    add(a, scale(sub(b, a), u))
}

/// `a` turned by `angle` radians (the direction that takes the x axis toward the y axis).
pub fn turn(a: V, angle: f64) -> V {
    let (s, c) = angle.sin_cos();
    [a[0] * c - a[1] * s, a[0] * s + a[1] * c]
}
