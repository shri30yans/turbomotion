//! Every font a composition names must exist, and that is decided here, with the rest of the plan, so a missing font
//! is reported beside the other mistakes in a film and not at the first frame that draws it.

use std::collections::BTreeMap;

use anyhow::Result;

use super::Plan;
use crate::{
    schema::{Geometry, Kind, Problem, Problems},
    text::layout::resolve_typeface,
};

/// Check each distinct font once; every layer using a missing one is named in its message.
pub(super) fn check(plan: &Plan) -> Result<()> {
    let mut users: BTreeMap<&str, Vec<(&str, &str, &super::LayerPlan)>> = BTreeMap::new();
    for (comp, c) in &plan.comps {
        for l in &c.layers {
            let mut named: Vec<&str> = Vec::new();
            match &l.def.kind {
                Kind::Text(t) => named.extend(t.font.as_deref().into_iter().chain(t.runs.iter().filter_map(|r| r.font.as_deref()))),
                Kind::Mesh(m) => {
                    if let Geometry::Text { font: Some(f), .. } = &m.geometry {
                        named.push(f);
                    }
                }
                _ => {}
            }
            for f in named {
                users.entry(f).or_default().push((comp, &l.label, l));
            }
        }
    }
    let mut problems = Vec::new();
    for (font, layers) in users {
        let Err(e) = resolve_typeface(plan, Some(font), 400, false) else { continue };
        for (comp, label, l) in layers {
            problems.push(Problem { comp: comp.to_string(), layer: Some(l.index), id: l.def.common.id.clone(), message: format!("{label}: {e:#}") });
        }
    }
    problems.sort_by(|a, b| (&a.comp, a.layer).cmp(&(&b.comp, b.layer)));
    Problems::finish(problems)
}
