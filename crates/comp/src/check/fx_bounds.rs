//! `fx-needs-bounds`: in layer space a plugin's output can only reach as far as its picture, which is the layer's box grown
//! by what each effect says it needs. A plugin that does not say (`// @bounds`) gets no room, so anything it spreads or
//! moves past the layer's edge is cut off. Say so, naming the plugin.

use super::{Finding, Severity};
use crate::{
    fxlib::{Bounds, FxOp},
    plan::Plan,
};

pub(super) fn run(plan: &Plan, out: &mut Vec<Finding>) -> anyhow::Result<()> {
    for (name, comp) in &plan.comps {
        for l in comp.layers.iter().filter(|l| l.in_layer_space) {
            for f in &l.fx {
                let FxOp::Plugin(p) = &f.op else { continue };
                if plan.fx.get(&p.name)?.bounds != Bounds::Undeclared {
                    continue;
                }
                let label = if name.is_empty() { l.label.clone() } else { format!("{} in comp {name:?}", l.label) };
                out.push(Finding {
                    severity: Severity::Warn,
                    rule: "fx-needs-bounds",
                    layer: Some(label),
                    t: None,
                    message: format!("effect {:?} does not declare how far its output reaches past its input (no `// @bounds`), so in layer space whatever it spreads or moves beyond the layer's own box is cut off", p.name),
                    fix: format!("add `// @bounds none` to {:?} if its output stays inside the layer, or `// @bounds expand=<pixels>` (an expression of its float parameters) if it reaches past it", p.name),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;

    use super::*;

    const GHOST: &str = "// @effect ghost\n// @summary Smears sideways.\n// @param reach float 10 0 100 -- px\n";
    const BODY: &str = "half4 main(float2 xy) { return image.eval(xy - float2(reach, 0.0)) * half(resolution.x * 0.0 + time * 0.0 + 1.0); }\n";

    fn findings(bounds_line: &str, space: &str) -> Vec<Finding> {
        let dir = std::env::temp_dir().join(format!("tm-fxb-{}-{}", std::process::id(), bounds_line.len() + space.len()));
        fs::create_dir_all(dir.join("effects")).unwrap();
        fs::write(dir.join("effects/ghost.sksl"), format!("{GHOST}{bounds_line}{BODY}")).unwrap();
        let value = json!({"width": 64, "height": 48, "dur": 1, "effect_space": space, "layers": [{"type": "solid", "color": "#fff", "size": [8, 8], "fx": [{"effect": "ghost"}]}]});
        let plan = Plan::from_value(value, &dir).unwrap();
        let mut out = vec![];
        run(&plan, &mut out).unwrap();
        fs::remove_dir_all(&dir).ok();
        out
    }

    #[test]
    fn an_undeclared_plugin_in_layer_space_gets_a_warning_that_names_it_and_the_fix() {
        let f = findings("", "layer");
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].rule, "fx-needs-bounds");
        assert!(f[0].message.contains("\"ghost\"") && f[0].fix.contains("// @bounds expand="), "{}", f[0]);
    }

    #[test]
    fn a_declared_plugin_or_screen_space_is_quiet() {
        assert!(findings("// @bounds expand=reach\n", "layer").is_empty());
        assert!(findings("// @bounds none\n", "layer").is_empty());
        assert!(findings("", "screen").is_empty(), "screen space has no layer picture to cut off");
    }
}
