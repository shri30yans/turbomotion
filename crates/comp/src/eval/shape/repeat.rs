//! A shape item's repeater at one instant: the step between copies and, with a `palette`, the colour each copy takes.

use anyhow::Result;
use serde::Serialize;

use crate::{
    eval::{
        palette::{resolve, Cycled},
        salt, Evaluator, LayerEnv,
    },
    plan::{CompPlan, LayerPlan},
    schema::{palette::Target, Repeat},
};

#[derive(Serialize, Clone)]
pub struct RepeatState {
    pub copies: u32,
    pub offset: [f64; 2],
    pub rotation: f64,
    pub scale: f64,
    pub opacity: f64,
    /// The palette the copies take their colours from.
    pub tint: Option<Tint>,
}

/// The colours of the copies and which paints take them.
#[derive(Serialize, Clone)]
pub struct Tint {
    pub paint: Target,
    pub colours: Cycled,
}

/// `r` at time `t` for item `k` of layer `l`.
pub fn state(ev: &Evaluator, c: &CompPlan, l: &LayerPlan, r: &Repeat, k: usize, t: f64) -> Result<RepeatState> {
    let env = |salt: u32| LayerEnv::new(ev, c, l.index, t, 0).with_salt(salt);
    let owner = salt::named(salt::SHAPE_REPEAT_PALETTE, &k.to_string());
    let tint = r.palette.as_ref().map(|p| Ok::<_, anyhow::Error>(Tint { paint: p.paint, colours: resolve(p, t - l.start, &env, owner, c.color.working)? })).transpose()?;
    Ok(RepeatState { copies: r.copies.max(1), offset: r.offset.unwrap_or([0.0, 0.0]), rotation: r.rotation, scale: r.scale, opacity: r.opacity, tint })
}
