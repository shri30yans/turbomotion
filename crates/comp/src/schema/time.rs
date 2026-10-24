//! The pre-pass over the raw file: `$variables` and time literals are replaced by plain numbers before the strict
//! parse, so every later stage sees seconds only.
//!
//! Time literals (anywhere a time is expected: `start`, `dur`, `end`, `at`, `delay`, `t`, and the first item of a key):
//!
//! ```text
//! 1.5s   45f   beat:8   bar:2   marker:drop   title.end   title.start   prev.end   comp.end   (each optionally  +0.25  or  -2f)
//! ```
//!
//! `beat:N` and `bar:N` need `bpm`; `prev` is the layer just before; a layer id needs to come earlier in the list.

use std::collections::HashMap;

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::{Map, Value};

const TIME_KEYS: &[&str] = &["start", "end", "dur", "at", "delay", "t"];

struct Ctx {
    fps: f64,
    bpm: Option<f64>,
    beat_offset: f64,
    markers: HashMap<String, f64>,
    layers: HashMap<String, (f64, Option<f64>)>,
    prev: Option<(f64, Option<f64>)>,
    comp_dur: f64,
}

pub fn preprocess(root: &mut Value) -> Result<()> {
    let vars = root.get("vars").and_then(Value::as_object).cloned().unwrap_or_default();
    substitute(root, &vars, false)?;
    let comp_durs: HashMap<String, f64> = root.get("comps").and_then(Value::as_object).map(|c| c.iter().filter_map(|(k, v)| Some((k.clone(), v.get("dur")?.as_f64()?))).collect()).unwrap_or_default();
    let fps = root.get("fps").and_then(Value::as_f64).unwrap_or(30.0);
    resolve_comp(root, fps, &comp_durs).context("composition")?;
    if let Some(comps) = root.get_mut("comps").and_then(Value::as_object_mut) {
        for (name, comp) in comps.iter_mut() {
            resolve_comp(comp, fps, &comp_durs).with_context(|| format!("comp {name:?}"))?;
        }
    }
    Ok(())
}

/// Replace `$name` / `${name}` in a whole JSON value (used to instantiate blocks).
pub fn substitute_vars(v: &mut Value, vars: &Map<String, Value>) -> Result<()> {
    substitute(v, vars, true)
}

/// `"$name"` becomes the variable's value (any JSON type); `"a ${name} b"` interpolates text.
fn substitute(v: &mut Value, vars: &Map<String, Value>, inside_vars: bool) -> Result<()> {
    match v {
        Value::String(s) => {
            if let Some(name) = s.strip_prefix('$').filter(|n| !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || c == '_')) {
                *v = vars.get(name).cloned().ok_or_else(|| anyhow!("unknown variable ${name}; the variables are {}", names(vars)))?;
            } else if s.contains("${") {
                let mut out = String::new();
                let mut rest = s.as_str();
                while let Some(i) = rest.find("${") {
                    out.push_str(&rest[..i]);
                    let j = rest[i..].find('}').ok_or_else(|| anyhow!("unclosed ${{ in {s:?}"))? + i;
                    let name = &rest[i + 2..j];
                    let val = vars.get(name).ok_or_else(|| anyhow!("unknown variable ${{{name}}}; the variables are {}", names(vars)))?;
                    out.push_str(&val.as_str().map_or_else(|| val.to_string(), str::to_string));
                    rest = &rest[j + 1..];
                }
                out.push_str(rest);
                *v = Value::String(out);
            }
        }
        Value::Array(a) => a.iter_mut().try_for_each(|x| substitute(x, vars, inside_vars))?,
        Value::Object(o) => {
            for (k, x) in o.iter_mut() {
                if k == "vars" && !inside_vars {
                    continue; // the declaration itself is not substituted
                }
                substitute(x, vars, inside_vars)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn names(vars: &Map<String, Value>) -> String {
    if vars.is_empty() { "none (declare them under \"vars\")".into() } else { vars.keys().map(|k| format!("${k}")).collect::<Vec<_>>().join(", ") }
}

fn resolve_comp(comp: &mut Value, fps: f64, comp_durs: &HashMap<String, f64>) -> Result<()> {
    let dur = comp.get("dur").and_then(Value::as_f64).unwrap_or(5.0);
    let bpm = comp.get("bpm").and_then(Value::as_f64);
    let beat_offset = comp.get("beat_offset").and_then(Value::as_f64).unwrap_or(0.0);
    let mut ctx = Ctx { fps, bpm, beat_offset, markers: HashMap::new(), layers: HashMap::new(), prev: None, comp_dur: dur };
    if let Some(ms) = comp.get_mut("markers").and_then(Value::as_array_mut) {
        for m in ms.iter_mut() {
            if let Some(t) = m.get_mut("t") {
                resolve_in_place(t, &ctx)?;
            }
            if let (Some(t), Some(n)) = (m.get("t").and_then(Value::as_f64), m.get("name").and_then(Value::as_str)) {
                ctx.markers.insert(n.to_string(), t);
            }
        }
    }
    let Some(layers) = comp.get_mut("layers").and_then(Value::as_array_mut) else { return Ok(()) };
    for (i, layer) in layers.iter_mut().enumerate() {
        resolve_layer(layer, &mut ctx, comp_durs).with_context(|| format!("layer {i}"))?;
    }
    Ok(())
}

fn resolve_layer(layer: &mut Value, ctx: &mut Ctx, comp_durs: &HashMap<String, f64>) -> Result<()> {
    let Some(o) = layer.as_object_mut() else { return Ok(()) };
    // start first: later fields may refer to it only through ids, not through itself.
    if let Some(v) = o.get_mut("start") {
        resolve_in_place(v, ctx)?;
    }
    let start = o.get("start").and_then(Value::as_f64).unwrap_or(0.0);
    if let Some(v) = o.get_mut("end") {
        resolve_in_place(v, ctx)?;
        let end = v.as_f64().ok_or_else(|| anyhow!("\"end\" must be a time"))?;
        ensure!(!o.contains_key("dur"), "give a layer \"dur\" or \"end\", not both");
        ensure!(end >= start, "\"end\" ({end}s) is before the layer's start ({start}s)");
        o.remove("end");
        o.insert("dur".into(), Value::from(end - start));
    }
    let own = Value::Object(o.clone());
    let _ = own;
    for (k, v) in o.iter_mut() {
        if k == "start" {
            continue;
        }
        let in_ctx = ctx_for_children(ctx);
        resolve_walk(v, Some(k), &in_ctx)?;
    }
    let dur = o.get("dur").and_then(Value::as_f64);
    let ty = o.get("type").and_then(Value::as_str).unwrap_or("");
    let end = match (dur, ty) {
        (Some(d), _) => Some(start + d),
        (None, "precomp") => {
            let name = o.get("comp").and_then(Value::as_str).unwrap_or("");
            let speed = o.get("speed").and_then(Value::as_f64).unwrap_or(1.0).max(1e-6);
            comp_durs.get(name).map(|d| start + (d - o.get("offset").and_then(Value::as_f64).unwrap_or(0.0)) / speed).map(|e| e.min(ctx.comp_dur.max(start)))
        }
        (None, "video" | "audio") => None,
        (None, _) => Some(ctx.comp_dur),
    };
    let entry = (start, end);
    if let Some(id) = o.get("id").and_then(Value::as_str) {
        ensure!(!ctx.layers.contains_key(id), "two layers are called {id:?}");
        ctx.layers.insert(id.to_string(), entry);
    }
    ctx.prev = Some(entry);
    Ok(())
}

fn ctx_for_children(ctx: &Ctx) -> Ctx {
    Ctx { fps: ctx.fps, bpm: ctx.bpm, beat_offset: ctx.beat_offset, markers: ctx.markers.clone(), layers: ctx.layers.clone(), prev: ctx.prev, comp_dur: ctx.comp_dur }
}

/// Walk a layer's fields; resolve time-shaped strings under time keys and in the first slot of keyframes.
fn resolve_walk(v: &mut Value, key: Option<&str>, ctx: &Ctx) -> Result<()> {
    match v {
        Value::String(_) if key.is_some_and(|k| TIME_KEYS.contains(&k)) => resolve_in_place(v, ctx)?,
        Value::Array(a) => {
            let keys_like = a.iter().all(|x| x.is_array() || x.is_object()) && !a.is_empty();
            for x in a.iter_mut() {
                if keys_like {
                    if let Some(first) = x.as_array_mut().and_then(|k| k.first_mut()) {
                        if first.is_string() {
                            resolve_in_place(first, ctx)?;
                        }
                    }
                }
                resolve_walk(x, None, ctx)?;
            }
        }
        Value::Object(o) => {
            for (k, x) in o.iter_mut() {
                resolve_walk(x, Some(k), ctx)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn resolve_in_place(v: &mut Value, ctx: &Ctx) -> Result<()> {
    if let Value::String(s) = v {
        if let Some(t) = parse_literal(s, ctx).with_context(|| format!("time {s:?}"))? {
            *v = Value::from(t);
        }
    }
    Ok(())
}

fn number(s: &str, fps: f64) -> Option<f64> {
    let s = s.trim();
    if let Some(f) = s.strip_suffix('f') {
        return f.parse::<f64>().ok().map(|x| x / fps);
    }
    s.strip_suffix('s').unwrap_or(s).parse().ok()
}

/// `Ok(None)` when the text is not shaped like a time (it may be an expression or a name).
fn parse_literal(text: &str, ctx: &Ctx) -> Result<Option<f64>> {
    let text = text.trim();
    // split a trailing +/- offset
    let mut head = text;
    let mut offset = 0.0;
    for (i, c) in text.char_indices().rev() {
        if (c == '+' || c == '-') && i > 0 {
            if let Some(n) = number(&text[i + 1..], ctx.fps) {
                head = text[..i].trim_end();
                offset = if c == '-' { -n } else { n };
                break;
            }
        }
    }
    let base = if let Some(n) = number(head, ctx.fps).filter(|_| head.ends_with(|c: char| c == 's' || c == 'f' || c.is_ascii_digit())) {
        n
    } else if let Some(n) = head.strip_prefix("beat:") {
        beat(n.trim().parse::<f64>().map_err(|_| anyhow!("beat:N needs a number"))?, ctx)?
    } else if let Some(n) = head.strip_prefix("bar:") {
        beat(4.0 * n.trim().parse::<f64>().map_err(|_| anyhow!("bar:N needs a number"))?, ctx)?
    } else if let Some(name) = head.strip_prefix("marker:") {
        *ctx.markers.get(name.trim()).ok_or_else(|| anyhow!("no marker called {name:?}; markers are {}", list(ctx.markers.keys())))?
    } else if head == "comp.end" || head == "end" {
        ctx.comp_dur
    } else if let Some((id, field)) = head.rsplit_once('.').filter(|(_, f)| ["start", "end", "dur"].contains(f)) {
        let (start, end) = if id == "prev" {
            ctx.prev.ok_or_else(|| anyhow!("\"prev\" needs a layer before this one"))?
        } else {
            *ctx.layers.get(id).ok_or_else(|| {
                anyhow!("no earlier layer with id {id:?}; ids so far: {}. (A layer can only refer to layers listed before it.)", list(ctx.layers.keys()))
            })?
        };
        match field {
            "start" => start,
            "end" => end.ok_or_else(|| anyhow!("layer {id:?} has no known end (video and audio need \"dur\" to be referenced)"))?,
            _ => end.ok_or_else(|| anyhow!("layer {id:?} has no known duration"))? - start,
        }
    } else {
        return Ok(None);
    };
    if head.contains(|c: char| c == ':' || c == '.') || number(head, ctx.fps).is_some() {
        Ok(Some(base + offset))
    } else {
        bail!("not a time")
    }
}

fn beat(n: f64, ctx: &Ctx) -> Result<f64> {
    let bpm = ctx.bpm.ok_or_else(|| anyhow!("beat:/bar: times need \"bpm\" on the composition"))?;
    Ok(ctx.beat_offset + n * 60.0 / bpm)
}

fn list<'a>(it: impl Iterator<Item = &'a String>) -> String {
    let mut v: Vec<_> = it.cloned().collect();
    v.sort();
    if v.is_empty() { "none".into() } else { v.join(", ") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(v: Value) -> Result<Value> {
        let mut v = v;
        preprocess(&mut v)?;
        Ok(v)
    }

    #[test]
    fn literals_become_seconds() {
        let v = run(json!({"fps": 30, "dur": 10, "bpm": 120, "markers": [{"t": "beat:8", "name": "drop"}], "layers": [
            {"type": "solid", "id": "a", "start": "1s", "dur": 2, "color": "#fff"},
            {"type": "null", "id": "b", "start": "a.end+0.5", "end": "marker:drop+30f"},
            {"type": "null", "start": "prev.end", "dur": "bar:1"}
        ]})).unwrap();
        let l = &v["layers"];
        assert_eq!(l[0]["start"], 1.0);
        assert_eq!(l[1]["start"], 3.5);
        assert_eq!(l[1]["dur"], 1.5);
        assert_eq!(l[2]["start"], 5.0);
        assert_eq!(l[2]["dur"], 2.0);
        assert_eq!(v["markers"][0]["t"], 4.0);
    }

    #[test]
    fn keys_and_tweens_take_literals_too() {
        let v = run(json!({"bpm": 60, "layers": [{"type": "null", "opacity": [["beat:1", 0], ["beat:2", 1]], "scale": {"from": 0, "to": 1, "at": "beat:3", "dur": "0.5s"}}]})).unwrap();
        assert_eq!(v["layers"][0]["opacity"][1][0], 2.0);
        assert_eq!(v["layers"][0]["scale"]["at"], 3.0);
        assert_eq!(v["layers"][0]["scale"]["dur"], 0.5);
    }

    #[test]
    fn expressions_and_names_are_left_alone() {
        let v = run(json!({"layers": [{"type": "text", "id": "t", "text": "beat:8", "opacity": "time * 2", "font": "Helvetica Neue"}]})).unwrap();
        assert_eq!(v["layers"][0]["text"], "beat:8");
        assert_eq!(v["layers"][0]["opacity"], "time * 2");
    }

    #[test]
    fn mistakes_say_what_is_known() {
        let e = format!("{:#}", run(json!({"layers": [{"type": "null", "start": "ghost.end"}]})).unwrap_err());
        assert!(e.contains("no earlier layer") && e.contains("ghost"), "{e}");
        let e = format!("{:#}", run(json!({"layers": [{"type": "null", "start": "beat:4"}]})).unwrap_err());
        assert!(e.contains("bpm"), "{e}");
        let e = format!("{:#}", run(json!({"layers": [{"type": "null", "start": "marker:x"}]})).unwrap_err());
        assert!(e.contains("no marker"), "{e}");
    }

    #[test]
    fn variables_substitute_values_and_text() {
        let v = run(json!({"vars": {"accent": "#ff8800", "n": 3, "who": "Ada"}, "layers": [{"type": "text", "text": "Hi ${who}", "color": "$accent", "size": "$n"}]})).unwrap();
        assert_eq!(v["layers"][0]["color"], "#ff8800");
        assert_eq!(v["layers"][0]["size"], 3);
        assert_eq!(v["layers"][0]["text"], "Hi Ada");
        assert!(run(json!({"layers": [{"type": "null", "x": "$nope"}]})).is_err());
    }
}
