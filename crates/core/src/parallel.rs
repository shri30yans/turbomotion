//! Segment-parallel rendering: split the frames into contiguous ranges, give each to a worker that renders it to its
//! own mp4 with its own GPU context, then join the parts. A frame is a pure function of time, so the join is seamless.

use std::{fs, ops::Range, path::{Path, PathBuf}, thread};

use anyhow::{ensure, Context, Result};

use super::encode;

/// Split `frames` into at most `parts` contiguous, near-equal ranges.
pub fn split(frames: Range<u32>, parts: u32) -> Vec<Range<u32>> {
    let len = frames.end - frames.start;
    let parts = parts.clamp(1, len.max(1));
    (0..parts).map(|i| frames.start + len * i / parts..frames.start + len * (i + 1) / parts).collect()
}

/// Render `frames` into `out` with `workers` threads. `segment` renders one range into the mp4 it is given and returns
/// whatever the caller wants to report (timings); the results come back in frame order.
pub fn render<T: Send>(frames: Range<u32>, workers: u32, out: &Path, segment: impl Fn(Range<u32>, &Path) -> Result<T> + Sync) -> Result<Vec<T>> {
    ensure!(frames.start < frames.end, "frame range {frames:?} is empty");
    let ranges = split(frames, workers);
    // Parts share the output's container: an alpha ProRes film is a .mov, and mp4 cannot hold ProRes.
    let ext = out.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
    let parts: Vec<PathBuf> = (0..ranges.len()).map(|i| out.with_extension(format!("part{i}.{ext}"))).collect();
    let results = thread::scope(|scope| {
        let handles: Vec<_> = ranges.iter().zip(&parts).map(|(range, part)| scope.spawn(|| segment(range.clone(), part))).collect();
        handles.into_iter().map(|h| h.join().expect("render worker panicked")).collect::<Result<Vec<T>>>()
    })?;
    encode::concat(&parts, out)?;
    parts.iter().try_for_each(|p| fs::remove_file(p).with_context(|| format!("removing {}", p.display())))?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_covers_the_range_exactly() {
        assert_eq!(split(10..110, 3), vec![10..43, 43..76, 76..110]);
        assert_eq!(split(0..2, 8), vec![0..1, 1..2]);
    }
}
