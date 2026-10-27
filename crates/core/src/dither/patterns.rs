//! The threshold patterns: each is a square tile in which every rank from 0 to 255 appears equally often, so the mean
//! threshold is exactly one half and the dither is zero-mean whatever the picture.

/// A tile of `side` x `side` ranks, row by row, each rank in `0..=255` used `side * side / 256` times.
pub struct Tile {
    pub side: usize,
    pub ranks: Vec<u8>,
}

/// The 16 x 16 Bayer matrix, built the usual way: each doubling places the previous one four times with offsets
/// 0, 2 / 3, 1 so that neighbours are as far apart in rank as they can be. A regular, cross-hatched pattern.
pub fn ordered() -> Tile {
    let mut m = vec![0usize];
    let mut side = 1;
    while side < 16 {
        let mut next = vec![0; side * side * 4];
        for y in 0..side {
            for x in 0..side {
                let v = 4 * m[y * side + x];
                for (dx, dy, add) in [(0, 0, 0), (1, 0, 2), (0, 1, 3), (1, 1, 1)] {
                    next[(y + dy * side) * side * 2 + x + dx * side] = v + add;
                }
            }
        }
        m = next;
        side *= 2;
    }
    Tile { side, ranks: m.iter().map(|&r| r as u8).collect() }
}

#[cfg(test)]
pub fn is_balanced(tile: &Tile) -> bool {
    let mut counts = [0usize; 256];
    tile.ranks.iter().for_each(|&r| counts[usize::from(r)] += 1);
    tile.ranks.len() == tile.side * tile.side && counts.iter().all(|&c| c * 256 == tile.ranks.len())
}
