//! Blocks: reusable parametrised sub-compositions. A layer `{"type": "precomp", "block": "lower_third", "vars":
//! {"name": "Ada"}}` instantiates `blocks/lower_third.json` (beside the composition) or a built-in of that name with
//! its `$variables` replaced. The same works for a nested comp of the file: `{"type": "precomp", "comp": "card",
//! "vars": {...}}` gets its own copy with those variables. The expansion happens on the raw JSON, before anything
//! else is read, so every later stage sees ordinary nested compositions.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::{Map, Value};

macro_rules! builtin {
    ($job:literal, $name:literal) => {
        ($name, include_str!(concat!("../../blocks/", $job, "/", $name, ".json")))
    };
}

/// The engine's own blocks, compiled in from `crates/comp/blocks/<job>/`. Anything found on a search path replaces
/// a built-in of the same name.
const BUILTIN: &[(&str, &str)] = &[
    builtin!("stage", "lower_third"),
    builtin!("card", "title_card"),
    builtin!("card", "stat"),
    builtin!("card", "holo_slab"),
    builtin!("stage", "connector"),
    builtin!("stage", "callout"),
    builtin!("card", "stamp"),
    builtin!("teach", "compare_two"),
    builtin!("teach", "definition"),
    builtin!("teach", "step_chain"),
    builtin!("mark", "icon_chart"),
    builtin!("mark", "icon_lock"),
    builtin!("mark", "icon_warn"),
    builtin!("mark", "icon_bank"),
    builtin!("mark", "icon_doc"),
    builtin!("mark", "icon_arrow"),
    builtin!("stage", "ticker"),
    builtin!("card", "security_card"),
    builtin!("teach", "claim_bench"),
    builtin!("card", "holo_ticket"),
    builtin!("mark", "punched_icon"),
    builtin!("card", "ridgeline_flip"),
    builtin!("stage", "card_orbit"),
    builtin!("stage", "hud_reticle"),
    builtin!("teach", "inspect_board"),
    builtin!("stage", "stage"),
    builtin!("stage", "deck_3d"),
    builtin!("mark", "reaction"),
    builtin!("teach", "term_card"),
    builtin!("teach", "scene_bug"),
    builtin!("mark", "icon"),
    builtin!("card", "panel"),
];

/// A block on disk or compiled in.
pub struct BlockSrc {
    pub name: String,
    pub job: String,
    pub source: String,
}

/// Environment variable holding extra block folders, `:`-separated.
pub const ENV_PATH: &str = "TURBOMOTION_BLOCK_PATH";

/// Every block, lowest priority first: the built-ins, `TURBOMOTION_BLOCK_PATH`, the composition's `"block_path"`
/// (relative to `dir`, the composition's folder), then the project's own `blocks/`. Later wins. There is no
/// searching of parent folders: a block comes from a folder somebody named.
pub fn available(dir: &Path, declared: &[String]) -> Result<Vec<BlockSrc>> {
    let mut found: BTreeMap<String, BlockSrc> = BTreeMap::new();
    for (n, s) in BUILTIN {
        found.insert((*n).to_string(), BlockSrc { name: (*n).to_string(), job: "builtin".into(), source: (*s).to_string() });
    }
    for folder in search_dirs(dir, declared)? {
        load_tree(&folder, "library", &mut found);
    }
    load_tree(&dir.join("blocks"), "project", &mut found);
    Ok(found.into_values().collect())
}

fn search_dirs(dir: &Path, declared: &[String]) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Ok(list) = std::env::var(ENV_PATH) {
        for entry in list.split(':').filter(|e| !e.is_empty()) {
            let p = PathBuf::from(entry);
            ensure!(p.is_dir(), "{ENV_PATH} lists {entry:?}, which is not a folder; fix or unset the variable");
            out.push(p);
        }
    }
    for entry in declared {
        let p = if Path::new(entry).is_absolute() { PathBuf::from(entry) } else { dir.join(entry) };
        ensure!(p.is_dir(), "\"block_path\" lists {entry:?}, which is not a folder (looked for {}); paths are relative to the composition's root (its own folder unless it sets \"root\"), like \"fx_path\"", p.display());
        out.push(p);
    }
    Ok(out)
}

fn load_tree(dir: &Path, default_job: &str, found: &mut BTreeMap<String, BlockSrc>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            let job = p.file_name().and_then(|n| n.to_str()).unwrap_or(default_job);
            load_tree(&p, job, found);
            continue;
        }
        if p.extension().is_some_and(|x| x == "json") {
            if let (Some(n), Ok(s)) = (p.file_stem().and_then(|n| n.to_str()), fs::read_to_string(&p)) {
                let job = p.parent().and_then(|d| d.file_name()).and_then(|n| n.to_str()).unwrap_or(default_job);
                let job = if job == "blocks" { default_job } else { job };
                found.insert(n.to_string(), BlockSrc { name: n.to_string(), job: job.to_string(), source: s });
            }
        }
    }
}

pub fn expand(root: &mut Value, dir: &Path) -> Result<()> {
    let declared: Vec<String> = root.get("block_path").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
    let blocks = available(dir, &declared)?;
    let palette = root.get("palette").and_then(Value::as_object).cloned().unwrap_or_default();
    let mut counter = 0;
    for depth in 0..8 {
        let mut made: Vec<(String, Value)> = vec![];
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
                    let src = blocks.iter().find(|s| s.name == *b).ok_or_else(|| {
                        let names: Vec<&str> = blocks.iter().map(|s| s.name.as_str()).collect();
                        anyhow!("{where_} layer {i}: no block {b:?}; blocks are {} (add kit/blocks/<job>/{b}.json)", names.join(", "))
                    })?;
                    (b.clone(), serde_json::from_str(&src.source).with_context(|| format!("block {b:?} is not valid JSON"))?)
                } else {
                    let c = o.get("comp").and_then(Value::as_str).ok_or_else(|| anyhow!("{where_} layer {i}: a precomp with vars needs \"comp\" or \"block\""))?;
                    (c.to_string(), existing.get(c).cloned().ok_or_else(|| anyhow!("{where_} layer {i}: no comp {c:?} to give variables to"))?)
                };
                let given = o.get("vars").and_then(Value::as_object).cloned().unwrap_or_default();
                let mut vars = source.get("vars").and_then(Value::as_object).cloned().unwrap_or_default();
                for (k, v) in &palette {
                    if vars.contains_key(k) && !given.contains_key(k) {
                        vars.insert(k.clone(), v.clone());
                    }
                }
                for (k, v) in given {
                    ensure!(vars.contains_key(&k), "{where_} layer {i}: block {label:?} has no variable {k:?}; its variables are {}", vars.keys().cloned().collect::<Vec<_>>().join(", "));
                    vars.insert(k, v);
                }
                super::time::substitute_vars(&mut source, &vars).with_context(|| format!("{where_} layer {i}: block {label:?}"))?;
                if let Some(s) = source.as_object_mut() {
                    s.remove("vars");
                    s.remove("best_for");
                    s.remove("breaks");
                    s.remove("job");
                    s.remove("props");
                    s.remove("uses");
                    s.remove("effect");
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
            let names: Vec<String> = comps.keys().cloned().collect();
            for name in names {
                if let Some(layers) = comps.get_mut(&name).and_then(|c| c.get_mut("layers")) {
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
        let _ = depth;
    }
    bail!("blocks nest more than 8 deep (a block probably includes itself)")
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

    #[test]
    fn a_block_may_contain_another_block() {
        let v = json!({"dur": 5, "layers": [
            {"type": "precomp", "block": "compare_two", "vars": {"left_title": "NOW", "right_title": "LATER"}, "pos": [960, 540]}
        ]});
        let plan = Plan::from_value(v, std::path::Path::new(".")).unwrap();
        let names: Vec<&str> = plan.comps.keys().filter(|k| !k.is_empty()).map(String::as_str).collect();
        assert!(names.iter().any(|n| n.starts_with("compare_two#")), "{names:?}");
        assert!(names.iter().any(|n| n.starts_with("holo_slab#")), "inner holo_slab not expanded: {names:?}");
        assert!(names.iter().any(|n| n.starts_with("connector#")), "{names:?}");
    }

    #[test]
    fn the_builtin_blocks_load() {
        for name in ["ticker", "security_card", "claim_bench", "holo_ticket", "punched_icon", "ridgeline_flip", "card_orbit", "hud_reticle", "inspect_board", "stage", "deck_3d", "reaction", "term_card", "scene_bug", "icon", "panel"] {
            Plan::from_value(json!({"dur": 8, "layers": [{"type": "precomp", "block": name, "pos": [960, 540]}]}), std::path::Path::new("."))
                .unwrap_or_else(|e| panic!("{name}: {e:#}"));
        }
    }

    #[test]
    fn a_film_palette_fills_block_vars_the_instance_did_not_set() {
        let mut v = json!({"palette": {"accent": "#e2c48a"}, "layers": [
            {"type": "precomp", "block": "lower_third", "vars": {"name": "Ada", "role": "Maths"}}
        ]});
        crate::schema::tokens::publish_palette(&mut v);
        super::expand(&mut v, std::path::Path::new(".")).unwrap();
        let inner = v["comps"].as_object().unwrap().values().next().unwrap();
        let bar = inner["layers"].as_array().unwrap().iter().find(|l| l["id"] == "bar").unwrap();
        assert_eq!(bar["items"][0]["fill"], "#e2c48a");
    }

    #[test]
    fn the_block_index_names_every_builtin() {
        let index = include_str!("../../blocks/README.md");
        for (name, _) in super::BUILTIN {
            assert!(index.contains(&format!("`{name}`")), "crates/comp/blocks/README.md does not name {name}");
        }
    }
}
