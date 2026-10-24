//! Looking at media files without decoding them: size, orientation, frame rate, duration, sound. Done once at plan
//! time so a missing or unusable file is a plan error with a fix, not a surprise at frame 900.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use skia_safe::{codec::Codec, Data};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageInfo {
    /// Pixel size after applying the file's EXIF orientation.
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VideoInfo {
    pub w: u32,
    pub h: u32,
    pub fps: f64,
    pub dur: f64,
    pub has_audio: bool,
}

/// Why a path cannot be used, with the files that are there.
pub fn missing(path: &Path, as_written: &str) -> anyhow::Error {
    let dir = path.parent().unwrap_or(Path::new("."));
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let mut near: Vec<String> = fs::read_dir(dir)
        .map(|rd| rd.filter_map(Result::ok).map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.to_lowercase().ends_with(&ext) && !ext.is_empty()).take(8).collect())
        .unwrap_or_default();
    near.sort();
    anyhow!(
        "{as_written:?} not found at {}{}",
        path.display(),
        if near.is_empty() { format!(" ({} does not exist or has no .{ext} files)", dir.display()) } else { format!("; files like it there: {}", near.join(", ")) }
    )
}

pub fn image(path: &Path, as_written: &str) -> Result<ImageInfo> {
    if !path.exists() {
        return Err(missing(path, as_written));
    }
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let codec = Codec::from_data(Data::new_copy(&bytes)).ok_or_else(|| anyhow!("{as_written:?} is not an image Skia can decode (PNG, JPEG, WebP, GIF, BMP and HEIF are supported)"))?;
    let d = codec.dimensions();
    let (w, h) = if codec.origin().swaps_width_height() { (d.height, d.width) } else { (d.width, d.height) };
    Ok(ImageInfo { w: w as u32, h: h as u32 })
}

fn ffprobe(path: &Path, as_written: &str) -> Result<Value> {
    if !path.exists() {
        return Err(missing(path, as_written));
    }
    let out = Command::new("ffprobe").args(["-v", "error", "-print_format", "json", "-show_streams", "-show_format"]).arg(path).output().context("running ffprobe (is ffmpeg installed?)")?;
    if !out.status.success() {
        bail!("{as_written:?} is not readable media: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    serde_json::from_slice(&out.stdout).context("reading ffprobe's answer")
}

fn rate(s: &str) -> f64 {
    s.split_once('/').and_then(|(a, b)| Some(a.parse::<f64>().ok()? / b.parse::<f64>().ok()?)).or_else(|| s.parse().ok()).unwrap_or(0.0)
}

pub fn video(path: &Path, as_written: &str) -> Result<VideoInfo> {
    let j = ffprobe(path, as_written)?;
    let streams = j["streams"].as_array().cloned().unwrap_or_default();
    let v = streams.iter().find(|s| s["codec_type"] == "video").ok_or_else(|| anyhow!("{as_written:?} has no video stream"))?;
    let (mut w, mut h) = (v["width"].as_u64().unwrap_or(0) as u32, v["height"].as_u64().unwrap_or(0) as u32);
    let rotation = v["side_data_list"].as_array().and_then(|l| l.iter().find_map(|d| d["rotation"].as_f64())).or_else(|| v["tags"]["rotate"].as_str().and_then(|r| r.parse().ok())).unwrap_or(0.0);
    if (rotation.abs() - 90.0).abs() < 1.0 || (rotation.abs() - 270.0).abs() < 1.0 {
        std::mem::swap(&mut w, &mut h);
    }
    let fps = rate(v["avg_frame_rate"].as_str().unwrap_or("0")).max(rate(v["r_frame_rate"].as_str().unwrap_or("0")).min(1000.0));
    let dur = v["duration"].as_str().and_then(|d| d.parse().ok()).or_else(|| j["format"]["duration"].as_str().and_then(|d| d.parse().ok())).unwrap_or(0.0);
    Ok(VideoInfo { w, h, fps: if fps > 0.0 { fps } else { 30.0 }, dur, has_audio: streams.iter().any(|s| s["codec_type"] == "audio") })
}

/// Duration in seconds of an audio file.
pub fn audio(path: &Path, as_written: &str) -> Result<f64> {
    let j = ffprobe(path, as_written)?;
    anyhow::ensure!(j["streams"].as_array().is_some_and(|s| s.iter().any(|s| s["codec_type"] == "audio")), "{as_written:?} has no audio stream");
    j["format"]["duration"].as_str().and_then(|d| d.parse().ok()).ok_or_else(|| anyhow!("{as_written:?} has no duration"))
}

pub fn absolute(dir: &Path, rel: &str) -> PathBuf {
    let p = Path::new(rel);
    if p.is_absolute() { p.to_path_buf() } else { dir.join(p) }
}
