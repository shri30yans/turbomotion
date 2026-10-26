//! `turbomotion parity init-effect <name>`: the first five minutes of the porting procedure (BUILD_PLAN section 4),
//! done mechanically: the shader file with its header, a probe film, the spec template, a test stub and the ledger row.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use super::{
    check::slug,
    ledger::{Ledger, EFFECT_COLUMNS},
};

fn write_new(path: &Path, text: &str) -> Result<()> {
    if path.exists() {
        bail!("{} already exists; init-effect never overwrites", path.display());
    }
    fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))?;
    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

const SKSL: &str = "// @effect NAME
// @summary TODO: one sentence, in After Effects' words.
// @param amount float 0 0 1 -- TODO: replace with the AE parameters (name, unit, default, range)
half4 main(float2 xy) {
    float4 s = straight(xy);
    return premul(s.rgb, s.a);
}
";

const TEST: &str = "//! Tests for the `NAME` effect (BUILD_PLAN 5.3): identity, one numeric behaviour, determinism, bounds.

use super::testkit::render;
use serde_json::json;

fn comp(fx: serde_json::Value) -> serde_json::Value {
    json!({\"width\": 64, \"height\": 32, \"dur\": 1, \"layers\": [{\"type\": \"solid\", \"color\": \"#ffffff\", \"size\": [8, 8], \"pos\": [32, 16], \"fx\": [fx]}]})
}

#[test]
fn NAME_with_neutral_parameters_returns_the_input_byte_for_byte() {
    let plain = render(json!({\"width\": 64, \"height\": 32, \"dur\": 1, \"layers\": [{\"type\": \"solid\", \"color\": \"#ffffff\", \"size\": [8, 8], \"pos\": [32, 16]}]}));
    assert_eq!(render(comp(json!({\"effect\": \"NAME\"}))), plain);
}

#[test]
fn NAME_renders_the_same_twice() {
    let fx = json!({\"effect\": \"NAME\", \"amount\": 1});
    assert_eq!(render(comp(fx.clone())), render(comp(fx)));
}

#[test]
#[ignore = \"TODO: assert one numeric behaviour from parity/specs/NAME.md, then remove this attribute\"]
fn NAME_numeric_behaviour() {
    unimplemented!(\"one pixel value or position, from the spec\");
}
";

fn spec(name: &str, ae: &str) -> String {
    format!(
        "# {ae} (`{name}`)\n\nWritten in my own words from AE documentation and `tmp/clones/effectcraft/docs/effects.md` (BUILD_PLAN section 4, step 2).\n\n\
## Parameters\n\n| name | unit | default | range | source | notes |\n| --- | --- | --- | --- | --- | --- |\n| TODO | | | | | mark `UNSURE` where sources disagree; write both |\n\n\
## Formula\n\nTODO pseudo-code.\n\n## Edge cases\n\nTODO.\n\n## Bounds\n\nGrows bounds: TODO (yes/no, by how much).\n\n## Colour\n\nLinear light: TODO (yes/no).\n"
    )
}

fn probe(name: &str) -> String {
    format!("{{\n  \"width\": 640, \"height\": 360, \"fps\": 30, \"dur\": 1,\n  \"bg\": \"#101014\",\n  \"layers\": [\n    {{\"type\": \"text\", \"text\": \"PROBE\", \"font\": \"Helvetica Neue\", \"weight\": 800, \"size\": 120, \"color\": \"#ffffff\", \"pos\": [320, 180], \"align\": \"center\", \"ui\": true,\n     \"fx\": [{{\"effect\": \"{name}\"}}]}}\n  ]\n}}\n")
}

/// Create the files and the ledger row; returns what was written. `ae` names the row to take over (a `todo` row), or a new
/// row is appended in `category`.
pub fn scaffold(ledger_dir: &Path, name: &str, ae: Option<&str>, category: &str) -> Result<Vec<PathBuf>> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') || name.starts_with(|c: char| c.is_ascii_digit()) {
        bail!("effect name {name:?} must be snake_case ascii (a-z, 0-9, _)");
    }
    let mut ledger = Ledger::load(ledger_dir)?;
    let root = ledger.root();
    let shader = root.join("crates/comp/src/fxlib/builtin").join(format!("{name}.sksl"));
    let tests_mod = root.join("crates/comp/src/render/mod.rs");
    let tests_file = root.join(format!("crates/comp/src/render/fx_{name}_tests.rs"));
    let e = &mut ledger.effects;
    let ae_name = ae.unwrap_or(name);
    let existing = e.rows.iter().position(|r| e.val(r, "name") == ae_name);
    match existing {
        Some(i) => {
            let status = e.val(&e.rows[i], "status").to_string();
            if status != "todo" {
                bail!("the row {ae_name:?} has status {status}; init-effect takes over only `todo` rows");
            }
        }
        None if ae.is_some() => bail!("no effect row named {ae_name:?} in effects.tsv; use the exact AE name from section 10, or omit --ae to add a new row"),
        None => {}
    }
    let slug = slug(ae_name);
    let files = [
        (shader.clone(), SKSL.replace("NAME", name)),
        (ledger_dir.join("probes").join(format!("{slug}.json")), probe(name)),
        (ledger_dir.join("specs").join(format!("{slug}.md")), spec(name, ae_name)),
        (tests_file.clone(), TEST.replace("NAME", name)),
    ];
    for (p, _) in &files {
        if p.exists() {
            bail!("{} already exists; init-effect never overwrites", p.display());
        }
    }
    let mod_src = fs::read_to_string(&tests_mod).with_context(|| format!("reading {}", tests_mod.display()))?;
    let marker = "#[cfg(test)]\nmod fx_tests;\n";
    if !mod_src.contains(marker) {
        bail!("{}: expected the line pair `#[cfg(test)] mod fx_tests;` to add the test module after", tests_mod.display());
    }
    for (p, text) in &files {
        write_new(p, text)?;
    }
    fs::write(&tests_mod, mod_src.replacen(marker, &format!("{marker}#[cfg(test)]\nmod fx_{name}_tests;\n"), 1))?;
    match existing {
        Some(i) => {
            let (status, engine, test) = (e.col("status"), e.col("engine"), e.col("test"));
            e.rows[i].cells[status] = "in_progress".into();
            e.rows[i].cells[engine] = name.into();
            e.rows[i].cells[test] = format!("{name}_numeric_behaviour");
        }
        None => {
            let mut cells = vec!["-".to_string(); EFFECT_COLUMNS.len()];
            for (col, v) in [("name", ae_name), ("category", category), ("recipe_class", "A"), ("tier", "T1"), ("status", "in_progress"), ("grows_bounds", "?"), ("linear", "?"), ("engine", name)] {
                cells[e.col(col)] = v.to_string();
            }
            let test = format!("{name}_numeric_behaviour");
            let t = e.col("test");
            cells[t] = test;
            e.push(cells);
        }
    }
    e.save()?;
    let mut written: Vec<PathBuf> = files.into_iter().map(|(p, _)| p).collect();
    written.push(tests_mod);
    written.push(e.path.clone());
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::{super::ledger::fixture, *};

    fn project(tag: &str, effects: &[&str]) -> PathBuf {
        let dir = fixture::write(tag, &[], effects);
        let root = dir.parent().unwrap();
        fs::create_dir_all(root.join("crates/comp/src/render")).unwrap();
        fs::create_dir_all(root.join("crates/comp/src/fxlib/builtin")).unwrap();
        fs::write(root.join("crates/comp/src/render/mod.rs"), "mod res;\n\n#[cfg(test)]\nmod fx_tests;\n#[cfg(test)]\nmod history_tests;\n").unwrap();
        dir
    }

    #[test]
    fn it_scaffolds_every_file_and_takes_over_the_todo_row() {
        let dir = project("init", &["Box Blur\tBlur\tB\tT0\ttodo\t?\t?\t-\t-\t-"]);
        let written = scaffold(&dir, "box_blur", Some("Box Blur"), "Blur").unwrap();
        let root = dir.parent().unwrap();
        assert!(fs::read_to_string(root.join("crates/comp/src/fxlib/builtin/box_blur.sksl")).unwrap().starts_with("// @effect box_blur\n"));
        assert!(fs::read_to_string(dir.join("probes/box_blur.json")).unwrap().contains("\"effect\": \"box_blur\""));
        assert!(dir.join("specs/box_blur.md").exists());
        assert!(fs::read_to_string(root.join("crates/comp/src/render/fx_box_blur_tests.rs")).unwrap().contains("fn box_blur_renders_the_same_twice()"));
        assert!(fs::read_to_string(root.join("crates/comp/src/render/mod.rs")).unwrap().contains("mod fx_tests;\n#[cfg(test)]\nmod fx_box_blur_tests;\n"));
        let row = Ledger::load(&dir).unwrap();
        assert_eq!(row.effects.rows.len(), 1, "the todo row is reused, not duplicated");
        assert_eq!(row.effects.val(&row.effects.rows[0], "status"), "in_progress");
        assert_eq!(row.effects.val(&row.effects.rows[0], "engine"), "box_blur");
        assert_eq!(written.len(), 6);
    }

    #[test]
    fn it_appends_a_row_for_an_unlisted_effect_and_refuses_to_overwrite() {
        let dir = project("init-new", &[]);
        scaffold(&dir, "wobble", None, "Distort").unwrap();
        let l = Ledger::load(&dir).unwrap();
        assert_eq!(l.effects.val(&l.effects.rows[0], "category"), "Distort");
        let again = scaffold(&dir, "wobble", None, "Distort").unwrap_err().to_string();
        assert!(again.contains("in_progress"), "a started row is not taken over twice: {again}");
        let l = Ledger::load(&dir).unwrap();
        assert_eq!(l.effects.rows.len(), 1, "the refused call added nothing");
        // a stray file blocks a fresh row, and nothing is overwritten
        let root = dir.parent().unwrap();
        fs::write(root.join("crates/comp/src/fxlib/builtin/stray.sksl"), "keep").unwrap();
        assert!(scaffold(&dir, "stray", None, "Distort").unwrap_err().to_string().contains("already exists"));
        assert_eq!(fs::read_to_string(root.join("crates/comp/src/fxlib/builtin/stray.sksl")).unwrap(), "keep");
    }

    #[test]
    fn it_rejects_bad_names_and_unknown_or_started_rows() {
        let dir = project("init-bad", &["Glow\tStylize\t-\tT1\thave\t?\t?\t-\tglow\t-"]);
        assert!(scaffold(&dir, "Bad Name", None, "x").is_err());
        assert!(scaffold(&dir, "glow2", Some("Nope"), "x").unwrap_err().to_string().contains("no effect row named"));
        assert!(scaffold(&dir, "glow2", Some("Glow"), "x").unwrap_err().to_string().contains("only `todo` rows"));
    }
}
