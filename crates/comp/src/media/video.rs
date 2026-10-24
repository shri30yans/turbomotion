//! Video input: frames come from one `ffmpeg` process per source that decodes forward as time advances, so a render
//! reads the file once. A jump (a loop, a time remap, the first frame of a worker's segment) restarts ffmpeg at the
//! new position. Frames are addressed by source frame number, so the same time always gives the same picture.

use std::{
    cell::RefCell,
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, ChildStdout, Command, Stdio},
    rc::Rc,
};

use anyhow::{bail, Context, Result};
use skia_safe::{images, AlphaType, ColorType, Data, Image, ImageInfo, ISize};

use super::probe::VideoInfo;

struct Decoder {
    child: Child,
    out: ChildStdout,
    /// Source frame number of the next frame the pipe will deliver.
    next: i64,
}

struct Source {
    info: VideoInfo,
    /// Output size of the decoder (the source size, capped).
    size: (i32, i32),
    decoder: Option<Decoder>,
    last: Option<(i64, Rc<Image>)>,
}

#[derive(Default)]
pub struct Videos {
    sources: RefCell<HashMap<PathBuf, Source>>,
}

impl Drop for Decoder {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Videos {
    /// The frame of `path` showing at `src_time` seconds.
    pub fn frame(&self, path: &Path, info: &VideoInfo, src_time: f64, max_width: i32) -> Result<Rc<Image>> {
        let mut map = self.sources.borrow_mut();
        let src = map.entry(path.to_path_buf()).or_insert_with(|| {
            let scale = if info.w as i32 > max_width { f64::from(max_width) / f64::from(info.w) } else { 1.0 };
            let size = (((f64::from(info.w) * scale).round() as i32).max(2) & !1, ((f64::from(info.h) * scale).round() as i32).max(2) & !1);
            Source { info: *info, size, decoder: None, last: None }
        });
        let n = ((src_time.max(0.0) * src.info.fps) + 1e-6).floor() as i64;
        if let Some((k, img)) = &src.last {
            if *k == n {
                return Ok(img.clone());
            }
        }
        let reachable = src.decoder.as_ref().is_some_and(|d| n >= d.next && n - d.next <= 24);
        if !reachable {
            src.decoder = Some(spawn(path, src, n)?);
        }
        let (w, h) = src.size;
        let bytes = (w * h * 4) as usize;
        let mut buf = vec![0u8; bytes];
        let d = src.decoder.as_mut().expect("spawned above");
        loop {
            if d.out.read_exact(&mut buf).is_err() {
                // Past the end of the file: hold the last frame we had.
                return match &src.last {
                    Some((_, img)) => Ok(img.clone()),
                    None => bail!("{} has no frame at {src_time:.2}s (the file is {:.2}s long)", path.display(), src.info.dur),
                };
            }
            let at = d.next;
            d.next += 1;
            if at == n {
                break;
            }
        }
        let info = ImageInfo::new(ISize::new(w, h), ColorType::RGBA8888, AlphaType::Unpremul, None);
        let img = images::raster_from_data(&info, Data::new_copy(&buf), (w * 4) as usize).context("uploading a video frame")?;
        let img = Rc::new(img);
        src.last = Some((n, img.clone()));
        Ok(img)
    }
}

fn spawn(path: &Path, src: &Source, n: i64) -> Result<Decoder> {
    let start = n as f64 / src.info.fps;
    let (w, h) = src.size;
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-v", "error", "-ss", &format!("{start:.6}"), "-i"]).arg(path).args(["-an", "-sn"]);
    if (w, h) != (src.info.w as i32, src.info.h as i32) {
        cmd.args(["-vf", &format!("scale={w}:{h}:flags=lanczos")]);
    }
    cmd.args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"]).stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = cmd.spawn().context("starting ffmpeg to decode video")?;
    let out = child.stdout.take().context("ffmpeg stdout")?;
    Ok(Decoder { child, out, next: n })
}
