//! Named tokens, resolved before the strict parse so layers never hard-code a film's ink or type.
//!
//! * **`palette`** — shared colours (`accent`, `ink`, `plate`, `loss`). Any kind may name them.
//! * **`type`** — text styles per role (`display` / `title` / `body`): size, font, weight, colour, tracking.
//! * **`space`** — pad, gap, radius. Published as `$pad`; a shape may say `"radius": "radius"`.
//! * **`motion`** — enter / exit / snap durations. Fills a layer enter/exit that omitted `dur`.
//! * **`image`** — default treatment for pictures (`treat` becomes `fx` when the layer has none).
//! * **`surface`** — named plate recipes (fill, stroke, radius) for shapes.
//! * **`look`** — finishing on the finished frame (grain, vignette). Not fonts; that is `type`.
//!
//! `"on": "plate"` parents a layer to that id and puts it in front of the face (`z: -4`).

use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use serde_json::{json, Map, Value};

const ROLES: &[&str] = &["display", "title", "body"];
const COLOR_FIELDS: &[&str] = &["color", "bg", "tint", "fill", "high", "low", "line", "haze", "ink", "edge"];

/// One type role: a number is still just a size; an object is the full style.
#[derive(Debug, Clone, Default)]
pub struct TypeRole {
    pub size: Option<f64>,
    pub font: Option<String>,
    pub weight: Option<u16>,
    pub tracking: Option<f64>,
    pub color: Option<String>,
}

/// The three type roles. `"type": {"display": 140}` and `"type": {"display": {"size": 140, "font": "Avenir Next"}}` both work.
#[derive(Debug, Clone, Default)]
pub struct TypeScale {
    pub display: Option<TypeRole>,
    pub title: Option<TypeRole>,
    pub body: Option<TypeRole>,
}

impl TypeScale {
    /// Fill in missing sizes from the frame height. Font and colour stay as declared.
    pub fn resolved(&self, height: f64) -> ResolvedScale {
        let h = height.max(1.0);
        ResolvedScale {
            display: self.display.as_ref().and_then(|r| r.size).unwrap_or((h * 0.13).round().max(72.0)),
            title: self.title.as_ref().and_then(|r| r.size).unwrap_or((h * 0.044).round().max(36.0)),
            body: self.body.as_ref().and_then(|r| r.size).unwrap_or((h * 0.028).round().max(24.0)),
        }
    }
}

impl<'de> Deserialize<'de> for TypeScale {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        parse_type_scale(&v).map_err(serde::de::Error::custom)
    }
}

/// Pixel sizes for the three type roles, after tokens are applied. Used by check.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedScale {
    pub display: f64,
    pub title: f64,
    pub body: f64,
}

impl ResolvedScale {
    pub fn get(self, role: &str) -> Option<f64> {
        match role {
            "display" => Some(self.display),
            "title" => Some(self.title),
            "body" => Some(self.body),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedRole {
    size: f64,
    font: Option<String>,
    weight: Option<u16>,
    tracking: Option<f64>,
    color: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedType {
    display: ResolvedRole,
    title: ResolvedRole,
    body: ResolvedRole,
}

impl ResolvedType {
    fn get(&self, role: &str) -> Option<&ResolvedRole> {
        match role {
            "display" => Some(&self.display),
            "title" => Some(&self.title),
            "body" => Some(&self.body),
            _ => None,
        }
    }

    fn with_sizes_from_height(&self, height: f64) -> ResolvedType {
        let sizes = TypeScale::default().resolved(height);
        ResolvedType {
            display: ResolvedRole { size: sizes.display, ..self.display.clone() },
            title: ResolvedRole { size: sizes.title, ..self.title.clone() },
            body: ResolvedRole { size: sizes.body, ..self.body.clone() },
        }
    }
}

/// Copy `palette` into `vars` so `$accent` works, without overwriting a var the file already set.
pub fn publish_palette(root: &mut Value) {
    publish_map(root, "palette");
    publish_space(root);
}

fn publish_map(root: &mut Value, key: &str) {
    let Some(Value::Object(map)) = root.get(key).cloned() else { return };
    let vars = root.as_object_mut().expect("composition").entry("vars").or_insert_with(|| Value::Object(Map::new()));
    let Some(vars) = vars.as_object_mut() else { return };
    for (k, v) in map {
        vars.entry(k).or_insert(v);
    }
}

fn publish_space(root: &mut Value) {
    let Some(Value::Object(space)) = root.get("space").cloned() else { return };
    let vars = root.as_object_mut().expect("composition").entry("vars").or_insert_with(|| Value::Object(Map::new()));
    let Some(vars) = vars.as_object_mut() else { return };
    for (k, v) in space {
        if v.is_number() {
            vars.entry(k).or_insert(v);
        }
    }
}

/// In front of a 3D plate. +z is away from the camera.
const ON_FACE_Z: f64 = -4.0;

fn ease_name(role: &str) -> &'static str {
    match role {
        "decel" => "expo.out",
        "accel" => "power2.in",
        "standard" => "sine.inOut",
        _ => "expo.out",
    }
}

/// Replace type tokens, palette names, surface recipes and image defaults so the strict parse sees numbers and hex.
pub fn apply(root: &mut Value) -> Result<()> {
    let palette = palette_of(root);
    resolve_color_tokens(root, &palette)?;
    let space = root.get("space").and_then(Value::as_object).cloned().unwrap_or_default();
    resolve_space_tokens(root, &space);
    let motion = root.get("motion").cloned();
    apply_on_face(root);
    apply_motion(root, motion.as_ref())?;
    let h = root.get("height").and_then(Value::as_f64).unwrap_or(1080.0);
    let scale = scale_of(root, h, &palette)?;
    publish_role_fonts(root, &scale);
    let surfaces = root.get("surface").cloned();
    apply_comp(root, &scale)?;
    apply_surfaces(root, surfaces.as_ref())?;
    apply_image_treat(root)?;
    if let Some(comps) = root.get_mut("comps").and_then(Value::as_object_mut) {
        let names: Vec<String> = comps.keys().cloned().collect();
        for name in names {
            let has_type = comps.get(&name).and_then(|c| c.get("type")).is_some();
            let ch = comps.get(&name).and_then(|c| c.get("height")).and_then(Value::as_f64).unwrap_or(h);
            // A card must not inherit 140px display type from a 1080p parent. Full-frame nested scenes still share the film scale.
            let local = if has_type {
                scale_of(comps.get(&name).unwrap(), ch, &palette)?
            } else if ch < h * 0.85 {
                scale.with_sizes_from_height(ch)
            } else {
                scale.clone()
            };
            let local_surfaces = comps.get(&name).and_then(|c| c.get("surface")).cloned().or_else(|| surfaces.clone());
            resolve_space_tokens(comps.get_mut(&name).unwrap(), &space);
            apply_on_face(comps.get_mut(&name).unwrap());
            apply_motion(comps.get_mut(&name).unwrap(), motion.as_ref())?;
            apply_comp(comps.get_mut(&name).unwrap(), &local)?;
            apply_surfaces(comps.get_mut(&name).unwrap(), local_surfaces.as_ref())?;
            apply_image_treat(comps.get_mut(&name).unwrap())?;
        }
    }
    Ok(())
}

fn palette_of(root: &Value) -> Map<String, Value> {
    root.get("palette").and_then(Value::as_object).cloned().unwrap_or_default()
}

fn parse_type_scale(v: &Value) -> Result<TypeScale> {
    let o = v.as_object().ok_or_else(|| anyhow!("\"type\" is an object of display, title, body"))?;
    if let Some(bad) = o.keys().find(|k| !ROLES.contains(&k.as_str())) {
        bail!("\"type\" has unknown role {bad:?}; roles are display, title, body");
    }
    Ok(TypeScale { display: o.get("display").map(parse_role).transpose()?, title: o.get("title").map(parse_role).transpose()?, body: o.get("body").map(parse_role).transpose()? })
}

fn parse_role(v: &Value) -> Result<TypeRole> {
    if let Some(n) = v.as_f64() {
        return Ok(TypeRole { size: Some(n), ..TypeRole::default() });
    }
    let o = v.as_object().ok_or_else(|| anyhow!("a type role is a size or {{\"size\", \"font\", \"weight\", \"color\", \"tracking\"}}"))?;
    if let Some(bad) = o.keys().find(|k| !["size", "font", "weight", "color", "tracking"].contains(&k.as_str())) {
        bail!("unknown type-role field {bad:?}; valid: size, font, weight, color, tracking");
    }
    Ok(TypeRole {
        size: o.get("size").and_then(Value::as_f64),
        font: o.get("font").and_then(Value::as_str).map(str::to_string),
        weight: o.get("weight").and_then(Value::as_u64).map(|w| w as u16),
        tracking: o.get("tracking").and_then(Value::as_f64),
        color: o.get("color").and_then(Value::as_str).map(str::to_string),
    })
}

fn scale_of(comp: &Value, height: f64, palette: &Map<String, Value>) -> Result<ResolvedType> {
    let t = match comp.get("type") {
        None => TypeScale::default(),
        Some(v) => parse_type_scale(v)?,
    };
    let sizes = t.resolved(height);
    let role = |r: Option<&TypeRole>, size: f64| -> Result<ResolvedRole> {
        let r = r.cloned().unwrap_or_default();
        let color = r.color.map(|c| resolve_name(&c, palette).unwrap_or(c));
        Ok(ResolvedRole { size: r.size.unwrap_or(size), font: r.font, weight: r.weight, tracking: r.tracking, color })
    };
    Ok(ResolvedType { display: role(t.display.as_ref(), sizes.display)?, title: role(t.title.as_ref(), sizes.title)?, body: role(t.body.as_ref(), sizes.body)? })
}

fn resolve_name(name: &str, palette: &Map<String, Value>) -> Option<String> {
    palette.get(name).and_then(|v| v.as_str().map(str::to_string).or_else(|| Some(v.to_string())))
}

fn reserved_color(s: &str) -> bool {
    matches!(s, "metal" | "white" | "black" | "transparent" | "none" | "red" | "green" | "blue" | "yellow" | "gold") || s.starts_with('#')
}

fn resolve_color_tokens(v: &mut Value, palette: &Map<String, Value>) -> Result<()> {
    match v {
        Value::Object(o) => {
            let keys: Vec<String> = o.keys().cloned().collect();
            for k in keys {
                if COLOR_FIELDS.contains(&k.as_str()) {
                    if let Some(s) = o.get(&k).and_then(Value::as_str) {
                        if !reserved_color(s) {
                            if let Some(c) = resolve_name(s, palette) {
                                o.insert(k.clone(), json!(c));
                            } else if !s.starts_with('$') {
                                bail!("{k} {s:?} is not in palette ({}); use a hex, a name like white, or a palette token", names(palette));
                            }
                        }
                    }
                }
                if let Some(child) = o.get_mut(&k) {
                    resolve_color_tokens(child, palette)?;
                }
            }
        }
        Value::Array(a) => a.iter_mut().try_for_each(|x| resolve_color_tokens(x, palette))?,
        _ => {}
    }
    Ok(())
}

fn names(palette: &Map<String, Value>) -> String {
    let mut n: Vec<_> = palette.keys().cloned().collect();
    n.sort();
    if n.is_empty() { "(none)".into() } else { n.join(", ") }
}

fn publish_role_fonts(root: &mut Value, scale: &ResolvedType) {
    let fonts = root.as_object_mut().expect("composition").entry("fonts").or_insert_with(|| json!({}));
    let Some(fonts) = fonts.as_object_mut() else { return };
    for (role, r) in [("display", &scale.display), ("title", &scale.title), ("body", &scale.body)] {
        if let Some(font) = &r.font {
            fonts.entry(role.to_string()).or_insert_with(|| json!(font));
        }
    }
}

fn apply_comp(comp: &mut Value, scale: &ResolvedType) -> Result<()> {
    let Some(layers) = comp.get_mut("layers").and_then(Value::as_array_mut) else { return Ok(()) };
    for (i, layer) in layers.iter_mut().enumerate() {
        if layer.get("type").and_then(Value::as_str) != Some("text") {
            continue;
        }
        let role = layer.get("role").and_then(Value::as_str).map(str::to_string);
        if let Some(token) = layer.get("size").and_then(Value::as_str) {
            let r = scale.get(token).ok_or_else(|| anyhow!("layer {i}: \"size\" is {token:?}; use a number or one of display, title, body"))?;
            layer["size"] = json!(r.size);
        }
        if let Some(role) = role {
            let r = scale.get(&role).ok_or_else(|| anyhow!("layer {i}: \"role\" is {role:?}; use display, title or body"))?;
            if layer.get("size").is_none() {
                layer["size"] = json!(r.size);
            }
            if layer.get("font").is_none() {
                if let Some(font) = &r.font {
                    layer["font"] = json!(font);
                }
            }
            if layer.get("weight").is_none() {
                if let Some(w) = r.weight {
                    layer["weight"] = json!(w);
                }
            }
            if layer.get("tracking").is_none() {
                if let Some(t) = r.tracking {
                    layer["tracking"] = json!(t);
                }
            }
            if layer.get("color").is_none() && layer.get("fill").is_none() {
                if let Some(c) = &r.color {
                    layer["color"] = json!(c);
                }
            }
        }
    }
    Ok(())
}

fn apply_surfaces(comp: &mut Value, surfaces: Option<&Value>) -> Result<()> {
    let Some(layers) = comp.get_mut("layers").and_then(Value::as_array_mut) else { return Ok(()) };
    for (i, layer) in layers.iter_mut().enumerate() {
        let Some(name) = layer.get("surface").and_then(Value::as_str).map(str::to_string) else { continue };
        let recipe = surfaces.as_ref().and_then(|s| s.get(&name)).ok_or_else(|| anyhow!("layer {i}: surface {name:?} is not defined on this composition"))?;
        if layer.get("type").and_then(Value::as_str) != Some("shape") {
            bail!("layer {i}: \"surface\" is for shape layers");
        }
        let items = layer.get_mut("items").and_then(Value::as_array_mut).ok_or_else(|| anyhow!("layer {i}: a surface needs items"))?;
        let item = items.get_mut(0).and_then(Value::as_object_mut).ok_or_else(|| anyhow!("layer {i}: surface {name:?} needs a shape item"))?;
        if let Some(fill) = recipe.get("fill") {
            item.entry("fill").or_insert_with(|| fill.clone());
        }
        if let Some(stroke) = recipe.get("stroke") {
            item.entry("stroke").or_insert_with(|| stroke.clone());
        }
        if let Some(radius) = recipe.get("radius") {
            item.entry("radius").or_insert_with(|| radius.clone());
        }
        layer.as_object_mut().unwrap().remove("surface");
    }
    Ok(())
}

fn apply_image_treat(comp: &mut Value) -> Result<()> {
    let treat = match comp.get("image").and_then(|i| i.get("treat")) {
        Some(Value::Array(a)) => Some(a.iter().map(treat_to_fx).collect::<Result<Vec<_>>>()?),
        None => None,
        Some(other) => bail!("\"image\".treat is an array of effect names or effect objects, not {other}"),
    };
    let Some(treat) = treat else { return Ok(()) };
    let Some(layers) = comp.get_mut("layers").and_then(Value::as_array_mut) else { return Ok(()) };
    for layer in layers {
        if layer.get("type").and_then(Value::as_str) != Some("image") {
            continue;
        }
        let empty = layer.get("fx").is_none_or(|f| f.as_array().is_some_and(|a| a.is_empty()) || f.is_null());
        if empty {
            layer["fx"] = json!(treat);
        }
    }
    Ok(())
}

fn resolve_space_tokens(v: &mut Value, space: &Map<String, Value>) {
    if space.is_empty() {
        return;
    }
    match v {
        Value::Object(o) => {
            let keys: Vec<String> = o.keys().cloned().collect();
            for k in keys {
                if matches!(k.as_str(), "radius" | "pad" | "gap") {
                    if let Some(name) = o.get(&k).and_then(Value::as_str) {
                        if let Some(n) = space.get(name) {
                            o.insert(k.clone(), n.clone());
                        }
                    }
                }
                if let Some(child) = o.get_mut(&k) {
                    resolve_space_tokens(child, space);
                }
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| resolve_space_tokens(x, space)),
        _ => {}
    }
}

fn apply_on_face(comp: &mut Value) {
    let Some(layers) = comp.get_mut("layers").and_then(Value::as_array_mut) else { return };
    for layer in layers {
        let on = layer.get("on").cloned();
        let Some(on) = on else { continue };
        let target = match &on {
            Value::String(s) if !s.is_empty() => s.clone(),
            Value::Bool(true) => layer.get("parent").and_then(Value::as_str).unwrap_or("").to_string(),
            _ => {
                if let Some(o) = layer.as_object_mut() {
                    o.remove("on");
                }
                continue;
            }
        };
        let Some(o) = layer.as_object_mut() else { continue };
        if !target.is_empty() {
            o.entry("parent").or_insert_with(|| json!(target));
        }
        o.insert("three_d".into(), json!(true));
        match o.get_mut("pos") {
            None => {
                o.insert("pos".into(), json!([0, 0, ON_FACE_Z]));
            }
            Some(Value::Array(a)) => {
                if a.len() == 2 {
                    a.push(json!(ON_FACE_Z));
                } else if a.len() >= 3 && a[2].as_f64() == Some(0.0) {
                    a[2] = json!(ON_FACE_Z);
                }
            }
            _ => {}
        }
        o.remove("on");
    }
}

fn apply_motion(comp: &mut Value, motion: Option<&Value>) -> Result<()> {
    let Some(motion) = motion else { return Ok(()) };
    let Some(layers) = comp.get_mut("layers").and_then(Value::as_array_mut) else { return Ok(()) };
    for layer in layers {
        let role = layer.get("motion").and_then(Value::as_str).map(str::to_string);
        if let Some(o) = layer.as_object_mut() {
            o.remove("motion");
        }
        for key in ["enter", "exit"] {
            let spec_key = role.as_deref().unwrap_or(key);
            let Some(spec) = motion.get(spec_key) else { continue };
            let dur = spec.get("dur").and_then(Value::as_f64);
            let ease = spec.get("ease").and_then(Value::as_str).map(|e| {
                if e.contains('.') { e.to_string() } else { ease_name(e).to_string() }
            });
            let Some(Value::Object(enter)) = layer.get_mut(key) else { continue };
            let missing = enter.get("dur").and_then(Value::as_f64).unwrap_or(0.0) == 0.0;
            if missing {
                if let Some(d) = dur {
                    enter.insert("dur".into(), json!(d));
                }
                if enter.get("ease").is_none() {
                    if let Some(e) = &ease {
                        enter.insert("ease".into(), json!(e));
                    }
                }
            }
        }
    }
    Ok(())
}

fn treat_to_fx(v: &Value) -> Result<Value> {
    match v {
        Value::String(name) => Ok(json!({"effect": name})),
        Value::Object(o) if o.get("effect").and_then(Value::as_str).is_some() => Ok(v.clone()),
        other => bail!("a treat is an effect name or {{\"effect\": \"grain\", ...}}, not {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn size_tokens_and_roles_become_pixels() {
        let mut v = json!({"height": 1080, "type": {"display": 160}, "layers": [
            {"type": "text", "text": "N", "size": "display"},
            {"type": "text", "text": "T", "role": "title"},
            {"type": "text", "text": "B", "role": "body", "size": 22}
        ]});
        apply(&mut v).unwrap();
        assert_eq!(v["layers"][0]["size"], 160.0);
        assert_eq!(v["layers"][1]["size"], 48.0); // 4.4% of 1080
        assert_eq!(v["layers"][2]["size"], 22); // explicit wins
        let mut bad = json!({"layers": [{"type": "text", "text": "x", "size": "huge"}]});
        assert!(apply(&mut bad).unwrap_err().to_string().contains("display, title, body"));
    }

    #[test]
    fn a_small_nested_comp_gets_type_from_its_own_height() {
        let mut v = json!({
            "height": 1080,
            "type": {"display": 140, "title": 48, "body": 28},
            "comps": {
                "card": {"width": 720, "height": 400, "layers": [
                    {"type": "text", "text": "18.4%", "role": "display"}
                ]},
                "scene": {"width": 1920, "height": 1080, "layers": [
                    {"type": "text", "text": "HEAD", "role": "display"}
                ]}
            },
            "layers": []
        });
        apply(&mut v).unwrap();
        let card = v["comps"]["card"]["layers"][0]["size"].as_f64().unwrap();
        assert!(card < 90.0, "card inherited the film's 140px display: {card}");
        assert_eq!(v["comps"]["scene"]["layers"][0]["size"], 140.0);
    }

    #[test]
    fn a_type_role_carries_font_and_palette_colour() {
        let mut v = json!({
            "height": 1080,
            "palette": {"accent": "#e2c48a", "ink": "#f4f7ff"},
            "type": {
                "display": {"size": 140, "font": "Avenir Next Condensed", "weight": 800, "color": "accent"},
                "title": {"size": 48, "font": "Avenir Next", "color": "ink"}
            },
            "layers": [
                {"type": "text", "text": "18.4%", "role": "display"},
                {"type": "text", "text": "GROSS", "role": "title", "color": "#ffffff"}
            ]
        });
        apply(&mut v).unwrap();
        assert_eq!(v["layers"][0]["font"], "Avenir Next Condensed");
        assert_eq!(v["layers"][0]["weight"], 800);
        assert_eq!(v["layers"][0]["color"], "#e2c48a");
        assert_eq!(v["layers"][1]["color"], "#ffffff");
        assert_eq!(v["fonts"]["display"], "Avenir Next Condensed");
    }

    #[test]
    fn a_shape_takes_a_named_surface() {
        let mut v = json!({
            "surface": {"card": {"fill": "#10161f", "stroke": {"color": "#e2c48a", "width": 1.5}, "radius": 18}},
            "layers": [{"type": "shape", "surface": "card", "items": [{"rect": [0, 0, 100, 80]}]}]
        });
        apply(&mut v).unwrap();
        assert_eq!(v["layers"][0]["items"][0]["radius"], 18.0);
        assert_eq!(v["layers"][0]["items"][0]["fill"], "#10161f");
        assert!(v["layers"][0].get("surface").is_none());
    }

    #[test]
    fn an_image_without_fx_takes_the_kind_treat() {
        let mut v = json!({
            "image": {"treat": ["grain", {"effect": "vignette", "strength": 0.3}]},
            "layers": [{"type": "image", "src": "a.jpg"}]
        });
        apply(&mut v).unwrap();
        assert_eq!(v["layers"][0]["fx"][0]["effect"], "grain");
        assert_eq!(v["layers"][0]["fx"][1]["strength"], 0.3);
    }

    #[test]
    fn space_tokens_fill_a_surface_radius() {
        let mut v = json!({
            "space": {"radius": 18},
            "surface": {"card": {"fill": "#10161f", "radius": "radius"}},
            "layers": [{"type": "shape", "surface": "card", "items": [{"rect": [0, 0, 100, 80]}]}]
        });
        apply(&mut v).unwrap();
        assert_eq!(v["layers"][0]["items"][0]["radius"], 18.0);
    }

    #[test]
    fn motion_fills_an_enter_that_omitted_duration() {
        let mut v = json!({
            "motion": {"enter": {"dur": 0.4, "ease": "decel"}},
            "layers": [{"type": "shape", "enter": {"preset": "slide_up"}, "items": [{"rect": [0, 0, 10, 10]}]}]
        });
        apply(&mut v).unwrap();
        assert_eq!(v["layers"][0]["enter"]["dur"], 0.4);
        assert_eq!(v["layers"][0]["enter"]["ease"], "expo.out");
        assert!(v["layers"][0].get("motion").is_none());
    }

    #[test]
    fn on_parents_a_layer_in_front_of_the_plate() {
        let mut v = json!({"layers": [
            {"type": "shape", "id": "plate", "three_d": true, "items": [{"rect": [0, 0, 100, 80]}]},
            {"type": "text", "text": "18.4%", "on": "plate", "pos": [50, 40]}
        ]});
        apply(&mut v).unwrap();
        assert_eq!(v["layers"][1]["parent"], "plate");
        assert_eq!(v["layers"][1]["three_d"], true);
        assert_eq!(v["layers"][1]["pos"][2], -4.0);
        assert!(v["layers"][1].get("on").is_none());
    }
}
