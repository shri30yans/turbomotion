//! When each shot starts. Shots run back to back and a transition overlaps two shots by its length (Remotion's
//! TransitionSeries model); a shot with an explicit `start` keeps it and the shot before is extended to cover the
//! transition, which is how narration-locked cuts stay on their word.

use anyhow::{anyhow, ensure, Context, Result};
use serde_json::Value;

use super::{time, transition::parse_trans, Shot, Trans};

pub fn schedule(list: &[Value], default: Option<&Trans>) -> Result<Vec<Shot>> {
    let mut shots: Vec<Shot> = vec![];
    for (i, s) in list.iter().enumerate() {
        let def = s.as_object().ok_or_else(|| anyhow!("shot {i} must be an object"))?.clone();
        let id = def.get("id").and_then(Value::as_str).map_or_else(|| format!("shot{i}"), String::from);
        let ctx = || format!("shot {id:?}");
        let dur = time(def.get("dur").ok_or_else(|| anyhow!("shot {id:?} needs \"dur\""))?, "shot dur").with_context(ctx)?;
        ensure!(dur > 0.0, "shot {id:?}: dur must be positive");
        let into = match (i, def.get("in")) {
            (0, _) => None,
            (_, Some(v)) => Some(parse_trans(v, default).with_context(ctx)?),
            (_, None) => default.cloned(),
        };
        let d = into.as_ref().map_or(0.0, |t| t.dur);
        let start = match (def.get("start"), shots.last_mut()) {
            (Some(v), prev) => {
                let s = time(v, "shot start").with_context(ctx)?;
                if let Some(p) = prev {
                    // the transition starts on the locked cut, so the outgoing shot must last through it
                    p.dur = p.dur.max(s + d - p.start);
                }
                s
            }
            (None, Some(p)) => {
                ensure!(d <= p.dur.min(dur) + 1e-9, "shot {id:?}: its {d}s transition is longer than a shot it joins");
                p.end() - d
            }
            (None, None) => 0.0,
        };
        shots.push(Shot { id, start, dur, def, into });
    }
    Ok(shots)
}
