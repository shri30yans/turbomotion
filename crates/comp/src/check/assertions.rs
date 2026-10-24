//! Assertions: intent written down, checked on evaluated frames. A file of these sits beside the composition as
//! `comp.assert.json`:
//!
//! ```json
//! [
//!   {"visible_by": {"layer": "title", "t": 1.2}},
//!   {"stays_in_frame": {"layer": "title", "safe": 0.9}},
//!   {"before": {"first": "title", "then": "photo"}},
//!   {"moves": {"layer": "photo", "between": [1, 3]}},
//!   {"hidden_at": {"layer": "title", "t": 4.9}},
//!   {"no_overlap": {"a": "title", "b": "card"}},
//!   {"keeps_moving": {"window": 2}}
//! ]
//! ```

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use super::{frame_times, shows, Finding, Severity};
use crate::{
    eval::{Content, Evaluator, Frame},
    plan::{Plan, ROOT},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Assertion {
    /// The layer is showing by time `t` (at some frame up to and including it).
    VisibleBy { layer: String, t: f64 },
    HiddenAt { layer: String, t: f64 },
    /// While showing (between `from` and `to` if given), the layer's bounds stay inside the frame shrunk to `safe` (default 1).
    StaysInFrame { layer: String, #[serde(default = "one")] safe: f64, from: Option<f64>, to: Option<f64> },
    /// `first` appears strictly before `then`.
    Before { first: String, then: String },
    /// The layer's position, size or opacity differs between two moments.
    Moves { layer: String, between: [f64; 2] },
    /// The two layers' bounds never overlap while both show (between `from` and `to` if given).
    NoOverlap { a: String, b: String, from: Option<f64>, to: Option<f64> },
    /// Something on screen changes in every `window` seconds: no dead holds.
    KeepsMoving { window: f64 },
}

fn one() -> f64 {
    1.0
}

pub fn load(path: &std::path::Path) -> Result<Vec<Assertion>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| anyhow!("{}: {e}", path.display()))
}

struct Run<'a> {
    plan: &'a Plan,
    ev: Evaluator<'a>,
}

impl Run<'_> {
    fn index(&self, id: &str) -> Result<usize> {
        let ids: Vec<&str> = self.plan.root().layers.iter().filter_map(|l| l.def.common.id.as_deref()).collect();
        self.plan.root().layers.iter().find(|l| l.def.common.id.as_deref() == Some(id)).map(|l| l.index).ok_or_else(|| anyhow!("no layer with id {id:?}; ids are {}", ids.join(", ")))
    }

    fn at(&self, t: f64) -> Result<Frame> {
        self.ev.frame(ROOT, t)
    }

    fn first_shown(&self, layer: usize) -> Result<Option<f64>> {
        for t in frame_times(self.plan, 4000) {
            if shows(&self.at(t)?.layers[layer]) {
                return Ok(Some(t));
            }
        }
        Ok(None)
    }
}

/// A snapshot of what a viewer could see change: positions, sizes, opacity and what the content is doing.
fn signature(f: &Frame) -> Vec<f64> {
    let mut s = vec![];
    for l in f.layers.iter().filter(|l| shows(l)) {
        s.push(l.index as f64);
        s.push(l.opacity * 100.0);
        s.extend(l.origin);
        s.extend(l.bounds.unwrap_or_default());
        match &l.content {
            Content::Text(t) => {
                s.push(t.text.len() as f64);
                s.extend(t.glyphs.iter().map(|g| g.opacity * 10.0 + g.blur + f64::from(g.ch as u32) + g.m.0.iter().map(|v| f64::from(*v)).sum::<f64>()));
            }
            Content::Particles(p) => s.push(p.count as f64 + l.local_t),
            Content::Texture { .. } | Content::Video { .. } => s.push(l.local_t * 100.0),
            Content::Shape { items } => s.extend(items.iter().map(|i| i.opacity + i.trim.map_or(0.0, |t| t[0] + t[1]))),
            _ => {}
        }
    }
    s
}

fn differs(a: &[f64], b: &[f64]) -> bool {
    a.len() != b.len() || a.iter().zip(b).any(|(x, y)| (x - y).abs() > 0.5)
}

pub fn run(plan: &Plan, assertions: &[Assertion]) -> Result<Vec<Finding>> {
    let r = Run { plan, ev: Evaluator::new(plan) };
    let root = plan.root();
    let (w, h) = (f64::from(root.width), f64::from(root.height));
    let mut out = vec![];
    let mut fail = |rule: &'static str, layer: Option<&str>, t: Option<f64>, message: String, fix: &str| {
        out.push(Finding { severity: Severity::Error, rule, layer: layer.map(String::from), t, message, fix: fix.to_string() });
    };
    for a in assertions {
        match a {
            Assertion::VisibleBy { layer, t } => {
                let i = r.index(layer)?;
                let seen = frame_times(plan, 4000).into_iter().filter(|x| *x <= *t + 1e-9).any(|x| r.at(x).is_ok_and(|f| shows(&f.layers[i])));
                if !seen {
                    let first = r.first_shown(i)?;
                    fail("visible_by", Some(layer), Some(*t), format!("is not showing by {t:.2}s{}", first.map_or(" (it never shows)".to_string(), |f| format!(" (it first shows at {f:.2}s)"))), "start it earlier, or shorten its entrance");
                }
            }
            Assertion::HiddenAt { layer, t } => {
                let i = r.index(layer)?;
                if shows(&r.at(*t)?.layers[i]) {
                    fail("hidden_at", Some(layer), Some(*t), "is still showing".into(), "end it earlier or add an exit");
                }
            }
            Assertion::StaysInFrame { layer, safe, from, to } => {
                let i = r.index(layer)?;
                let (mx, my) = (w * (1.0 - safe) / 2.0, h * (1.0 - safe) / 2.0);
                for t in frame_times(plan, 1500) {
                    if from.is_some_and(|f| t < f) || to.is_some_and(|e| t > e) {
                        continue;
                    }
                    let st = &r.at(t)?.layers[i];
                    if let (true, Some(b)) = (shows(st), st.bounds) {
                        if b[0] < mx - 0.5 || b[1] < my - 0.5 || b[2] > w - mx + 0.5 || b[3] > h - my + 0.5 {
                            fail("stays_in_frame", Some(layer), Some(t), format!("bounds [{:.0}, {:.0}, {:.0}, {:.0}] leave the {:.0}% box [{mx:.0}, {my:.0}, {:.0}, {:.0}]", b[0], b[1], b[2], b[3], safe * 100.0, w - mx, h - my), "move or shrink it, or limit the assertion with from/to");
                            break;
                        }
                    }
                }
            }
            Assertion::Before { first, then } => {
                let (a, b) = (r.index(first)?, r.index(then)?);
                match (r.first_shown(a)?, r.first_shown(b)?) {
                    (Some(x), Some(y)) if x < y => {}
                    (x, y) => fail("before", Some(first), None, format!("{first:?} first shows at {}, {then:?} at {}", x.map_or("never".into(), |v| format!("{v:.2}s")), y.map_or("never".into(), |v| format!("{v:.2}s"))), "reorder their start times"),
                }
            }
            Assertion::Moves { layer, between } => {
                let i = r.index(layer)?;
                let (a, b) = (r.at(between[0])?, r.at(between[1])?);
                let sig = |f: &Frame| signature(&Frame { layers: vec![], ..shallow(f) }).into_iter().chain(signature_of(&f.layers[i])).collect::<Vec<_>>();
                if !differs(&sig(&a), &sig(&b)) {
                    fail("moves", Some(layer), Some(between[0]), format!("looks the same at {:.2}s and {:.2}s", between[0], between[1]), "animate pos, scale, opacity or a text animator across that range");
                }
            }
            Assertion::NoOverlap { a, b, from, to } => {
                let (ia, ib) = (r.index(a)?, r.index(b)?);
                for t in frame_times(plan, 1500) {
                    if from.is_some_and(|f| t < f) || to.is_some_and(|e| t > e) {
                        continue;
                    }
                    let f = r.at(t)?;
                    if let (true, true, Some(x), Some(y)) = (shows(&f.layers[ia]), shows(&f.layers[ib]), f.layers[ia].bounds, f.layers[ib].bounds) {
                        if x[0] < y[2] && y[0] < x[2] && x[1] < y[3] && y[1] < x[3] {
                            fail("no_overlap", Some(a), Some(t), format!("{a:?} and {b:?} overlap"), "move one of them or stagger their times");
                            break;
                        }
                    }
                }
            }
            Assertion::KeepsMoving { window } => {
                let times = frame_times(plan, 1500);
                let sigs: Vec<Vec<f64>> = times.iter().map(|t| r.at(*t).map(|f| signature(&f))).collect::<Result<_>>()?;
                let mut start = 0;
                for i in 1..times.len() {
                    if differs(&sigs[i], &sigs[i - 1]) {
                        start = i;
                    } else if times[i] - times[start] >= *window {
                        fail("keeps_moving", None, Some(times[start]), format!("nothing changes from {:.2}s for {window:.1}s", times[start]), "add motion (a slow push-in, a parallax drift, a looping texture) or cut sooner");
                        break;
                    }
                }
            }
        }
    }
    Ok(out)
}

fn shallow(f: &Frame) -> Frame {
    Frame { comp: f.comp.clone(), t: f.t, width: f.width, height: f.height, bg: f.bg, camera: None, lights: vec![], look: vec![], layers: vec![] }
}

fn signature_of(l: &crate::eval::LayerState) -> Vec<f64> {
    let mut s = vec![l.opacity * 100.0, f64::from(u8::from(shows(l)))];
    s.extend(l.origin);
    s.extend(l.bounds.unwrap_or_default());
    if let Content::Text(t) = &l.content {
        s.extend(t.glyphs.iter().map(|g| g.opacity * 10.0 + g.blur + f64::from(g.ch as u32) + g.m.0.iter().map(|v| f64::from(*v)).sum::<f64>()));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run_on(comp: serde_json::Value, a: serde_json::Value) -> Vec<&'static str> {
        let plan = Plan::from_value(comp, std::path::Path::new(".")).unwrap();
        let a: Vec<Assertion> = serde_json::from_value(a).unwrap();
        run(&plan, &a).unwrap().iter().map(|f| f.rule).collect()
    }

    fn comp() -> serde_json::Value {
        json!({"dur": 4, "layers": [
            {"type": "solid", "id": "bg", "color": "#102030"},
            {"type": "text", "id": "title", "text": "Hi", "size": 80, "color": "#fff", "pos": [960, 540], "align": "center", "start": 1.0, "enter": {"preset": "fade", "dur": 0.4}},
            {"type": "shape", "id": "dot", "pos": {"from": [100, 100], "to": [1800, 100], "at": 0.5, "dur": 2, "ease": "none"}, "items": [{"ellipse": [0, 0, 40, 40], "fill": "#f00"}]}
        ]})
    }

    #[test]
    fn assertions_pass_when_true_and_fail_with_a_reason_when_not() {
        let ok = json!([{"visible_by": {"layer": "title", "t": 1.6}}, {"hidden_at": {"layer": "title", "t": 0.5}}, {"before": {"first": "bg", "then": "title"}},
            {"moves": {"layer": "dot", "between": [1.0, 2.0]}}, {"stays_in_frame": {"layer": "title", "safe": 0.9}}, {"keeps_moving": {"window": 1.5}}]);
        assert_eq!(run_on(comp(), ok), Vec::<&str>::new());
        let bad = json!([{"visible_by": {"layer": "title", "t": 0.9}}, {"moves": {"layer": "title", "between": [2.0, 3.0]}}, {"before": {"first": "title", "then": "bg"}},
            {"stays_in_frame": {"layer": "dot", "safe": 0.5}}, {"no_overlap": {"a": "dot", "b": "bg"}}]);
        assert_eq!(run_on(comp(), bad), vec!["visible_by", "moves", "before", "stays_in_frame", "no_overlap"]);
    }

    #[test]
    fn unknown_ids_name_the_known_ones() {
        let plan = Plan::from_value(comp(), std::path::Path::new(".")).unwrap();
        let a: Vec<Assertion> = serde_json::from_value(json!([{"hidden_at": {"layer": "titel", "t": 1.0}}])).unwrap();
        let e = run(&plan, &a).err().unwrap().to_string();
        assert!(e.contains("title") && e.contains("dot"), "{e}");
    }
}
