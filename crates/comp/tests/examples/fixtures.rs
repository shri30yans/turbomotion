//! Media files for examples that name one (`photo.jpg`, `clip.mp4`, `music.wav`): made once per test run with ffmpeg,
//! which the engine needs anyway to read media.

use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use serde_json::Value;

const MEDIA: [&str; 6] = [".jpg", ".png", ".mp4", ".wav", ".mp3", ".webp"];

/// The shared fixtures folder (`<target tmp>/example-media`).
pub fn folder() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("example-media-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    })
    .clone()
}

fn make(name: &str) {
    let path = folder().join(name);
    if path.exists() {
        return;
    }
    let args: Vec<&str> = match Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("") {
        // a photograph is 4K: a 1080p film can zoom into it 2x before the picture goes soft
        "jpg" | "png" | "webp" => vec!["-f", "lavfi", "-i", "testsrc2=size=3840x2160:rate=1", "-frames:v", "1"],
        "mp4" => vec!["-f", "lavfi", "-i", "testsrc2=size=640x360:rate=30", "-t", "3", "-pix_fmt", "yuv420p", "-c:v", "libx264"],
        _ => vec!["-f", "lavfi", "-i", "sine=frequency=220:duration=4"],
    };
    let out = Command::new("ffmpeg").args(["-v", "error", "-nostdin", "-y"]).args(&args).arg(&path).output().expect("ffmpeg (the engine needs it too)");
    assert!(out.status.success() && path.exists(), "making the fixture {name}: {}", String::from_utf8_lossy(&out.stderr));
}

/// Make every media file a film value names (any string ending in a media extension, as a bare file name).
pub fn ensure(film: &Value) {
    match film {
        Value::String(s) if MEDIA.iter().any(|e| s.to_ascii_lowercase().ends_with(e)) && !s.contains('/') => make(s),
        Value::Array(a) => a.iter().for_each(ensure),
        Value::Object(o) => o.values().for_each(ensure),
        _ => {}
    }
}
