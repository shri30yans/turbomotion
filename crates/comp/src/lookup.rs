//! Naming a layer: by its `id`, or by the path of ids through precompositions (`intro/title`, `intro/card/word`).
//!
//! A film's shots are precomposition layers named by the shot id, so `title` inside shot `intro` is `intro/title`.
//! `resolve` accepts the bare id as well, when exactly one layer anywhere carries it. This is the one implementation
//! behind `comp inspect --layer`, and what the script crate's `sdk inspect --layer` should call.

use anyhow::{bail, Result};

use crate::plan::{Plan, ROOT};
use crate::schema::Kind;

/// How many known paths an error shows before it stops listing.
const SHOWN: usize = 12;

/// Every addressable layer path of the plan, root layers first, each precomposition's layers under `id/`.
pub fn paths(plan: &Plan) -> Vec<String> {
    let mut out = Vec::new();
    collect(plan, ROOT, "", &mut out);
    out
}

fn collect(plan: &Plan, comp: &str, prefix: &str, out: &mut Vec<String>) {
    let Some(comp) = plan.comps.get(comp) else { return };
    let mut nested = Vec::new();
    for layer in &comp.layers {
        let Some(id) = layer.def.common.id.as_deref() else { continue };
        out.push(format!("{prefix}{id}"));
        if let Kind::Precomp(p) = &layer.def.kind {
            nested.push((format!("{prefix}{id}/"), p.comp.clone()));
        }
    }
    for (child_prefix, name) in nested {
        collect(plan, &name, &child_prefix, out);
    }
}

/// The path to select for `query`.
/// - an exact path (or root id) is returned as it is;
/// - a query with a `/` that names no known layer passes through unchanged, so the selector reports its own, more specific error;
/// - a bare id resolves to the one layer that carries it; none or several is an error that lists the paths.
pub fn resolve(plan: &Plan, query: &str) -> Result<String> {
    let known = paths(plan);
    if known.iter().any(|p| p == query) || query.contains('/') {
        return Ok(query.to_owned());
    }
    let suffix = format!("/{query}");
    let matches: Vec<&String> = known.iter().filter(|p| p.ends_with(&suffix)).collect();
    match matches.as_slice() {
        [one] => Ok((*one).clone()),
        [] => bail!("no layer {query:?}; layers are: {}", listing(known.iter())),
        many => bail!("layer {query:?} is in more than one place; use one of: {}", listing(many.iter().copied())),
    }
}

fn listing<'a>(paths: impl ExactSizeIterator<Item = &'a String>) -> String {
    let total = paths.len();
    let shown = paths.take(SHOWN).map(String::as_str).collect::<Vec<_>>().join(", ");
    if total > SHOWN { format!("{shown}, … ({total} in all)") } else { shown }
}
