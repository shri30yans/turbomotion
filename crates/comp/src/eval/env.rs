//! What an expression can see: the layer's clock, the composition's beat grid, and other layers' transforms.

use anyhow::{anyhow, bail, Result};

use super::Evaluator;
use crate::{
    anim::expr::{Arg, Env},
    plan::CompPlan,
};

/// The environment one property is evaluated in.
pub struct LayerEnv<'a> {
    ev: &'a Evaluator<'a>,
    comp: &'a CompPlan,
    layer: usize,
    t_comp: f64,
    salt: u32,
    depth: u32,
}

impl<'a> LayerEnv<'a> {
    pub fn new(ev: &'a Evaluator<'a>, comp: &'a CompPlan, layer: usize, t_comp: f64, depth: u32) -> Self {
        LayerEnv { ev, comp, layer, t_comp, salt: 0, depth }
    }

    /// Distinguish properties of one layer so `wiggle` on x and on y do not move in lockstep.
    pub fn with_salt(mut self, salt: u32) -> Self {
        self.salt = salt;
        self
    }

    fn beat_pos(&self) -> Result<f64> {
        let bpm = self.comp.bpm.ok_or_else(|| anyhow!("beat() and bar() need \"bpm\" on the composition"))?;
        Ok((self.t_comp - self.comp.beat_offset) * bpm / 60.0)
    }
}

impl Env for LayerEnv<'_> {
    fn var(&self, name: &str) -> Option<f64> {
        let l = self.comp.layers.get(self.layer)?;
        Some(match name {
            "time" => self.t_comp - l.start,
            "comp_time" => self.t_comp,
            "index" => self.layer as f64,
            "fps" => f64::from(self.ev.plan.fps),
            "frame" => (self.t_comp * f64::from(self.ev.plan.fps)).round(),
            "width" => f64::from(self.comp.width),
            "height" => f64::from(self.comp.height),
            "dur" => l.end - l.start,
            "progress" => ((self.t_comp - l.start) / (l.end - l.start).max(1e-9)).clamp(0.0, 1.0),
            "bpm" => self.comp.bpm.unwrap_or(0.0),
            _ => return None,
        })
    }

    fn call(&self, name: &str, args: &[Arg]) -> Option<Result<f64>> {
        let run = || -> Result<f64> {
            Ok(match name {
                // beat(): 0 on the beat, rising to 1 just before the next: `1 - beat()` is a kick pulse.
                "beat" => self.beat_pos()?.rem_euclid(1.0),
                "bar" => (self.beat_pos()? / 4.0).rem_euclid(1.0),
                "beat_index" => self.beat_pos()?.floor(),
                // marker("drop"): seconds since the marker (negative before it).
                "marker" => {
                    let want = args.first().ok_or_else(|| anyhow!("marker() needs a name"))?.str()?;
                    let m = self.comp.markers.iter().find(|m| m.name == want).ok_or_else(|| anyhow!("no marker called {want:?}"))?;
                    self.t_comp - m.t
                }
                // ref("title", "x"): another layer's transform property right now.
                "ref" => {
                    if self.depth >= 6 {
                        bail!("ref() chains are too deep (a loop of layers following each other?)");
                    }
                    let id = args.first().ok_or_else(|| anyhow!("ref() needs a layer id"))?.str()?;
                    let prop = args.get(1).ok_or_else(|| anyhow!("ref(\"id\", \"prop\") needs a property"))?.str()?;
                    let l = self.comp.layers.iter().find(|l| l.def.common.id.as_deref() == Some(id)).ok_or_else(|| anyhow!("ref(): no layer with id {id:?}"))?;
                    let p = self.ev.local_props(self.comp, l, self.t_comp, self.depth + 1)?;
                    match prop {
                        "x" => p.pos[0],
                        "y" => p.pos[1],
                        "z" => p.pos[2],
                        "rotation" => p.rotation,
                        "opacity" => p.opacity,
                        "scale" | "sx" => p.scale[0],
                        "sy" => p.scale[1],
                        other => bail!("ref(): unknown property {other:?}; use x, y, z, rotation, opacity, scale, sx, sy"),
                    }
                }
                _ => return Err(anyhow!("unknown function {name}()")),
            })
        };
        if ["beat", "bar", "beat_index", "marker", "ref"].contains(&name) { Some(run()) } else { None }
    }

    fn salt(&self) -> u32 {
        (self.layer as u32).wrapping_mul(7919).wrapping_add(self.salt.wrapping_mul(104_729))
    }
}
