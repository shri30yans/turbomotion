//! The soundtrack: audio layers and the sound of video layers, from the root composition and every nested one,
//! mixed in Rust (per-sample gain from volume properties and fades) and muxed into the rendered video.

pub mod signal;
#[cfg(test)]
mod tests;

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{bail, ensure, Context, Result};

use crate::{
    eval::{Evaluator, LayerEnv},
    plan::{CompPlan, Media, Plan, ROOT},
    schema::Kind,
};

pub const SAMPLE_RATE: u32 = 48_000;

/// One piece of sound on the root timeline.
struct Clip {
    path: PathBuf,
    /// Root-timeline seconds where the clip begins and ends.
    from: f64,
    to: f64,
    src_in: f64,
    speed: f64,
    /// Fixed gain from a sound-kit pick (its spread variation); 1 for other files.
    gain: f64,
    fade_in: f64,
    fade_out: f64,
    /// `family/variant` when the sound came from the built-in kit.
    kit: Option<String>,
    /// Where in `comp` this clip's layer lives, to evaluate its volume.
    comp: String,
    layer: usize,
    /// Root time = `a * comp time + b`.
    map: (f64, f64),
}

fn collect(plan: &Plan, comp: &str, map: (f64, f64), window: (f64, f64), out: &mut Vec<Clip>) {
    let c = &plan.comps[comp];
    for l in &c.layers {
        let (from, to) = ((map.0 * l.start + map.1).max(window.0), (map.0 * l.end + map.1).min(window.1));
        if to <= from {
            continue;
        }
        match (&l.def.kind, &l.media) {
            (Kind::Audio(a), Media::Audio { path, rate, gain, kit, .. }) => {
                let speed = a.speed * rate;
                out.push(Clip { path: path.clone(), from, to, src_in: a.src_in + (from - (map.0 * l.start + map.1)) / map.0 * speed, speed, gain: *gain, kit: kit.clone(), fade_in: a.fade_in, fade_out: a.fade_out, comp: comp.to_string(), layer: l.index, map })
            }
            (Kind::Video(v), Media::Video { path, info }) if !v.mute && info.has_audio => {
                out.push(Clip { path: path.clone(), from, to, src_in: v.src_in + (from - (map.0 * l.start + map.1)) / map.0 * v.speed, speed: v.speed, gain: 1.0, kit: None, fade_in: 0.0, fade_out: 0.0, comp: comp.to_string(), layer: l.index, map })
            }
            (Kind::Precomp(p), _) => {
                // inner time = (comp time - start) * speed + offset  =>  comp time = (inner - offset)/speed + start
                let (a, b) = (map.0 / p.speed.max(1e-6), map.0 * (l.start - p.offset / p.speed.max(1e-6)) + map.1);
                collect(plan, &p.comp, (a, b), (from, to), out);
            }
            _ => {}
        }
    }
}

fn decode(clip: &Clip) -> Result<Vec<f32>> {
    let src_len = (clip.to - clip.from) * clip.speed + 0.05;
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-ss", &format!("{:.6}", clip.src_in.max(0.0)), "-t", &format!("{src_len:.6}"), "-i"])
        .arg(&clip.path)
        .args(["-vn", "-ac", "2", "-ar", &SAMPLE_RATE.to_string(), "-f", "f32le", "-"])
        .stderr(Stdio::piped())
        .output()
        .context("running ffmpeg to decode audio")?;
    ensure!(out.status.success(), "could not decode {}: {}", clip.path.display(), String::from_utf8_lossy(&out.stderr).trim());
    Ok(out.stdout.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect())
}

fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.9 { x } else { x.signum() * (0.9 + 0.1 * ((a - 0.9) / 0.1).tanh()) }
}

/// Mix every clip into one stereo WAV (32-bit float) of `dur` seconds. `None` when the composition has no sound.
pub fn mix(plan: &Plan, dur: f64, wav: &Path) -> Result<Option<()>> {
    mix_range(plan, 0.0..dur, wav)
}

/// One line per sound-kit clip on the timeline, in time order: when, which exact variant was chosen, its rate and gain.
pub fn sound_log(plan: &Plan) -> Vec<String> {
    let mut clips = vec![];
    collect(plan, ROOT, (1.0, 0.0), (0.0, plan.root().dur), &mut clips);
    clips.sort_by(|a, b| a.from.total_cmp(&b.from));
    clips.iter().filter_map(|c| Some(format!("{:7.3}s  {:<24} rate {:.3}  gain {:+.1} dB", c.from, c.kit.as_deref()?, c.speed, 20.0 * c.gain.log10()))).collect()
}

/// Mix a film-time interval into a zero-based WAV; preserve the source time, automation and fades.
pub fn mix_range(plan: &Plan, range: std::ops::Range<f64>, wav: &Path) -> Result<Option<()>> {
    ensure!(range.start.is_finite() && range.end.is_finite() && range.start >= 0.0 && range.end > range.start,
        "audio range must have finite seconds with 0 <= start < end");
    if range.start == 0.0 {
        for line in sound_log(plan) {
            eprintln!("sound {line}");
        }
    }
    let mut clips = vec![];
    collect(plan, ROOT, (1.0, 0.0), (0.0, plan.root().dur), &mut clips);
    clips.retain(|clip| clip.to > range.start && clip.from < range.end);
    let dur = range.end - range.start;
    if clips.is_empty() {
        return Ok(None);
    }
    let ev = Evaluator::new(plan);
    let sr = f64::from(SAMPLE_RATE);
    let total = (dur * sr).round() as usize;
    let mut mixbuf = vec![0f32; total * 2];
    for clip in &clips {
        let pcm = decode(clip).with_context(|| format!("audio {}", clip.path.display()))?;
        let comp: &CompPlan = &plan.comps[&clip.comp];
        let first = ((clip.from - range.start).max(0.0) * sr).round() as usize;
        let last = (((clip.to - range.start).max(0.0) * sr).round() as usize).min(total);
        let block = (SAMPLE_RATE / 100) as usize;
        let gain_at = |root_t: f64| -> Result<f64> {
            let comp_t = (root_t - clip.map.1) / clip.map.0;
            let vol = match &comp.layers[clip.layer].def.kind {
                Kind::Audio(a) => a.volume.as_ref(),
                Kind::Video(v) => v.volume.as_ref(),
                _ => None,
            };
            let mut g = match vol {
                Some(p) => p.0.at(comp_t - comp.layers[clip.layer].start, &LayerEnv::new(&ev, comp, clip.layer, comp_t, 0))?,
                None => 1.0,
            };
            let into = root_t - clip.from;
            let left = clip.to - root_t;
            if clip.fade_in > 0.0 {
                g *= (into / clip.fade_in).clamp(0.0, 1.0);
            }
            if clip.fade_out > 0.0 {
                g *= (left / clip.fade_out).clamp(0.0, 1.0);
            }
            Ok((g * clip.gain).max(0.0))
        };
        let mut i = first;
        while i < last {
            let end = (i + block).min(last);
            let (g0, g1) = (gain_at(range.start + i as f64 / sr)?, gain_at(range.start + end as f64 / sr)?);
            for k in i..end {
                let u = (k - i) as f64 / (end - i).max(1) as f64;
                let g = (g0 + (g1 - g0) * u) as f32;
                let pos = (range.start + k as f64 / sr - clip.from) * clip.speed * sr;
                let (p0, f) = (pos.floor() as usize, (pos - pos.floor()) as f32);
                for ch in 0..2 {
                    let a = pcm.get(p0 * 2 + ch).copied().unwrap_or(0.0);
                    let b = pcm.get((p0 + 1) * 2 + ch).copied().unwrap_or(a);
                    mixbuf[k * 2 + ch] += (a + (b - a) * f) * g;
                }
            }
            i = end;
        }
    }
    let mut bytes = Vec::with_capacity(44 + mixbuf.len() * 4);
    let data_len = (mixbuf.len() * 4) as u32;
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 8).to_le_bytes());
    bytes.extend_from_slice(&8u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for s in &mixbuf {
        bytes.extend_from_slice(&soft_clip(*s).to_le_bytes());
    }
    fs::write(wav, bytes).with_context(|| format!("writing {}", wav.display()))?;
    Ok(Some(()))
}

/// Put `wav` under the silent `video`, writing `out`.
pub fn mux(video: &Path, wav: &Path, out: &Path) -> Result<()> {
    let status = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(video)
        .arg("-i")
        .arg(wav)
        .args(["-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-shortest"])
        .arg(out)
        .status()
        .context("running ffmpeg to add the soundtrack")?;
    if !status.success() {
        bail!("ffmpeg could not add the soundtrack");
    }
    Ok(())
}
