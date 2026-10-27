//! A blue-noise tile by void-and-cluster (Ulichney, 1993): the pixels are ranked so that those of any one rank are
//! spread as evenly as the tile allows, which puts the dither's error at high spatial frequencies where the eye is
//! least sensitive. Built on a torus (the tile repeats), from a fixed seed, with no floating-point reductions whose
//! order could differ, so the tile is the same on every run and machine.

use anyhow::{anyhow, Result};

use super::patterns::Tile;

const SIDE: usize = 64;
const PIXELS: usize = SIDE * SIDE;
/// Width of the Gaussian that measures how crowded a pixel's neighbourhood is, in pixels (the paper's 1.5).
const SIGMA: f64 = 1.5;
/// The Gaussian is cut at this many sigmas.
const REACH: isize = 5;
/// Pixels set in the starting pattern: a tenth of the tile, which the relaxation then moves into a stable spread.
const INITIAL: usize = PIXELS / 10;
/// The seed of the starting pattern.
const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// How crowded each pixel's neighbourhood is, as a Gaussian-weighted count of the set pixels around it.
struct Crowding {
    energy: Vec<f64>,
    kernel: Vec<(isize, isize, f64)>,
}

impl Crowding {
    fn new() -> Self {
        let mut kernel = Vec::new();
        for dy in -REACH..=REACH {
            for dx in -REACH..=REACH {
                kernel.push((dx, dy, (-((dx * dx + dy * dy) as f64) / (2.0 * SIGMA * SIGMA)).exp()));
            }
        }
        Self { energy: vec![0.0; PIXELS], kernel }
    }

    /// Set (`+1`) or clear (`-1`) pixel `i`.
    fn change(&mut self, i: usize, sign: f64) {
        let (x, y) = ((i % SIDE) as isize, (i / SIDE) as isize);
        for &(dx, dy, w) in &self.kernel {
            let at = ((y + dy).rem_euclid(SIDE as isize) as usize) * SIDE + (x + dx).rem_euclid(SIDE as isize) as usize;
            self.energy[at] += sign * w;
        }
    }

    /// The set pixel with the most crowded neighbourhood: the tightest cluster.
    fn tightest(&self, set: &[bool]) -> Result<usize> {
        self.extreme(set, true, |a, b| a > b)
    }

    /// The clear pixel with the emptiest neighbourhood: the largest void.
    fn largest_void(&self, set: &[bool]) -> Result<usize> {
        self.extreme(set, false, |a, b| a < b)
    }

    fn extreme(&self, set: &[bool], among: bool, better: fn(f64, f64) -> bool) -> Result<usize> {
        let mut best: Option<usize> = None;
        for i in (0..PIXELS).filter(|&i| set[i] == among) {
            if best.is_none_or(|b| better(self.energy[i], self.energy[b])) {
                best = Some(i);
            }
        }
        best.ok_or_else(|| anyhow!("blue-noise tile: no {} pixel left to rank", if among { "set" } else { "clear" }))
    }
}

/// A starting pattern of `INITIAL` set pixels: the first of a shuffle of all positions by a fixed xorshift.
fn starting_pattern() -> Vec<bool> {
    let mut order: Vec<usize> = (0..PIXELS).collect();
    let mut state = SEED;
    for i in (1..PIXELS).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        order.swap(i, (state % (i as u64 + 1)) as usize);
    }
    let mut set = vec![false; PIXELS];
    order[..INITIAL].iter().for_each(|&i| set[i] = true);
    set
}

fn crowding_of(set: &[bool]) -> Crowding {
    let mut crowding = Crowding::new();
    set.iter().enumerate().filter(|(_, &on)| on).for_each(|(i, _)| crowding.change(i, 1.0));
    crowding
}

/// The ranks of every pixel, `0..PIXELS`, with the pattern relaxed first so that no cluster remains.
fn rank_every_pixel() -> Result<Vec<usize>> {
    let mut set = starting_pattern();
    let mut crowding = crowding_of(&set);
    loop {
        let cluster = crowding.tightest(&set)?;
        set[cluster] = false;
        crowding.change(cluster, -1.0);
        let void = crowding.largest_void(&set)?;
        set[void] = true;
        crowding.change(void, 1.0);
        if void == cluster {
            break;
        }
    }
    let relaxed = set.clone();
    let mut ranks = vec![0; PIXELS];
    // the set pixels, tightest cluster last, take the ranks below the pattern's size
    let mut taking = relaxed.clone();
    for rank in (0..INITIAL).rev() {
        let cluster = crowding.tightest(&taking)?;
        taking[cluster] = false;
        crowding.change(cluster, -1.0);
        ranks[cluster] = rank;
    }
    // the rest are filled in largest void first
    let mut filling = relaxed;
    let mut crowding = crowding_of(&filling);
    for rank in INITIAL..PIXELS {
        let void = crowding.largest_void(&filling)?;
        filling[void] = true;
        crowding.change(void, 1.0);
        ranks[void] = rank;
    }
    Ok(ranks)
}

/// The 64 x 64 tile, its 4096 ranks folded onto 256 (16 pixels each).
pub fn tile() -> Result<Tile> {
    Ok(Tile { side: SIDE, ranks: rank_every_pixel()?.iter().map(|&r| (r * 256 / PIXELS) as u8).collect() })
}
