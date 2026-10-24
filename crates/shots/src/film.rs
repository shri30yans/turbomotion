//! The compiled film description (`build/film.json`), as written by `timeline.py`.

use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Film {
    pub fps: u32,
    pub width: i32,
    pub height: i32,
    #[serde(rename = "durationInFrames")]
    pub duration_in_frames: u32,
    pub chapters: Vec<Chapter>,
    pub shots: Vec<Shot>,
    pub assets: HashMap<String, Asset>,
}

#[derive(Debug, Deserialize)]
pub struct Chapter {
    pub title: String,
    #[serde(rename = "startFrame")]
    pub start_frame: u32,
}

#[derive(Debug, Deserialize)]
pub struct Shot {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub args: Vec<String>,
    pub params: HashMap<String, String>,
    /// Shot length in seconds.
    pub duration: f32,
    #[serde(rename = "startFrame")]
    pub start_frame: u32,
    #[serde(rename = "durationInFrames")]
    pub duration_in_frames: u32,
    /// Number of cue marks the script places in the shot.
    pub cues: usize,
    /// Seconds into the shot at which each cued word is spoken.
    #[serde(rename = "cueTimes")]
    pub cue_times: Vec<f32>,
    /// Cue index by the name the script gave it (`@payoff`).
    #[serde(rename = "cueNames", default)]
    pub cue_names: HashMap<String, usize>,
}

#[derive(Debug, Deserialize)]
pub struct Asset {
    pub file: String,
    pub kind: String,
    pub depth: Option<String>,
}

impl Film {
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    /// The shot on screen at an absolute frame.
    pub fn shot_at(&self, frame: u32) -> Option<&Shot> {
        self.shots
            .iter()
            .find(|s| (s.start_frame..s.start_frame + s.duration_in_frames).contains(&frame))
    }
}

impl Shot {
    /// Time of cue `index`, with the same fallbacks as `cueAt` in `core.ts`:
    /// an explicit default, else extrapolated past the last cue.
    pub fn cue_at(&self, index: usize, fallback: Option<f32>) -> f32 {
        if let Some(t) = self.cue_times.get(index) {
            return t.max(0.05);
        }
        fallback.unwrap_or_else(|| {
            let last = self.cue_times.last().copied().unwrap_or(0.3);
            let missing = (index + 1 - self.cue_times.len()) as f32;
            (self.duration - 0.6).min(last + 0.9 * missing)
        })
    }

    /// The frame a review still of this shot shows: `at` seconds in, else settled 1.3 s after
    /// the last cue (or most of the way through a shot without cues). Never the very last moment.
    ///
    /// The arithmetic runs in f64 on the film's millisecond values, as the old JavaScript did:
    /// near the end of a shot a single-precision sum can land on the other side of a frame.
    pub fn still_frame(&self, at: Option<f32>, fps: u32) -> u32 {
        let milliseconds = |s: f32| (f64::from(s) * 1000.0).round() / 1000.0;
        let duration = milliseconds(self.duration);
        let settled = match self.cue_times.iter().copied().map(milliseconds).reduce(f64::max) {
            Some(last) => last + 1.3,
            None => duration * 0.72,
        };
        let seconds = at.map_or_else(|| settled.max(duration * 0.45), f64::from).min(duration - 0.1);
        self.start_frame + (seconds * f64::from(fps)).floor() as u32
    }

    pub fn param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(String::as_str).filter(|v| !v.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(duration: f32, cue_times: Vec<f32>) -> Shot {
        Shot {
            id: "s001".into(),
            kind: "photo".into(),
            args: vec![],
            params: HashMap::new(),
            duration,
            start_frame: 100,
            duration_in_frames: (duration * 30.0).round() as u32,
            cues: cue_times.len(),
            cue_times,
            cue_names: HashMap::new(),
        }
    }

    #[test]
    fn stills_settle_after_the_last_cue_but_never_on_the_final_instant() {
        // 8.374 + 1.3 is past the end of a 9.2 s shot: the still is pinned 0.1 s before it,
        // which is frame 273 (9.1 s) in exact arithmetic although f32 sums land on 272.
        assert_eq!(shot(9.2, vec![1.054, 8.374]).still_frame(None, 30), 100 + 273);
        assert_eq!(shot(4.0, vec![1.0]).still_frame(None, 30), 100 + 69);
        assert_eq!(shot(2.1, vec![]).still_frame(Some(1.0), 30), 100 + 30);
    }
}
