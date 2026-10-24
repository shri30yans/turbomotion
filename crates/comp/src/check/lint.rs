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
                if let Some(&i) = shown.iter().find(|i| matches!(&frames[**i].layers[l.index].content, Content::Text(t) if t.size < h * 0.024)) {
                    if let Content::Text(t) = &frames[i].layers[l.index].content {
                        out.push(warn("text-too-small", &l.label, Some(times[i]), format!("text is {:.0}px tall in a {h:.0}px frame", t.size), "use at least 2.5% of the frame height (27px at 1080p) so it reads on a phone"));
                    }
                }
                // contrast against the plain background, when nothing else is painted below
                let below_paints = root.layers[..l.index].iter().any(|b| matches!(b.def.kind, Kind::Solid(_) | Kind::Image(_) | Kind::Video(_) | Kind::Texture(_) | Kind::Shape(_) | Kind::Precomp(_)));
                if !below_paints {
                    if let (Some(&i), Some(bg)) = (shown.first(), frames.first().map(|f| f.bg.unwrap_or([0.0, 0.0, 0.0, 1.0]))) {
                        if let Content::Text(t) = &frames[i].layers[l.index].content {
                            let r = contrast(luminance(t.color), luminance(bg));
                            if r < 4.5 {
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
}
