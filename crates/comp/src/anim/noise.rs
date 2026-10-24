//! Deterministic noise for wiggles, particles and jitter: integer hashes, so every platform and every frame order
//! gives the same numbers. `noise1`/`noise2` return -1..1.

pub fn hash(seed: u32, a: i32, b: i32) -> u32 {
    let mut x = seed.wrapping_mul(0x9E37_79B1) ^ (a as u32).wrapping_mul(0x85EB_CA6B) ^ (b as u32).wrapping_mul(0xC2B2_AE35);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    x
}

/// Uniform 0..1 from a seed and two integers.
pub fn rand01(seed: u32, a: i32, b: i32) -> f64 {
    f64::from(hash(seed, a, b)) / f64::from(u32::MAX)
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lat(seed: u32, x: i32, y: i32) -> f64 {
    rand01(seed, x, y) * 2.0 - 1.0
}

pub fn noise1(seed: u32, x: f64) -> f64 {
    let i = x.floor();
    let f = fade(x - i);
    let (a, b) = (lat(seed, i as i32, 0), lat(seed, i as i32 + 1, 0));
    a + (b - a) * f
}

pub fn noise2(seed: u32, x: f64, y: f64) -> f64 {
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (fade(x - ix), fade(y - iy));
    let (ix, iy) = (ix as i32, iy as i32);
    let top = lat(seed, ix, iy) + (lat(seed, ix + 1, iy) - lat(seed, ix, iy)) * fx;
    let bot = lat(seed, ix, iy + 1) + (lat(seed, ix + 1, iy + 1) - lat(seed, ix, iy + 1)) * fx;
    top + (bot - top) * fy
}

/// Fractal noise: `octaves` layers, each twice the frequency and half the weight, normalised to about -1..1.
pub fn fbm1(seed: u32, x: f64, octaves: u32) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves.max(1) {
        sum += amp * noise1(seed.wrapping_add(o * 101), x * freq);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic_smooth_and_bounded() {
        assert_eq!(noise1(3, 1.37), noise1(3, 1.37));
        assert_ne!(noise1(3, 1.37), noise1(4, 1.37));
        let mut prev = noise1(1, 0.0);
        for i in 1..200 {
            let v = noise1(1, f64::from(i) * 0.01);
            assert!(v.abs() <= 1.0 && (v - prev).abs() < 0.1);
            prev = v;
        }
    }
}
