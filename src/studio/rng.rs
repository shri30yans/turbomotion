//! numpy's legacy `RandomState` (MT19937), bit for bit: the Python engine seeds its dust, tiles, paper, grain and glitch
//! from it, and the native renderer must draw the same particles.

const N: usize = 624;
const M: usize = 397;

pub struct RandomState {
    key: [u32; N],
    pos: usize,
    gauss: Option<f64>,
}

impl RandomState {
    /// `np.random.RandomState(seed)` for a 32-bit integer seed (`init_genrand`).
    pub fn new(seed: u32) -> Self {
        let mut key = [0u32; N];
        let mut s = seed;
        for (i, k) in key.iter_mut().enumerate() {
            *k = s;
            s = 1812433253u32.wrapping_mul(s ^ (s >> 30)).wrapping_add(i as u32 + 1);
        }
        Self { key, pos: N, gauss: None }
    }

    fn refill(&mut self) {
        let k = &mut self.key;
        let twist = |u: u32, v: u32| (((u & 0x8000_0000) | (v & 0x7fff_ffff)) >> 1) ^ if v & 1 != 0 { 0x9908_b0df } else { 0 };
        for i in 0..N - M {
            k[i] = k[i + M] ^ twist(k[i], k[i + 1]);
        }
        for i in N - M..N - 1 {
            k[i] = k[i + M - N] ^ twist(k[i], k[i + 1]);
        }
        k[N - 1] = k[M - 1] ^ twist(k[N - 1], k[0]);
        self.pos = 0;
    }

    pub fn next_u32(&mut self) -> u32 {
        if self.pos >= N {
            self.refill();
        }
        let mut y = self.key[self.pos];
        self.pos += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// `rand()`: a double in [0, 1) with 53 random bits.
    pub fn rand(&mut self) -> f64 {
        let a = (self.next_u32() >> 5) as f64;
        let b = (self.next_u32() >> 6) as f64;
        (a * 67108864.0 + b) / 9007199254740992.0
    }

    /// `randn()`: the legacy polar Box-Muller, which yields values in pairs.
    pub fn randn(&mut self) -> f64 {
        if let Some(g) = self.gauss.take() {
            return g;
        }
        loop {
            let x1 = 2.0 * self.rand() - 1.0;
            let x2 = 2.0 * self.rand() - 1.0;
            let r2 = x1 * x1 + x2 * x2;
            if r2 < 1.0 && r2 != 0.0 {
                let f = (-2.0 * r2.ln() / r2).sqrt();
                self.gauss = Some(f * x1);
                return f * x2;
            }
        }
    }

    /// `randint(lo, hi)` for small ranges: masked rejection on 32-bit draws.
    pub fn randint(&mut self, lo: i64, hi: i64) -> i64 {
        let max = (hi - lo - 1) as u32;
        if max == 0 {
            return lo;
        }
        let mask = u32::MAX >> max.leading_zeros();
        loop {
            let v = self.next_u32() & mask;
            if v <= max {
                return lo + i64::from(v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_numpy() {
        let mut r = RandomState::new(3);
        assert_eq!(r.rand(), 0.5507979025745755);
        assert_eq!(r.rand(), 0.7081478226181048);
        let mut r = RandomState::new(3);
        assert_eq!([r.randn(), r.randn(), r.randn()], [1.7886284734303186, 0.43650985051198943, 0.09649746807200862]);
        let mut r = RandomState::new(7);
        assert_eq!([r.randint(0, 1060), r.randint(0, 1060), r.randint(0, 1060), r.randint(6, 90), r.randint(6, 90)], [175, 537, 502, 73, 89]);
        let mut r = RandomState::new(11);
        let g: Vec<f32> = (0..6).map(|_| r.randn() as f32).collect();
        let want = [1.7494547, -0.286073, -0.48456514, -2.6533186, -0.00828463, -0.31963137];
        // libm's `ln` can differ from numpy's in the last bit of the double, so compare to float32 precision
        assert!(g.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-6), "{g:?}");
    }
}
