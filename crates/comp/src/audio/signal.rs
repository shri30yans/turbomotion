//! Audio as a signal an expression can read: loudness over time, and the moments of hard onsets (kicks, claps).
//! Decoded once per file at low resolution; `amp("track")` and `onset("track")` read it.

use std::{path::Path, process::{Command, Stdio}};

use anyhow::{bail, Context, Result};

/// Envelope samples per second.
pub const RATE: f64 = 100.0;

pub struct Signal {
    /// Loudness 0..~1 (normalised to the file's 95th percentile), `RATE` values per second.
    pub env: Vec<f32>,
    /// Seconds of detected onsets, ascending.
    pub onsets: Vec<f64>,
}

impl Signal {
    pub fn load(path: &Path) -> Result<Signal> {
        let out = Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(path)
            .args(["-vn", "-ac", "1", "-ar", "8000", "-f", "f32le", "-"])
            .stderr(Stdio::piped())
            .output()
            .context("running ffmpeg to analyse audio")?;
        if !out.status.success() {
            bail!("could not decode {} for analysis: {}", path.display(), String::from_utf8_lossy(&out.stderr).trim());
        }
        let samples: Vec<f32> = out.stdout.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
        Ok(Signal::from_samples(&samples, 8000.0))
    }

    pub fn from_samples(samples: &[f32], sample_rate: f64) -> Signal {
        let win = (sample_rate / RATE).round().max(1.0) as usize;
        let mut env: Vec<f32> = samples.chunks(win).map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt()).collect();
        let mut sorted = env.clone();
        sorted.sort_by(f32::total_cmp);
        let p95 = sorted.get((sorted.len() as f64 * 0.95) as usize).copied().unwrap_or(1.0).max(1e-6);
        env.iter_mut().for_each(|v| *v = (*v / p95).min(1.5));
        // onsets: a rise of loudness well above the recent average, at a local peak of the rise, at least 120 ms apart
        let mut onsets = vec![];
        let (look, gap) = ((RATE * 0.15) as usize, (RATE * 0.12) as usize);
        let mut last: Option<usize> = None;
        for i in look..env.len().saturating_sub(1) {
            let avg = env[i - look..i].iter().sum::<f32>() / look as f32;
            let rise = env[i] - avg;
            if rise > 0.25 && env[i] >= env[i + 1] && env[i] >= env[i - 1] && last.is_none_or(|l| i > l + gap) {
                onsets.push(i as f64 / RATE);
                last = Some(i);
            }
        }
        Signal { env, onsets }
    }

    pub fn amp(&self, t: f64) -> f64 {
        let x = (t * RATE).max(0.0);
        let i = x.floor() as usize;
        match (self.env.get(i), self.env.get(i + 1)) {
            (Some(a), Some(b)) => f64::from(*a) + (f64::from(*b) - f64::from(*a)) * (x - x.floor()),
            (Some(a), None) => f64::from(*a),
            _ => 0.0,
        }
    }

    /// Seconds since the latest onset at or before `t` (a large number before the first).
    pub fn since_onset(&self, t: f64) -> f64 {
        match self.onsets.partition_point(|o| *o <= t) {
            0 => 99.0,
            k => t - self.onsets[k - 1],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_kicks_in_a_synthetic_signal() {
        // 4 seconds at 8 kHz: a decaying 80 Hz burst every second
        let sr = 8000.0;
        let samples: Vec<f32> = (0..(4.0 * sr) as usize)
            .map(|i| {
                let t = i as f64 / sr;
                let since = t.rem_euclid(1.0);
                ((2.0 * std::f64::consts::PI * 80.0 * t).sin() * (-since * 12.0).exp() * 0.8) as f32 + 0.002
            })
            .collect();
        let s = Signal::from_samples(&samples, sr);
        assert!(s.onsets.len() >= 3, "{:?}", s.onsets);
        assert!(s.onsets.iter().any(|o| (o - 1.0).abs() < 0.08), "{:?}", s.onsets);
        assert!(s.amp(1.01) > s.amp(1.6));
        assert!(s.since_onset(2.1) < 0.3);
    }
}
