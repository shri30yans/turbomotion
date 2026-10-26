//! Reading the temporal handles of a key (`ease_in`, `ease_out`) and the handles of its motion path (`in`, `out`).

use anyhow::{anyhow, bail, ensure, Result};
use serde_json::{Map, Value as Json};
use turbomotion_core::suggest::hint;

use super::{
    key::{Handle, Point, Side, Tangents},
    parse::num,
    shape::LINEAR_INFLUENCE,
    value::Kind,
};

/// The influence of "easy": After Effects' Easy Ease is a speed of zero and an influence of 33.33 %.
const EASY_INFLUENCE: f64 = LINEAR_INFLUENCE;

/// `"linear"`, `"hold"`, `"auto"`, `"easy"` or `{"speed": s, "influence": percent}`. A handle without `speed` is the
/// half of a `continuous` pair that takes its speed from the other side; it comes back with an empty speed.
pub(super) fn side(field: &str, v: &Json) -> Result<Side> {
    match v {
        Json::String(s) => match s.as_str() {
            "linear" => Ok(Side::Linear),
            "hold" => Ok(Side::Hold),
            "auto" => Ok(Side::Auto),
            "easy" => Ok(Side::Bezier(Handle { speed: vec![0.0], influence: EASY_INFLUENCE })),
            other => bail!("{field} is \"linear\", \"hold\", \"auto\", \"easy\" or {{\"speed\", \"influence\"}}, not {other:?}.{}", hint(other, &["linear", "hold", "auto", "easy"])),
        },
        Json::Object(o) => handle(field, o).map(Side::Bezier),
        other => bail!("{field} is \"linear\", \"hold\", \"auto\", \"easy\" or {{\"speed\", \"influence\"}}, not {other}"),
    }
}

fn handle(field: &str, o: &Map<String, Json>) -> Result<Handle> {
    const FIELDS: &[&str] = &["speed", "influence"];
    if let Some(bad) = o.keys().find(|k| !FIELDS.contains(&k.as_str())) {
        bail!("{field} has no field {bad:?}; a handle is {{\"speed\", \"influence\"}}.{}", hint(bad, FIELDS));
    }
    let percent = num(&format!("{field} influence"), o.get("influence").ok_or_else(|| anyhow!("{field} needs an \"influence\" (percent, 0 to 100)"))?)?;
    let speed = match o.get("speed") {
        None => Vec::new(),
        Some(Json::Array(a)) => a.iter().map(|s| num(&format!("{field} speed"), s)).collect::<Result<_>>()?,
        Some(s) => vec![num(&format!("{field} speed"), s)?],
    };
    Ok(Handle { speed, influence: percent / 100.0 })
}

/// Makes the two sides of a `continuous` key share one speed, and checks no other handle lacks one.
pub(super) fn join(into: &mut Option<Side>, out: &mut Option<Side>, continuous: bool) -> Result<()> {
    if continuous {
        let (Some(Side::Bezier(i)), Some(Side::Bezier(o))) = (into.as_mut(), out.as_mut()) else {
            bail!("a continuous key takes its ease_in and ease_out both as {{\"speed\", \"influence\"}} (one of them may leave the speed out)");
        };
        match (i.speed.is_empty(), o.speed.is_empty()) {
            (true, true) => bail!("a continuous key needs a \"speed\" on ease_in or ease_out"),
            (true, false) => i.speed = o.speed.clone(),
            (false, true) => o.speed = i.speed.clone(),
            (false, false) => ensure!(i.speed == o.speed, "a continuous key has one speed, but ease_in has {:?} and ease_out {:?}", i.speed, o.speed),
        }
    }
    for (name, s) in [("ease_in", into), ("ease_out", out)] {
        if matches!(s, Some(Side::Bezier(h)) if h.speed.is_empty()) {
            bail!("{name} needs a \"speed\" (or \"continuous\": true to take it from the other side)");
        }
    }
    Ok(())
}

/// The motion path handles at a key. `default` is the property's `spatial` setting for a key that says nothing.
pub(super) fn tangents(o: &Map<String, Json>, kind: Kind, default: &Tangents) -> Result<Tangents> {
    let offset = |field: &str| -> Result<Option<Point>> {
        let Some(v) = o.get(field) else { return Ok(None) };
        let parts = kind.read(v).map_err(|e| anyhow!("\"{field}\" is a handle with the same number of parts as the value: {e}"))?;
        let p = parts.parts();
        Ok(Some([p.v[0], p.v[1], p.v[2]]))
    };
    let (into, out) = (offset("in")?, offset("out")?);
    let mode = o.get("spatial").map(|m| m.as_str().ok_or_else(|| anyhow!("\"spatial\" is \"auto\", \"linear\", \"bezier\" or \"continuous\", not {m}"))).transpose()?;
    let loose = |what: &str| -> Result<()> {
        ensure!(into.is_none() && out.is_none(), "\"in\" and \"out\" are the handles of a bezier key, but this key is spatial: {what:?}. Use spatial: \"bezier\" (or leave spatial out) to give handles");
        Ok(())
    };
    match mode {
        None if into.is_some() || out.is_some() => Ok(Tangents::Bezier { into: into.unwrap_or([0.0; 3]), out: out.unwrap_or([0.0; 3]) }),
        None => Ok(default.clone()),
        Some("auto") => loose("auto").map(|()| Tangents::Auto),
        Some("linear") => loose("linear").map(|()| Tangents::Linear),
        Some("bezier") => Ok(Tangents::Bezier { into: into.unwrap_or([0.0; 3]), out: out.unwrap_or([0.0; 3]) }),
        Some("continuous") => {
            let mirror = |p: Point| p.map(|x| -x);
            match (into, out) {
                (None, None) => bail!("a continuous key needs \"in\" or \"out\"; the other handle is its mirror"),
                (Some(i), None) => Ok(Tangents::Bezier { into: i, out: mirror(i) }),
                (None, Some(o)) => Ok(Tangents::Bezier { into: mirror(o), out: o }),
                (Some(i), Some(o)) => {
                    ensure!(opposite(i, o), "a continuous key's \"in\" {i:?} and \"out\" {o:?} must point opposite ways along one line");
                    Ok(Tangents::Bezier { into: i, out: o })
                }
            }
        }
        Some(other) => bail!("\"spatial\" is \"auto\", \"linear\", \"bezier\" or \"continuous\", not {other:?}.{}", hint(other, &["auto", "linear", "bezier", "continuous"])),
    }
}

/// Whether two handles lie on one line pointing away from each other (a zero handle fits any).
fn opposite(a: Point, b: Point) -> bool {
    let norm = |p: Point| p.iter().map(|x| x * x).sum::<f64>().sqrt();
    let cross = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let dot: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    norm(cross) <= 1e-9 * norm(a) * norm(b) && dot <= 0.0
}
