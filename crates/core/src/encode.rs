//! Raw BGRA frames piped into ffmpeg's hardware H.264 encoder.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
};

use anyhow::{bail, Context, Result};

/// How frames become H.264.
pub enum Codec {
    /// Apple's media engine via VideoToolbox: encoding costs almost no CPU. Quality is set by bitrate.
    Hardware { bitrate: String },
    /// libx264 on the CPU: quality is set by CRF, speed by the preset.
    Software { crf: u32, preset: String },
}

impl Codec {
    fn args(&self) -> Vec<String> {
        match self {
            Self::Hardware { bitrate } => ["-c:v", "h264_videotoolbox", "-allow_sw", "0", "-b:v", bitrate].map(String::from).to_vec(),
            Self::Software { crf, preset } => {
                ["-c:v", "libx264", "-crf", &crf.to_string(), "-preset", preset].map(String::from).to_vec()
            }
        }
    }
}

/// Frame geometry and rate of the video being encoded.
#[derive(Debug, Clone, Copy)]
pub struct VideoFormat {
    pub width: i32,
    pub height: i32,
    pub fps: u32,
}

pub struct Encoder {
    child: Child,
    stdin: ChildStdin,
}

impl Encoder {
    pub fn spawn(out: &Path, format: VideoFormat, codec: &Codec) -> Result<Self> {
        let VideoFormat { width, height, fps } = format;
        let mut child = Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "bgra"])
            .args(["-s", &format!("{width}x{height}"), "-r", &fps.to_string(), "-i", "-"])
            .args(codec.args())
            .args(["-pix_fmt", "yuv420p", "-an"])
            .arg(out)
            .stdin(Stdio::piped())
            .spawn()
            .context("starting ffmpeg")?;
        let stdin = child.stdin.take().context("ffmpeg stdin")?;
        Ok(Self { child, stdin })
    }

    pub fn write(&mut self, frame: &[u8]) -> Result<()> {
        self.stdin.write_all(frame).context("writing a frame to ffmpeg")
    }

    pub fn finish(self) -> Result<()> {
        let Self { mut child, stdin } = self;
        drop(stdin);
        let status = child.wait().context("waiting for ffmpeg")?;
        if !status.success() {
            bail!("ffmpeg exited with {status}");
        }
        Ok(())
    }
}

/// Join segments end to end without re-encoding.
pub fn concat(parts: &[PathBuf], out: &Path) -> Result<()> {
    // ffmpeg resolves list entries relative to the list file, so hand it absolute paths.
    let list = parts.iter().try_fold(String::new(), |mut list, part| -> Result<String> {
        let part = std::path::absolute(part).with_context(|| format!("resolving {}", part.display()))?;
        list.push_str(&format!("file '{}'\n", part.display()));
        Ok(list)
    })?;
    let list_path = out.with_extension("concat.txt");
    fs::write(&list_path, list).context("writing the concat list")?;
    let status = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "concat", "-safe", "0", "-i"])
        .arg(&list_path)
        .args(["-c", "copy"])
        .arg(out)
        .status()
        .context("starting ffmpeg concat")?;
    fs::remove_file(&list_path).context("removing the concat list")?;
    if !status.success() {
        bail!("ffmpeg concat exited with {status}");
    }
    Ok(())
}
