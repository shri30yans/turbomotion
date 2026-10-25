//! Rules that need no knowledge of intent.

use super::{frame_times, shows, Finding, Severity};
use crate::{
    eval::{Content, Evaluator},
    plan::{Plan, ROOT},
    schema::Kind,
};

fn warn(rule: &'static str, layer: &str, t: Option<f64>, message: String, fix: &str) -> Finding {
    Finding { severity: Severity::Warn, rule, layer: Some(layer.to_string()), t, message, fix: fix.to_string() }
}

fn luminance(c: [f32; 4]) -> f64 {
    let lin = |v: f32| {
        let v = f64::from(v) / 255.0;
        if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

fn contrast(a: f64, b: f64) -> f64 {
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

pub fn run(plan: &Plan) -> anyhow::Result<Vec<Finding>> {
    let root = plan.root();
    let ev = Evaluator::new(plan);
    let (w, h) = (f64::from(root.width), f64::from(root.height));
    let times = frame_times(plan, 240);
    let frames = times.iter().map(|t| ev.frame(ROOT, *t)).collect::<anyhow::Result<Vec<_>>>()?;
    let mut out = vec![];

    let any_camera = root.layers.iter().any(|l| matches!(l.def.kind, Kind::Camera(_)));
    let any_light = root.layers.iter().any(|l| matches!(l.def.kind, Kind::Light(_)));
    let any_3d = root.layers.iter().any(|l| l.def.common.three_d);
    if any_3d && !any_camera {
        out.push(Finding { severity: Severity::Warn, rule: "3d-without-camera", layer: None, t: None, message: "layers use three_d but there is no camera layer, so depth has no effect".into(), fix: "add {\"type\": \"camera\"} (zoom, pos, poi) to the composition".into() });
    }
    if any_light && !any_3d {
        out.push(Finding { severity: Severity::Warn, rule: "light-without-3d", layer: None, t: None, message: "there are lights but no three_d layer to shade".into(), fix: "set \"three_d\": true on the layers the lights should shade".into() });
    }

    for (name, comp) in plan.comps.iter().filter(|(n, _)| !n.is_empty()) {
        let nested_3d = comp.layers.iter().any(|l| l.def.common.three_d);
        let nested_cam = comp.layers.iter().any(|l| matches!(l.def.kind, Kind::Camera(_)));
        if nested_3d && !nested_cam {
            out.push(Finding {
                severity: Severity::Warn,
                rule: "nested-3d-flatten",
                layer: Some(name.clone()),
                t: None,
                message: format!("comp {name:?} has 3D layers but no camera, so depth is flattened when this block is placed"),
                fix: "put {\"type\": \"camera\"} inside the block (see deck_3d); do not three_d the instance".into(),
            });
        }
    }
    for l in &root.layers {
        if !l.def.common.three_d {
            continue;
        }
        let crate::schema::Kind::Precomp(p) = &l.def.kind else { continue };
        let Some(inner) = plan.comps.get(&p.comp) else { continue };
        if inner.layers.iter().any(|x| matches!(x.def.kind, Kind::Camera(_))) {
            out.push(warn(
                "nested-3d-flatten",
                &l.label,
                None,
                format!("instance {:?} is three_d but its block already has a camera; nested precomps flatten 3D", l.label),
                "leave three_d off the instance; the camera lives inside the block",
            ));
        }
    }

    // anything a nested composition's own frame cuts off, and anything that paints past a plate
    for (name, comp) in plan.comps.iter().filter(|(n, _)| !n.is_empty()) {
        let (cw, ch) = (f64::from(comp.width), f64::from(comp.height));
        let n = 24;
        let backdrops: Vec<usize> = comp.layers.iter().filter(|l| l.def.common.layout.as_deref() == Some("backdrop")).map(|l| l.index).collect();
        'layers: for l in &comp.layers {
            let skip = matches!(l.def.kind, Kind::Camera(_) | Kind::Light(_) | Kind::Null | Kind::Audio(_) | Kind::Adjustment) || l.is_matte_source || l.def.common.hidden;
            if skip {
                continue;
            }
            let clipped = !l.masks.is_empty() || l.matte.is_some() || l.clip_to.is_some();
            for k in 0..n {
                let t = l.start + (l.end - l.start).max(0.0) * (f64::from(k) + 0.5) / f64::from(n);
                if t >= comp.dur {
                    continue;
                }
                let f = ev.frame(name, t)?;
                let st = &f.layers[l.index];
                if !(shows(st) && st.opacity > 0.9) {
                    continue;
                }
                if let Some(b) = st.bounds {
                    if matches!(l.def.kind, Kind::Text(_)) && (b[0] < -1.0 || b[1] < -1.0 || b[2] > cw + 1.0 || b[3] > ch + 1.0) {
                        let label = format!("{} in comp {name:?}", l.label);
                        out.push(warn("clipped-by-comp", &label, Some(t), format!("text reaches [{:.0}, {:.0}, {:.0}, {:.0}] but its composition is only {cw:.0}x{ch:.0}, so the edge is cut off", b[0], b[1], b[2], b[3]), "enlarge the nested composition's width/height, shorten the text or lower its size"));
                        continue 'layers;
                    }
                    if !clipped && l.def.common.layout.as_deref() != Some("backdrop") {
                        for &bi in &backdrops {
                            let Some(pb) = f.layers[bi].bounds else { continue };
                            if !shows(&f.layers[bi]) {
                                continue;
                            }
                            let pad = 8.0;
                            if b[0] < pb[0] - pad || b[1] < pb[1] - pad || b[2] > pb[2] + pad || b[3] > pb[3] + pad {
                                let label = format!("{} in comp {name:?}", l.label);
                                out.push(warn("paints-past-plate", &label, Some(t), format!("reaches [{:.0}, {:.0}, {:.0}, {:.0}] which is outside the plate [{:.0}, {:.0}, {:.0}, {:.0}]", b[0], b[1], b[2], b[3], pb[0], pb[1], pb[2], pb[3]), "set \"clip_to\": \"<plate id>\" (keeps the plate visible), or size the layer to the plate and mask it to the radius"));
                                continue 'layers;
                            }
                        }
                    }
                }
            }
        }
    }

    for l in &root.layers {
        let shown: Vec<usize> = (0..frames.len()).filter(|i| shows(&frames[*i].layers[l.index])).collect();
        let drawable = !matches!(l.def.kind, Kind::Camera(_) | Kind::Light(_) | Kind::Null | Kind::Audio(_) | Kind::Adjustment) && !l.is_matte_source && !l.def.common.hidden;
        if !drawable {
            continue;
        }
        if l.start >= root.dur {
            out.push(warn("starts-after-end", &l.label, None, format!("starts at {:.2}s but the composition ends at {:.2}s", l.start, root.dur), "move it earlier or lengthen the composition's dur"));
            continue;
        }
        if shown.is_empty() {
            out.push(warn("never-visible", &l.label, None, "is never visible (inactive, fully transparent, or behind the camera at every sampled frame)".into(), "check its start/dur, opacity and, for 3D layers, its z against the camera"));
            continue;
        }
        let on_screen = shown.iter().any(|i| frames[*i].layers[l.index].bounds.is_some_and(|b| b[2] > 0.0 && b[0] < w && b[3] > 0.0 && b[1] < h));
        if !on_screen && !matches!(l.def.kind, Kind::Particles(_)) {
            let t = times[shown[0]];
            out.push(warn("never-on-screen", &l.label, Some(t), "is visible but always entirely outside the frame".into(), "check pos, anchor and scale; `inspect --t <time> --layer <id>` shows its bounds"));
        }

        match &l.def.kind {
            Kind::Text(_) => {
                // safe area and legibility, judged where the text is fully shown
                let full: Vec<usize> = shown.iter().copied().filter(|i| frames[*i].layers[l.index].opacity > 0.9).collect();
                if let Some(&i) = full.iter().find(|i| {
                    frames[**i].layers[l.index].bounds.is_some_and(|b| b[0] < w * 0.05 - 0.5 || b[2] > w * 0.95 + 0.5 || b[1] < h * 0.05 - 0.5 || b[3] > h * 0.95 + 0.5)
                }) {
                    let b = frames[i].layers[l.index].bounds.unwrap_or_default();
                    out.push(warn("outside-safe-area", &l.label, Some(times[i]), format!("text reaches [{:.0}, {:.0}, {:.0}, {:.0}] which is outside the 90% safe area of {w:.0}x{h:.0}", b[0], b[1], b[2], b[3]), "keep text inside 5% margins: shrink size, add line breaks, or move pos"));
                }
                let is_ui = matches!(&l.def.kind, crate::schema::Kind::Text(tx) if tx.ui);
                if is_ui {
                } else if let Some(&i) = shown.iter().find(|i| matches!(&frames[**i].layers[l.index].content, Content::Text(t) if t.size < h * 0.02)) {
                    if let Content::Text(t) = &frames[i].layers[l.index].content {
                        out.push(Finding { severity: Severity::Error, rule: "text-too-small", layer: Some(l.label.clone()), t: Some(times[i]), message: format!("text is {:.0}px tall in a {h:.0}px frame (under 2% of the height)", t.size), fix: "use at least 2.5% of the frame height (27px at 1080p) so it reads on a phone".into() });
                    }
                } else if let Some(&i) = shown.iter().find(|i| matches!(&frames[**i].layers[l.index].content, Content::Text(t) if t.size < h * 0.024)) {
                    if let Content::Text(t) = &frames[i].layers[l.index].content {
                        out.push(warn("text-too-small", &l.label, Some(times[i]), format!("text is {:.0}px tall in a {h:.0}px frame", t.size), "use at least 2.5% of the frame height (27px at 1080p) so it reads on a phone"));
                    }
                }
                if let crate::schema::Kind::Text(tx) = &l.def.kind {
                    if let Some(role) = tx.role {
                        let scale = root.type_scale;
                        let (want, lo, hi) = match role {
                            crate::schema::Role::Display => (scale.display, scale.display * 0.8, scale.display * 1.25),
                            crate::schema::Role::Title => (scale.title, scale.title * 0.8, scale.title * 1.25),
                            crate::schema::Role::Body => (scale.body, scale.body * 0.8, scale.body * 1.25),
                        };
                        if let Some(&i) = shown.iter().find(|i| matches!(&frames[**i].layers[l.index].content, Content::Text(t) if t.size < lo - 0.5 || t.size > hi + 0.5)) {
                            if let Content::Text(t) = &frames[i].layers[l.index].content {
                                out.push(warn("role-size", &l.label, Some(times[i]), format!("{} role is {:.0}px; the type scale's {} is {:.0}px", role.as_str(), t.size, role.as_str(), want), "use \"size\": \"display\" (or title/body) so the role picks the token, or change the composition's \"type\" scale"));
                            }
                        }
                    }
                }
                // contrast against the composition background or a full-frame solid below; skip when a card or photo sits under the type
                let mut bg = frames.first().and_then(|f| f.bg).unwrap_or([0.0, 0.0, 0.0, 1.0]);
                let mut covered = false;
                for b in &root.layers[..l.index] {
                    let Some(st) = shown.first().map(|&i| &frames[i].layers[b.index]) else { continue };
                    if !super::shows(st) {
                        continue;
                    }
                    match &st.content {
                        Content::Solid { color, size } if size[0] >= w * 0.95 && size[1] >= h * 0.95 => bg = *color,
                        Content::Solid { .. } | Content::Image { .. } | Content::Video { .. } | Content::Texture { .. } | Content::Shape { .. } | Content::Precomp { .. } => {
                            covered = true;
                            break;
                        }
                        _ => {}
                    }
                }
                if !covered {
                    if let Some(&i) = shown.first() {
                        if let Content::Text(t) = &frames[i].layers[l.index].content {
                            let sample = t.fill.as_ref().and_then(|f| f.color).unwrap_or(t.color);
                            let r = contrast(luminance(sample), luminance(bg));
                            let body = t.size < h * 0.05;
                            if r < 3.0 {
                                out.push(Finding { severity: Severity::Error, rule: "low-contrast", layer: Some(l.label.clone()), t: Some(times[i]), message: format!("text/background contrast is {r:.1}:1 (3:1 is the floor for any type)"), fix: "lighten the text, darken the background, or add a backing shape".into() });
                            } else if body && r < 4.5 {
                                out.push(warn("low-contrast", &l.label, Some(times[i]), format!("text/background contrast is {r:.1}:1 (4.5:1 is the minimum for body text)"), "lighten the text, darken the background, or add a backing shape"));
                            }
                        }
                    }
                }
            }
            Kind::Image(_) | Kind::Video(_) => {
                // pixels shown larger than the file has
                let mut worst: Option<(f64, f64)> = None;
                for &i in &shown {
                    let st = &frames[i].layers[l.index];
                    let (view, b) = match (&st.content, st.bounds) {
                        (Content::Image { view, .. } | Content::Video { view, .. }, Some(b)) => (view, b),
                        _ => continue,
                    };
                    // screen pixels per layer unit, times layer units per file pixel
                    let ratio = (b[2] - b[0]) / view.viewport[0].max(1.0) * view.dest[2] / view.src[2].max(1.0);
                    if worst.is_none_or(|(r, _)| ratio > r) {
                        worst = Some((ratio, times[i]));
                    }
                }
                if let Some((r, t)) = worst.filter(|(r, _)| *r > 1.5) {
                    out.push(warn("upscaled-media", &l.label, Some(t), format!("the picture is shown {r:.1}x larger than its pixels, so it will look soft"), "use a higher-resolution source, or reduce the zoom/scale"));
                }
            }
            _ => {}
        }
    }
    out.extend(visible_effect_inputs(plan, &ev)?);
    out.extend(super::cover::run(plan));
    Ok(out)
}

/// Moments of a composition at which the layers an effect reads are looked at.
const EFFECT_INPUT_SAMPLES: usize = 24;

/// A layer an effect reads as a map or mask is drawn on the composite as well unless it is hidden, so the map shows on
/// screen. That is rarely meant: say `"hidden": true` on it.
fn visible_effect_inputs(plan: &Plan, ev: &Evaluator) -> anyhow::Result<Vec<Finding>> {
    let mut out = vec![];
    for (name, comp) in &plan.comps {
        // (layer read, who reads it, through which effect)
        let readers = comp.layers.iter().flat_map(|l| l.fx.iter().map(move |f| (l.label.as_str(), f))).chain(comp.look.iter().map(|f| ("look", f)));
        let mut reads: Vec<(usize, &str, &str)> = vec![];
        for (reader, f) in readers {
            reads.extend(f.layers_read().into_iter().map(|read| (read, reader, f.name())));
        }
        for (read, reader, effect) in reads {
            let seen = (0..EFFECT_INPUT_SAMPLES).map(|k| comp.dur * (k as f64 + 0.5) / EFFECT_INPUT_SAMPLES as f64).find_map(|t| match ev.frame(name, t) {
                Ok(f) => shows(&f.layers[read]).then_some(Ok(t)),
                Err(e) => Some(Err(e)),
            });
            if let Some(t) = seen.transpose()? {
                let label = &comp.layers[read].label;
                let place = if name.is_empty() { String::new() } else { format!(" in comp {name:?}") };
                out.push(warn("visible-effect-input", &format!("{label}{place}"), Some(t), format!("is read by effect {effect:?} on {reader} but is also drawn on screen, so the map itself shows"), "set \"hidden\": true on it; an effect still reads a hidden layer"));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn findings(v: serde_json::Value) -> Vec<&'static str> {
        run(&Plan::from_value(v, std::path::Path::new(".")).unwrap()).unwrap().iter().map(|f| f.rule).collect()
    }

    #[test]
    fn it_finds_the_usual_mistakes_and_stays_quiet_about_a_good_composition() {
        let good = json!({"dur": 3, "layers": [
            {"type": "solid", "color": "#101828"},
            {"type": "text", "text": "Hello", "size": 96, "color": "#ffffff", "pos": [960, 540], "align": "center"}
        ]});
        assert_eq!(findings(good), Vec::<&str>::new());
        let bad = json!({"dur": 3, "layers": [
            {"type": "text", "text": "Edge", "size": 96, "color": "#ffffff", "pos": [20, 540]},
            {"type": "text", "text": "Dim", "size": 96, "color": "#111111", "pos": [960, 300], "align": "center"},
            {"type": "text", "text": "tiny", "size": 12, "color": "#ffffff", "pos": [960, 900], "align": "center"},
            {"type": "solid", "color": "#fff", "start": 9},
            {"type": "solid", "color": "#fff", "opacity": 0, "size": [10, 10]},
            {"type": "shape", "pos": [5000, 5000], "items": [{"rect": [0, 0, 50, 50], "fill": "#fff"}]},
            {"type": "solid", "color": "#fff", "three_d": true}
        ]});
        let got = findings(bad);
        for rule in ["outside-safe-area", "low-contrast", "text-too-small", "starts-after-end", "never-visible", "never-on-screen", "3d-without-camera"] {
            assert!(got.contains(&rule), "{rule} missing from {got:?}");
        }
    }

    #[test]
    fn text_cut_off_by_a_nested_composition_is_reported() {
        let v = json!({"dur": 2, "comps": {"small": {"width": 300, "height": 100, "dur": 2, "layers": [
            {"type": "text", "text": "This is far too wide for its box", "size": 60, "color": "#fff", "pos": [10, 50]}]}},
            "layers": [{"type": "solid", "color": "#123"}, {"type": "precomp", "comp": "small", "pos": [960, 540]}]});
        assert!(findings(v).contains(&"clipped-by-comp"));
    }

    #[test]
    fn a_full_frame_texture_on_a_smaller_plate_is_reported() {
        let v = json!({"dur": 2, "comps": {"card": {"width": 800, "height": 400, "dur": 2, "layers": [
            {"type": "shape", "id": "plate", "pos": [400, 200], "layout": "backdrop",
             "items": [{"rect": [-300, -150, 600, 300], "radius": 16, "fill": "#123"}]},
            {"type": "texture", "shader": "ridgeline", "params": {"tint": "#e2c48a"}}
        ]}}, "layers": [{"type": "solid", "color": "#000"}, {"type": "precomp", "comp": "card", "pos": [960, 540]}]});
        let got = findings(v);
        assert!(got.contains(&"paints-past-plate"), "{got:?}");
        let clipped = json!({"dur": 2, "comps": {"card": {"width": 800, "height": 400, "dur": 2, "layers": [
            {"type": "shape", "id": "plate", "pos": [400, 200], "layout": "backdrop",
             "items": [{"rect": [-300, -150, 600, 300], "radius": 16, "fill": "#123"}]},
            {"type": "texture", "shader": "ridgeline", "params": {"tint": "#e2c48a"}, "clip_to": "plate"}
        ]}}, "layers": [{"type": "solid", "color": "#000"}, {"type": "precomp", "comp": "card", "pos": [960, 540]}]});
        let got = findings(clipped);
        assert!(!got.contains(&"paints-past-plate"), "{got:?}");
    }

    #[test]
    fn a_nested_3d_block_without_a_camera_is_reported() {
        let v = json!({"dur": 2, "comps": {"card": {"width": 400, "height": 200, "dur": 2, "layers": [
            {"type": "shape", "three_d": true, "items": [{"rect": [0, 0, 100, 80], "fill": "#fff"}]}
        ]}}, "layers": [{"type": "precomp", "comp": "card", "pos": [960, 540]}]});
        assert!(findings(v).contains(&"nested-3d-flatten"));
    }

    #[test]
    fn a_map_layer_that_is_also_drawn_is_reported_and_a_hidden_one_is_not() {
        let comp = |hidden: bool| {
            json!({"dur": 2, "layers": [
                {"type": "texture", "shader": "noise", "id": "map", "hidden": hidden},
                {"type": "solid", "color": "#123", "fx": [{"effect": "displacement_map", "map": "map"}]}]})
        };
        assert!(findings(comp(false)).contains(&"visible-effect-input"));
        assert!(!findings(comp(true)).contains(&"visible-effect-input"));
        let mask = json!({"dur": 2, "layers": [
            {"type": "texture", "shader": "noise", "id": "map"},
            {"type": "solid", "color": "#123", "fx": [{"effect": "blur", "composite": {"mask": {"kind": "layer", "layer": "map"}}}]}]});
        assert!(findings(mask).contains(&"visible-effect-input"));
    }

    #[test]
    fn small_text_in_a_mocked_interface_is_declared_not_ignored() {
        let plain = json!({"dur": 2, "layers": [{"type": "text", "text": "General", "size": 20, "color": "#fff", "pos": [960, 540], "align": "center"}]});
        assert!(findings(plain).contains(&"text-too-small"));
        let ui = json!({"dur": 2, "layers": [{"type": "text", "text": "General", "size": 20, "color": "#fff", "pos": [960, 540], "align": "center", "ui": true}]});
        assert!(!findings(ui).contains(&"text-too-small"));
        let off_screen_ui = json!({"dur": 2, "layers": [{"type": "text", "text": "General", "size": 20, "color": "#fff", "pos": [10, 540], "ui": true}]});
        assert!(findings(off_screen_ui).contains(&"outside-safe-area"), "safe area still applies to ui text");
    }
}
