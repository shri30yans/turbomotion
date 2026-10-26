//! `sdk new scene NAME`: write a typed scene folder (and optionally a probe film that renders only that scene).
//! Generic by design: the templates in `templates/scene/` know nothing about any project.

use std::{fs, path::{Path, PathBuf}};

use anyhow::{bail, ensure, Result};

const INDEX: &str = include_str!("../templates/scene/index.ts");
const TYPES: &str = include_str!("../templates/scene/types.ts");
const PROBE: &str = include_str!("../templates/scene/probe.ts");
const EFFECT_FILM: &str = include_str!("../templates/probe/film.ts");
const EFFECT_SHADER: &str = include_str!("../templates/probe/effect.sksl");

/// What a scaffold wrote and the command that renders it.
#[derive(Debug)]
pub struct Created {
    pub files: Vec<PathBuf>,
    pub render: String,
}

/// A folder name that is also a safe file stem: a lowercase letter, then lowercase letters, digits, `-` or `_`.
fn validate(name: &str) -> Result<()> {
    let mut chars = name.chars();
    let ok = chars.next().is_some_and(|c| c.is_ascii_lowercase()) && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    ensure!(ok, "scene name {name:?} must start with a lowercase letter and use only lowercase letters, digits, - and _");
    Ok(())
}

/// `chevron-tunnel` -> `chevronTunnel`, the scene's function; `ChevronTunnelOptions` is its options type.
fn camel(name: &str) -> String {
    let mut out = String::new();
    for (i, part) in name.split(['-', '_']).filter(|p| !p.is_empty()).enumerate() {
        let (head, tail) = part.split_at(1);
        out.push_str(&if i == 0 { head.to_owned() } else { head.to_uppercase() });
        out.push_str(tail);
    }
    out
}

fn fill(template: &str, name: &str) -> String {
    let function = camel(name);
    let type_name = format!("{}{}Options", function[..1].to_uppercase(), &function[1..]);
    template.replace("__FUNCTION__", &function).replace("__TYPE__", &type_name).replace("__NAME__", name)
}

/// Write `dir/name/{index.ts,types.ts}` and, with `probe`, `dir/probe_name.ts`. Nothing is written if any file already exists.
pub fn scene(name: &str, dir: &Path, probe: bool) -> Result<Created> {
    validate(name)?;
    let mut plan = vec![(dir.join(name).join("index.ts"), fill(INDEX, name)), (dir.join(name).join("types.ts"), fill(TYPES, name))];
    let probe_file = dir.join(format!("probe_{name}.ts"));
    if probe {
        plan.push((probe_file.clone(), fill(PROBE, name)));
    }
    let conflicts: Vec<String> = plan.iter().filter(|(path, _)| path.exists()).map(|(path, _)| path.display().to_string()).collect();
    if !conflicts.is_empty() {
        bail!("refusing to overwrite: {}", conflicts.join(", "));
    }
    fs::create_dir_all(dir.join(name))?;
    for (path, text) in &plan {
        fs::write(path, text)?;
    }
    let entry = if probe { probe_file.display().to_string() } else { "<a film that imports the scene>".into() };
    Ok(Created {
        files: plan.into_iter().map(|(path, _)| path).collect(),
        render: format!("turbomotion sdk render {entry} --out {name}.mp4 --sheet {name}.png --times 0.5,1 --sheet-only"),
    })
}

/// An effect name is an identifier (`@effect` allows letters, digits and underscores) that is not already an effect.
fn validate_effect(name: &str) -> Result<()> {
    let mut chars = name.chars();
    let ok = chars.next().is_some_and(|c| c.is_ascii_lowercase()) && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    ensure!(ok, "probe name {name:?} is the name of an effect: start with a lowercase letter, then lowercase letters, digits and _");
    let taken = turbomotion_comp::fxlib::Library::builtin()?.descriptors().iter().any(|d| d.name == name);
    ensure!(!taken, "{name:?} is already an effect (`turbomotion comp fx list`); pick another name");
    Ok(())
}

/// `sdk new probe NAME`: `dir/NAME/{film.ts, effects/NAME.sksl, blocks/}`. The film puts one text layer under the
/// effect, names both folders in `fx_path` and `block_path`, and renders as a contact sheet. Nothing is written if any file exists.
pub fn probe(name: &str, dir: &Path) -> Result<Created> {
    validate_effect(name)?;
    let root = dir.join(name);
    let plan = [(root.join("film.ts"), fill(EFFECT_FILM, name)), (root.join("effects").join(format!("{name}.sksl")), fill(EFFECT_SHADER, name)), (root.join("blocks").join(".gitkeep"), String::new())];
    let conflicts: Vec<String> = plan.iter().filter(|(path, _)| path.exists()).map(|(path, _)| path.display().to_string()).collect();
    ensure!(conflicts.is_empty(), "refusing to overwrite: {}", conflicts.join(", "));
    for (path, text) in &plan {
        fs::create_dir_all(path.parent().expect("every file is in a folder"))?;
        fs::write(path, text)?;
    }
    let (film, sheet) = (root.join("film.ts"), root.join("sheet.png"));
    Ok(Created {
        files: plan.into_iter().map(|(path, _)| path).collect(),
        render: format!("turbomotion sdk render {} --sheet {} --times 0.5,1.5 --sheet-only", film.display(), sheet.display()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_folder_safe_identifiers() {
        for good in ["vs", "chevron-tunnel", "scene_2"] { assert!(validate(good).is_ok(), "{good}"); }
        for bad in ["", "2fast", "Upper", "a/b", "..", "has space", "-x"] { assert!(validate(bad).is_err(), "{bad:?}"); }
    }

    #[test]
    fn the_function_name_is_camel_case_of_the_folder_name() {
        assert_eq!(camel("vs"), "vs");
        assert_eq!(camel("chevron-tunnel"), "chevronTunnel");
        assert_eq!(camel("big_sale-card"), "bigSaleCard");
    }
}
