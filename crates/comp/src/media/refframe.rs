//! A reference video's frames for `comp sheet --ref`: one still per requested time, optionally cropped (for
//! letterboxed sources), decoded with `ffmpeg` into PNG bytes.

use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{bail, ensure, Context, Result};

/// `x,y,w,h` in the reference video's own pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Crop {
    pub fn parse(s: &str) -> Result<Crop> {
        let n: Vec<u32> = s.split(',').map(|p| p.trim().parse::<u32>()).collect::<Result<_, _>>().with_context(|| format!("--crop {s:?}: write four whole numbers x,y,w,h (pixels of the reference), like 0,280,1080,1360"))?;
        ensure!(n.len() == 4, "--crop {s:?}: write four numbers x,y,w,h, found {}", n.len());
        ensure!(n[2] > 0 && n[3] > 0, "--crop {s:?}: the width and height must be above 0");
        Ok(Crop { x: n[0], y: n[1], w: n[2], h: n[3] })
    }

    fn filter(self) -> String {
        format!("crop={}:{}:{}:{}", self.w, self.h, self.x, self.y)
    }
}

pub struct RefVideo {
    pub path: PathBuf,
    pub crop: Option<Crop>,
    /// Added to every film time to find the reference moment.
    pub offset: f64,
}

impl RefVideo {
    /// The PNG of the reference at film time `t` (plus the offset).
    pub fn frame_png(&self, t: f64) -> Result<Vec<u8>> {
        let at = (t + self.offset).max(0.0);
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-v", "error", "-ss", &format!("{at:.4}"), "-i"]).arg(&self.path).args(["-an", "-sn", "-frames:v", "1"]);
        if let Some(c) = self.crop {
            cmd.args(["-vf", &c.filter()]);
        }
        cmd.args(["-f", "image2pipe", "-vcodec", "png", "-"]).stdout(Stdio::piped()).stderr(Stdio::piped());
        let out = cmd.output().context("starting ffmpeg to read the reference video")?;
        if !out.status.success() {
            bail!("ffmpeg could not read {} at {at:.2}s: {}", self.path.display(), String::from_utf8_lossy(&out.stderr).trim());
        }
        ensure!(!out.stdout.is_empty(), "the reference {} has no frame at {at:.2}s (it ends sooner, or --crop lies outside the picture)", self.path.display());
        Ok(out.stdout)
    }

    pub fn exists(path: &Path) -> Result<()> {
        ensure!(path.is_file(), "--ref {}: no such file", path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_parses_and_explains() {
        assert_eq!(Crop::parse("0, 280,1080,1360").unwrap(), Crop { x: 0, y: 280, w: 1080, h: 1360 });
        assert_eq!(Crop { x: 1, y: 2, w: 3, h: 4 }.filter(), "crop=3:4:1:2");
        assert!(format!("{:#}", Crop::parse("1,2,3").unwrap_err()).contains("x,y,w,h"));
        assert!(Crop::parse("1,2,0,4").is_err());
        assert!(Crop::parse("a,b,c,d").is_err());
    }

    #[test]
    fn a_missing_reference_is_named() {
        let e = RefVideo::exists(Path::new("/no/such/ref.mp4")).unwrap_err().to_string();
        assert!(e.contains("/no/such/ref.mp4"), "{e}");
    }
}
