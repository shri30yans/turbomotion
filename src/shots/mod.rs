//! Shot types: each prepares its textures and text once, then draws any moment of the shot.
//! Every type appears once in `TABLE`, with the parameters it understands.

mod caption;
mod card;
mod compare;
mod diagram;
mod map;
mod painting;
mod photo;
mod stat;

use std::path::Path;

use anyhow::{bail, Result};
use skia_safe::{gpu::DirectContext, Canvas};

use crate::{
    film::{Film, Shot},
    photo::PhotoEffect,
    text::Fonts,
};

/// A point in a shot's timeline.
#[derive(Debug, Clone, Copy)]
pub struct Moment {
    pub seconds: f32,
    /// Frames since the shot began (Remotion's `useCurrentFrame` inside a sequence).
    pub frame: f32,
}

impl Moment {
    pub fn at(shot: &Shot, frame: u32, fps: u32) -> Self {
        let local = frame - shot.start_frame;
        Self { seconds: local as f32 / fps as f32, frame: local as f32 }
    }
}

/// What a scene needs while drawing.
pub struct Stage<'a> {
    pub film: &'a Film,
    pub fonts: &'a Fonts,
    pub photo: &'a PhotoEffect,
}

/// What a scene needs while preparing.
pub struct Loader<'a> {
    pub context: &'a mut DirectContext,
    pub fonts: &'a Fonts,
    pub film: &'a Film,
    /// The project's `public/`, which asset paths in the film are relative to.
    pub public: &'a Path,
    /// The asset directory (`fonts/`, `grain/`, `paper.jpg`).
    pub assets: &'a Path,
}

pub trait Scene {
    fn draw(&self, canvas: &Canvas, stage: &Stage, at: Moment) -> Result<()>;
}

struct Entry {
    kinds: &'static [&'static str],
    params: &'static [&'static str],
    /// Checks beyond the parameter names, run before any work starts.
    verify: fn(&Shot) -> Result<()>,
    prepare: fn(&mut Loader, &Shot) -> Result<Box<dyn Scene>>,
}

const TABLE: &[Entry] = &[
    Entry { kinds: &["photo", "illustration"], params: photo::PARAMS, verify: accept, prepare: photo::prepare },
    Entry { kinds: &["stat"], params: stat::PARAMS, verify: accept, prepare: stat::prepare },
    Entry { kinds: &["card"], params: card::PARAMS, verify: accept, prepare: card::prepare },
    Entry { kinds: &["painting"], params: painting::PARAMS, verify: accept, prepare: painting::prepare },
    Entry { kinds: &["compare"], params: compare::PARAMS, verify: accept, prepare: compare::prepare },
    Entry { kinds: &["map"], params: map::PARAMS, verify: accept, prepare: map::prepare },
    Entry { kinds: &["diagram"], params: diagram::DIAGRAM_PARAMS, verify: diagram::verify, prepare: diagram::prepare_diagram },
    Entry { kinds: &["plan"], params: diagram::PLAN_PARAMS, verify: accept, prepare: diagram::prepare_plan },
];

fn accept(_: &Shot) -> Result<()> {
    Ok(())
}

fn entry(shot: &Shot) -> Result<&'static Entry> {
    match TABLE.iter().find(|e| e.kinds.contains(&shot.kind.as_str())) {
        Some(entry) => Ok(entry),
        None => bail!("{}: shot type {:?} is not supported natively", shot.id, shot.kind),
    }
}

/// The words a diagram step puts on screen, with when each arrives (for `scenes.py`).
pub use diagram::reads as diagram_reads;

/// Reject a shot the renderer cannot draw faithfully, before any work starts.
pub fn check(shot: &Shot) -> Result<()> {
    let entry = entry(shot)?;
    let known = entry.params;
    let mut unsupported: Vec<&str> = shot.params.keys().map(String::as_str).filter(|k| !known.contains(k)).collect();
    unsupported.sort_unstable();
    if !unsupported.is_empty() {
        bail!("{}: parameters not supported natively: {}", shot.id, unsupported.join(", "));
    }
    (entry.verify)(shot)
}

pub fn prepare(loader: &mut Loader, shot: &Shot) -> Result<Box<dyn Scene>> {
    (entry(shot)?.prepare)(loader, shot)
}

/// A required shot parameter.
fn required<'a>(shot: &'a Shot, key: &str) -> Result<&'a str> {
    shot.param(key).ok_or_else(|| anyhow::anyhow!("{}: {} shot needs a {key:?} parameter", shot.id, shot.kind))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn shot(kind: &str, params: &[(&str, &str)]) -> Shot {
        Shot {
            id: "s001".into(),
            kind: kind.into(),
            args: vec!["a".into()],
            params: params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<HashMap<_, _>>(),
            duration: 1.0,
            start_frame: 0,
            duration_in_frames: 30,
            cues: 0,
            cue_times: vec![],
            cue_names: HashMap::new(),
        }
    }

    #[test]
    fn accepts_the_parameters_a_type_understands() {
        assert!(check(&shot("photo", &[("from", "0.5,0.5,1"), ("depth", "1")])).is_ok());
        assert!(check(&shot("stat", &[("value", "12"), ("unit", "m")])).is_ok());
    }

    #[test]
    fn rejects_unknown_parameters_and_types() {
        assert!(check(&shot("photo", &[("sparkle", "1")])).is_err());
        assert!(check(&shot("stat", &[("tag", "x")])).is_err());
        assert!(check(&shot("hologram", &[])).is_err());
    }

    #[test]
    fn moments_count_from_the_start_of_the_shot() {
        let mut s = shot("photo", &[]);
        s.start_frame = 60;
        let m = Moment::at(&s, 75, 30);
        assert_eq!((m.seconds, m.frame), (0.5, 15.0));
    }
}
