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
                "ref" | "vel" => {
                    if self.depth >= 6 {
                        bail!("{name}() chains are too deep (a loop of layers following each other?)");
                    }
                    let id = args.first().ok_or_else(|| anyhow!("{name}() needs a layer id"))?.str()?;
                    let prop = args.get(1).ok_or_else(|| anyhow!("{name}(\"id\", \"prop\") needs a property"))?.str()?;
                    let l = self.comp.layers.iter().find(|l| l.def.common.id.as_deref() == Some(id)).ok_or_else(|| anyhow!("{name}(): no layer with id {id:?}"))?;
                    let at = |t: f64| -> Result<f64> {
                        let p = self.ev.local_props(self.comp, l, t, self.depth + 1)?;
                        Ok(match prop {
                            "x" => p.pos[0],
                            "y" => p.pos[1],
                            "z" => p.pos[2],
                            "rotation" => p.pose.rotation[2],
                            "rotation_x" => p.pose.rotation[0],
                            "rotation_y" => p.pose.rotation[1],
                            "orientation_x" => p.pose.orientation.to_orientation()[0],
                            "orientation_y" => p.pose.orientation.to_orientation()[1],
                            "orientation_z" => p.pose.orientation.to_orientation()[2],
                            "opacity" => p.opacity,
                            "scale" | "sx" => p.scale[0],
                            "sy" => p.scale[1],
                            other => bail!("{name}(): unknown property {other:?}; use x, y, z, rotation, rotation_x, rotation_y, orientation_x, orientation_y, orientation_z, opacity, scale, sx, sy"),
                        })
                    };
                    if name == "ref" {
                        at(self.t_comp)?
                    } else {
                        // vel("knob", "x"): units per second, by central difference over one frame.
                        let h = 0.5 / f64::from(self.ev.plan.fps);
                        (at(self.t_comp + h)? - at((self.t_comp - h).max(0.0))?) / (self.t_comp + h - (self.t_comp - h).max(0.0))
                    }
                }
                // amp("track"): loudness 0..1 of an audio or video layer at this moment.
                "amp" => {
                    let id = args.first().ok_or_else(|| anyhow!("amp() needs a layer id"))?.str()?;
                    let (sig, t) = self.ev.signal(id, self.comp, self.t_comp)?;
                    sig.amp(t)
                }
                // onset("track"): seconds since the latest hard hit (kick, clap) in the track.
                "onset" => {
                    let id = args.first().ok_or_else(|| anyhow!("onset() needs a layer id"))?.str()?;
                    let (sig, t) = self.ev.signal(id, self.comp, self.t_comp)?;
                    sig.since_onset(t)
                }
                _ => return Err(anyhow!("unknown function {name}()")),
            })
        };
        if ["beat", "bar", "beat_index", "marker", "ref", "vel", "amp", "onset"].contains(&name) { Some(run()) } else { None }
    }

    fn salt(&self) -> u32 {
        (self.layer as u32).wrapping_mul(7919).wrapping_add(self.salt.wrapping_mul(104_729))
    }
}

#[cfg(test)]
mod tests {
    use crate::plan::{Plan, ROOT};
    use serde_json::json;

    #[test]
    fn vel_reads_another_layers_speed_in_units_per_second() {
        let plan = Plan::from_value(
            json!({"dur": 2, "layers": [
                {"type": "null", "id": "a", "pos": [{"from": 0, "to": 100, "dur": 2, "ease": "none"}, 500]},
                {"type": "null", "id": "b", "pos": [0, 0], "scale": "1 + vel(\"a\", \"x\") / 1000"}]}),
            std::path::Path::new("."),
        )
        .unwrap();
        let ev = crate::eval::Evaluator::new(&plan);
        let c = &plan.comps[ROOT];
        let p = ev.local_props(c, &c.layers[1], 1.0, 0).unwrap();
        assert!((p.scale[0] - 1.05).abs() < 1e-3, "50 px/s is 0.05 of 1000: {}", p.scale[0]);
    }
}
