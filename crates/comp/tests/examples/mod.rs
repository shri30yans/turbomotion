//! Finding the runnable examples in the catalogue and in the reference pages, and turning each into a whole film.

pub mod fixtures;
pub mod plugins;

use std::{collections::BTreeMap, fs, path::Path};

use serde_json::{json, Value};

/// One example as a film `comp check` can run.
pub struct Example {
    pub name: String,
    pub film: Value,
    /// Rules this example may trip: documented, deliberate (empty for nearly all).
    pub allowed: Vec<&'static str>,
}

fn base(layers: Vec<Value>) -> Value {
    json!({"width": 1920, "height": 1080, "fps": 30, "dur": 3, "bg": "#1a1d29", "comps": {"inner": {"dur": 2, "layers": []}}, "layers": layers})
}

/// What the catalogue declares about effects: which inputs read another layer (`kind: layer`). The harness supplies a
/// source layer for exactly those, and for nothing else, so an example that forgets or misnames one still fails.
pub struct Declared {
    layer_inputs: BTreeMap<String, Vec<String>>,
    /// The catalogue's own example of each layer type (`camera`, `mesh`): the context a fragment is placed in.
    layer_examples: BTreeMap<String, Value>,
}

impl Declared {
    pub fn from(catalogue: &Value) -> Declared {
        let layer_inputs = catalogue["effects"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|e| {
                let names = e["inputs"].as_array().into_iter().flatten().filter(|i| i["kind"] == "layer").filter_map(|i| i["name"].as_str().map(str::to_string)).collect();
                (e["name"].as_str().expect("an effect name").to_string(), names)
            })
            .collect();
        let layer_examples = catalogue["layers"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|l| Some((l["name"].as_str()?.to_string(), l["example"].clone())))
            .collect();
        Declared { layer_inputs, layer_examples }
    }

    /// A layer example is a fragment of a film. The catalogue documents what a fragment needs beside it: a 3D layer is
    /// "projected through" a `camera`, and a `light` shades "3D layers". Where the fragment needs either and the film
    /// has none, place the catalogue's own example of that layer type beside it (a `mesh` for a light, a `camera`
    /// for 3D). Nothing else is added.
    fn in_context(&self, mut layers: Vec<Value>) -> Value {
        let has = |layers: &[Value], f: &dyn Fn(&Value) -> bool| layers.iter().any(f);
        let is_3d = |l: &Value| l["three_d"] == true;
        let is_type = |t: &'static str| move |l: &Value| l["type"] == t;
        if has(&layers, &is_type("light")) && !has(&layers, &is_3d) {
            layers.insert(0, self.layer_examples["mesh"].clone());
        }
        if has(&layers, &is_3d) && !has(&layers, &is_type("camera")) {
            layers.push(self.layer_examples["camera"].clone());
        }
        base(layers)
    }

    /// Wrap an example in the film it belongs to, by its shape; an example of no known shape is an error.
    pub fn classify(&self, name: &str, v: &Value) -> Result<Value, String> {
        let Some(o) = v.as_object() else { return Err(format!("{name}: cannot tell what kind of example {v}")) };
        if o.contains_key("composition") {
            return Ok(o["composition"].clone());
        }
        if o.contains_key("timeline") {
            let mut film = base(vec![]);
            film["dur"] = json!(12);
            film.as_object_mut().unwrap().extend(o.clone());
            return Ok(film);
        }
        // a layer names its kind in `type`; a composition's `type` is its text scale (an object)
        if o.get("type").is_some_and(Value::is_string) {
            return Ok(self.in_context(vec![v.clone()]));
        }
        if let Some(effect) = o.get("effect").and_then(Value::as_str) {
            let mut layers = vec![json!({"type": "solid", "color": "#2a4d9b", "size": [960, 540], "pos": [960, 540], "fx": [v]})];
            // a hidden noise texture is the picture an effect reads when it takes another layer (a map, a noise field)
            for input in self.layer_inputs.get(effect).into_iter().flatten() {
                if let Some(id) = o.get(input).and_then(Value::as_str) {
                    layers.push(json!({"type": "texture", "shader": "noise", "id": id, "hidden": true}));
                }
            }
            return Ok(base(layers));
        }
        if o.contains_key("enter") || o.contains_key("exit") {
            let mut layer = json!({"type": "solid", "color": "#2a4d9b", "size": [960, 540], "pos": [960, 540]});
            layer.as_object_mut().unwrap().extend(o.clone());
            return Ok(base(vec![layer]));
        }
        if o.contains_key("layers") || o.contains_key("width") {
            return Ok(v.clone());
        }
        Err(format!("{name}: cannot tell what kind of example this is (keys: {})", o.keys().cloned().collect::<Vec<_>>().join(", ")))
    }
}

fn walk(path: &str, v: &Value, out: &mut Vec<(String, Value)>) {
    match v {
        Value::Object(o) => {
            for (k, x) in o {
                let here = format!("{path}.{k}");
                if k == "example" && x.is_object() {
                    out.push((here.clone(), x.clone()));
                }
                if k == "composition" && !path.contains("runnable_property_examples") {
                    continue;
                }
                walk(&here, x, out);
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                let label = x.get("name").or_else(|| x.get("preset")).and_then(Value::as_str).map_or(i.to_string(), str::to_string);
                walk(&format!("{path}[{label}]"), x, out);
            }
        }
        _ => {}
    }
}

fn finish(declared: &Declared, found: Vec<(String, Value)>) -> Vec<Example> {
    found
        .into_iter()
        .map(|(name, v)| {
            let film = declared.classify(&name, &v).unwrap_or_else(|e| panic!("{e}"));
            fixtures::ensure(&film);
            Example { name, film, allowed: vec![] }
        })
        .collect()
}

/// Every `example` object anywhere in the catalogue, every runnable property example, and every text preset.
pub fn collect_catalogue(all: &Value) -> Vec<Example> {
    let mut found = Vec::new();
    walk("catalogue", all, &mut found);
    for (i, p) in all["runnable_property_examples"].as_array().into_iter().flatten().enumerate() {
        found.push((format!("catalogue.runnable_property_examples[{}]", p["name"].as_str().unwrap_or(&i.to_string())), json!({"composition": p["composition"]})));
    }
    for p in all["text_presets"].as_array().into_iter().flatten() {
        let name = p.as_str().map(str::to_string).or_else(|| p["preset"].as_str().map(str::to_string)).expect("a text preset name");
        found.push((format!("catalogue.text_presets[{name}]"), json!({"type": "text", "text": "Preset", "font": "Helvetica Neue", "size": 120, "ui": true, "pos": [960, 540], "preset": {"name": name, "at": 0.2}})));
    }
    // the same examples again with `composition` entries expanded by the walker are duplicates: keep the first of each name
    let mut seen = std::collections::BTreeSet::new();
    finish(&Declared::from(all), found.into_iter().filter(|(n, _)| seen.insert(n.clone())).collect())
}

/// Every fenced `json` block of `docs/reference/*.md`.
pub fn collect_docs() -> Vec<Example> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/reference");
    let mut found = Vec::new();
    let mut pages: Vec<_> = fs::read_dir(&dir).expect("docs/reference").flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "md")).collect();
    pages.sort();
    for page in pages {
        let text = fs::read_to_string(&page).unwrap();
        let mut block: Option<(usize, String)> = None;
        for (n, line) in text.lines().enumerate() {
            match (&mut block, line) {
                (None, "```json") => block = Some((n + 2, String::new())),
                (Some((start, body)), "```") => {
                    let v: Value = serde_json::from_str(body).unwrap_or_else(|e| panic!("{}:{start}: invalid JSON in a ```json block: {e}", page.display()));
                    found.push((format!("{}:{start}", page.file_name().unwrap().to_string_lossy()), v));
                    block = None;
                }
                (Some((_, body)), _) => {
                    body.push_str(line);
                    body.push('\n');
                }
                _ => {}
            }
        }
    }
    finish(&Declared::from(&turbomotion_comp::catalogue::all()), found)
}
