//! Helpers shared by the `schema_*` tests: load one layer and read what it evaluates to.

use std::path::Path;

use anyhow::Result;
use serde_json::{json, Value};
use turbomotion_comp::eval::{Content, Evaluator};
use turbomotion_comp::plan::{Plan, ROOT};

pub fn load(layer: Value) -> Result<Plan> {
    Plan::from_value(json!({"dur": 4, "layers": [layer]}), Path::new("."))
}

/// The error of loading `layer`, with its whole chain of context.
pub fn error_of(layer: Value) -> String {
    match load(layer) {
        Ok(_) => panic!("expected the layer to be rejected"),
        Err(e) => format!("{e:#}"),
    }
}

/// The evaluated items of a shape layer at `t`, as JSON.
pub fn shape_items_at(plan: &Plan, t: f64) -> Value {
    match &Evaluator::new(plan).frame(ROOT, t).expect("frame evaluates").layers[0].content {
        Content::Shape { items } => serde_json::to_value(items).expect("items serialise"),
        _ => panic!("not a shape layer"),
    }
}

/// A whole composition (not just one layer) as a plan.
pub fn plan_of(composition: Value) -> Plan {
    Plan::from_value(composition, Path::new(".")).expect("the composition loads")
}
