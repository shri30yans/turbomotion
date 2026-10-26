//! Looking at a layer's media once, when the plan is made: a missing or unusable file is a plan error with a fix.

use std::path::Path;

use anyhow::{Context, Result};

use super::Media;
use crate::{media::probe, schema::Kind};

pub(super) fn probe_media(kind: &Kind, dir: &Path, spread: Option<(usize, u64)>) -> Result<Media> {
    Ok(match kind {
        Kind::Image(i) => {
            let path = probe::absolute(dir, &i.src);
            let info = probe::image(&path, &i.src)?;
            let depth = i.parallax.as_ref().map(|p| -> Result<_> {
                let dp = probe::absolute(dir, &p.depth);
                let di = probe::image(&dp, &p.depth).context("parallax depth map")?;
                Ok((dp, di))
            });
            Media::Image { path, info, depth: depth.transpose()? }
        }
        Kind::Video(v) => {
            let path = probe::absolute(dir, &v.src);
            let info = probe::video(&path, &v.src)?;
            Media::Video { path, info }
        }
        Kind::Audio(a) => {
            if let Some(cue) = a.src.strip_prefix("kit:") {
                let (order, seed) = spread.unwrap_or((0, 0));
                let pick = crate::media::kit::kit().pick(cue, order, seed).map_err(|e| e.context(format!("audio {:?}", a.src)))?;
                if !pick.path.exists() {
                    return Err(probe::missing(&pick.path, &a.src));
                }
                return Ok(Media::Audio { dur: probe::audio(&pick.path, &a.src)?, path: pick.path, rate: pick.rate, gain: pick.gain, kit: Some(pick.cue) });
            }
            let path = probe::absolute(dir, &a.src);
            let dur = probe::audio(&path, &a.src)?;
            Media::Audio { path, dur, rate: 1.0, gain: 1.0, kit: None }
        }
        _ => Media::None,
    })
}
