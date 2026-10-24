//! Blocks: reusable parametrised sub-compositions. A layer `{"type": "precomp", "block": "lower_third", "vars":
//! {"name": "Ada"}}` instantiates `blocks/lower_third.json` (beside the composition) or a built-in of that name with
//! its `$variables` replaced. The same works for a nested comp of the file: `{"type": "precomp", "comp": "card",
//! "vars": {...}}` gets its own copy with those variables. The expansion happens on the raw JSON, before anything
//! else is read, so every later stage sees ordinary nested compositions.

use std::{fs, path::Path};

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::{Map, Value};

const BUILTIN: &[(&str, &str)] = &[
    ("lower_third", include_str!("../blocks/lower_third.json")),
    ("title_card", include_str!("../blocks/title_card.json")),
    ("stat", include_str!("../blocks/stat.json")),
];

/// `(name, source)` of every block available beside `dir` and built in; a project file replaces a built-in.
pub fn available(dir: &Path) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = BUILTIN.iter().map(|(n, s)| ((*n).to_string(), (*s).to_string())).collect();
    if let Ok(rd) = fs::read_dir(dir.join("blocks")) {
        for e in rd.filter_map(Result::ok) {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "json") {
                if let (Some(n), Ok(s)) = (p.file_stem().and_then(|n| n.to_str()), fs::read_to_string(&p)) {
                    found.retain(|(k, _)| k != n);
                    found.push((n.to_string(), s));
                }
            }
        }
    }
    found.sort();
    found
}

pub fn expand(root: &mut Value, dir: &Path) -> Result<()> {
    let blocks = available(dir);
    let mut made: Vec<(String, Value)> = vec![];
    let mut counter = 0;
    let existing: Map<String, Value> = root.get("comps").and_then(Value::as_object).cloned().unwrap_or_default();

    let mut visit = |layers: &mut Value, where_: &str| -> Result<()> {
        let Some(list) = layers.as_array_mut() else { return Ok(()) };
        for (i, layer) in list.iter_mut().enumerate() {
            let Some(o) = layer.as_object_mut() else { continue };
            if o.get("type").and_then(Value::as_str) != Some("precomp") {
                continue;
            }
            let block = o.get("block").and_then(Value::as_str).map(String::from);
            let has_vars = o.get("vars").and_then(Value::as_object).is_some_and(|v| !v.is_empty());
            if block.is_none() && !has_vars {
                continue;
            }
            let (label, mut source): (String, Value) = if let Some(b) = &block {
                let (_, text) = blocks.iter().find(|(n, _)| n == b).ok_or_else(|| {
                    let names: Vec<&str> = blocks.iter().map(|(n, _)| n.as_str()).collect();
                    anyhow!("{where_} layer {i}: no block {b:?}; blocks are {} (add blocks/{b}.json)", names.join(", "))
                })?;
                (b.clone(), serde_json::from_str(text).with_context(|| format!("block {b:?} is not valid JSON"))?)
            } else {
                let c = o.get("comp").and_then(Value::as_str).ok_or_else(|| anyhow!("{where_} layer {i}: a precomp with vars needs \"comp\" or \"block\""))?;
                (c.to_string(), existing.get(c).cloned().ok_or_else(|| anyhow!("{where_} layer {i}: no comp {c:?} to give variables to"))?)
            };
            let given = o.get("vars").and_then(Value::as_object).cloned().unwrap_or_default();
            let mut vars = source.get("vars").and_then(Value::as_object).cloned().unwrap_or_default();
            for (k, v) in given {
                ensure!(vars.contains_key(&k), "{where_} layer {i}: block {label:?} has no variable {k:?}; its variables are {}", vars.keys().cloned().collect::<Vec<_>>().join(", "));
                vars.insert(k, v);
            }
            super::time::substitute_vars(&mut source, &vars).with_context(|| format!("{where_} layer {i}: block {label:?}"))?;
            if let Some(s) = source.as_object_mut() {
                s.remove("vars");
            }
            counter += 1;
            let name = format!("{label}#{counter}");
            made.push((name.clone(), source));
            o.remove("block");
            o.remove("vars");
            o.insert("comp".into(), Value::String(name));
        }
        Ok(())
    };

    if let Some(layers) = root.get_mut("layers") {
        visit(layers, "root")?;
    }
    if let Some(comps) = root.get_mut("comps").and_then(Value::as_object_mut) {
        for (name, comp) in comps.iter_mut() {
            if let Some(layers) = comp.get_mut("layers") {
                visit(layers, &format!("comp {name:?}"))?;
            }
        }
    }
    if made.is_empty() {
        return Ok(());
    }
    let Some(o) = root.as_object_mut() else { bail!("the composition must be a JSON object") };
    let comps = o.entry("comps").or_insert_with(|| Value::Object(Map::new()));
    let comps = comps.as_object_mut().ok_or_else(|| anyhow!("\"comps\" must be an object"))?;
    for (name, c) in made {
        comps.insert(name, c);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::plan::Plan;
    use serde_json::json;

    #[test]
    fn a_block_is_instantiated_with_its_variables_and_unknown_ones_are_rejected() {
        let v = json!({"dur": 6, "layers": [
            {"type": "precomp", "block": "lower_third", "vars": {"name": "Grace Hopper", "role": "Rear Admiral"}, "pos": [600, 900], "start": 1},
            {"type": "precomp", "block": "stat", "vars": {"to": 42, "label": "answers"}, "pos": [1400, 400]}
        ]});
        let plan = Plan::from_value(v, std::path::Path::new(".")).unwrap();
        let names: Vec<&String> = plan.comps.keys().filter(|k| !k.is_empty()).collect();
        assert_eq!(names.len(), 2, "{names:?}");
        let ev = crate::eval::Evaluator::new(&plan);
        let f = ev.frame(crate::plan::ROOT, 2.0).unwrap();
        let crate::eval::Content::Precomp { frame: Some(inner) } = &f.layers[0].content else { panic!("block frame") };
        let texts: Vec<String> = inner.layers.iter().filter_map(|l| if let crate::eval::Content::Text(t) = &l.content { Some(t.text.clone()) } else { None }).collect();
        assert_eq!(texts, vec!["Grace Hopper", "Rear Admiral"]);
        let e = format!("{:#}", Plan::from_value(json!({"layers": [{"type": "precomp", "block": "lower_third", "vars": {"nmae": "x"}}]}), std::path::Path::new(".")).err().unwrap());
        assert!(e.contains("no variable \"nmae\"") && e.contains("name"), "{e}");
        let e = format!("{:#}", Plan::from_value(json!({"layers": [{"type": "precomp", "block": "lower_thrd"}]}), std::path::Path::new(".")).err().unwrap());
        assert!(e.contains("lower_third"), "{e}");
    }
}
