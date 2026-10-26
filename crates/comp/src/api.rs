//! Library entry points for tools built on the engine (the TypeScript SDK): import SVG artwork, validate a composition
//! value the way `comp check` does, and inspect an evaluated frame. Nothing here reads outside the folder it is given.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::{
    check::{self, Finding},
    eval::Evaluator,
    plan::{Plan, ROOT},
};

/// A shape layer imported from SVG markup, with the importer's notes about anything it skipped.
pub struct Svg {
    pub layer: Value,
    pub warnings: Vec<String>,
}

/// Import SVG markup as a shape layer `id`, scaled to `fit` pixels wide (the `comp svg` importer).
pub fn svg_layer(markup: &str, id: &str, fit: f64) -> Result<Svg> {
    let imported = crate::tools::svg::import(markup, id, fit)?;
    Ok(Svg { layer: imported.layer, warnings: imported.warnings })
}

/// Plan `value` (asset paths resolve against `dir` and the value's `root`) and evaluate it at the moments `comp check`
/// samples, so a mistake in any layer, expression, font or reference surfaces now. Returns the lint findings.
pub fn validate(value: Value, dir: &Path) -> Result<Vec<Finding>> {
    let plan = Plan::from_value(value, dir)?;
    let evaluator = Evaluator::new(&plan);
    for t in check::frame_times(&plan, 24) {
        evaluator.frame(ROOT, t).with_context(|| format!("evaluating the composition at {t:.2}s"))?;
    }
    check::lint::run(&plan)
}

/// The evaluated frame at `t` seconds, or one layer of it (`"shot/layer"` reaches into nested compositions).
/// `layer` is an id or a path (`shot/id`); a bare id resolves through `lookup`.
pub fn inspect(value: Value, dir: &Path, t: f64, layer: Option<&str>) -> Result<Value> {
    let plan = Plan::from_value(value, dir)?;
    let frame = Evaluator::new(&plan).frame(ROOT, t)?;
    let Some(path) = layer else { return Ok(serde_json::to_value(&frame)?) };
    let (containing, selected) = crate::inspect::select(&frame, &crate::lookup::resolve(&plan, path)?)?;
    let mut value = serde_json::to_value(containing)?;
    value["layers"] = serde_json::json!([selected]);
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn svg_markup_becomes_a_shape_layer() {
        let svg = svg_layer(r##"<svg viewBox="0 0 10 10"><rect width="10" height="10" fill="#f00"/></svg>"##, "box", 100.0).unwrap();
        assert_eq!(svg.layer["id"], "box");
        assert_eq!(svg.layer["type"], "shape");
        assert!(svg_layer("<svg/>", "x", 10.0).is_err());
    }

    #[test]
    fn validation_reports_bad_layers_and_inspection_reaches_inside_precomps() {
        let good = json!({"width":200,"height":100,"dur":2,"comps":{"card":{"dur":2,"layers":[{"id":"disc","type":"solid","color":"#fff"}]}},
            "layers":[{"id":"s1","type":"precomp","comp":"card"}]});
        assert!(validate(good.clone(), Path::new(".")).unwrap().iter().all(|f| f.severity != check::Severity::Error));
        let frame = inspect(good, Path::new("."), 0.5, Some("s1/disc")).unwrap();
        assert_eq!(frame["layers"][0]["id"], "disc");
        let bad = json!({"dur":2,"layers":[{"id":"x","type":"nonsense"}]});
        assert!(validate(bad, Path::new(".")).is_err());
    }
}
