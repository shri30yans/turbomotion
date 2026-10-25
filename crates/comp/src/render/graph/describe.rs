//! `comp graph`: the passes and edges of a composition frame, as text.

use std::fmt::Write as _;

use anyhow::Result;

use super::{comp_graph, Graph, Pass, Stage, Write};
use crate::{
    eval::{Content, Evaluator, Frame},
    plan::Plan,
};

fn stage_name(g: &Graph, s: &Stage) -> String {
    let label = |l: &usize| g.comp.layers[*l].label.as_str();
    match s {
        Stage::Backdrop => "backdrop".to_string(),
        Stage::Paint { layer } => format!("paint {}", label(layer)),
        Stage::Effects { owner, upto, pictures } => {
            let on = match owner {
                super::FxOwner::Layer(l) | super::FxOwner::Adjustment(l) => label(l).to_string(),
                super::FxOwner::Look => "look".to_string(),
            };
            let reads: Vec<&str> = pictures.iter().map(label).collect();
            format!("effects 0..{upto} of {on}{}", if reads.is_empty() { String::new() } else { format!(" (reads {})", reads.join(", ")) })
        }
        Stage::MaskCoverage { layer } => format!("masks of {}", label(layer)),
        Stage::MatteCoverage { mode, opacity } => format!("matte {mode:?} at opacity {opacity:.2}"),
        Stage::Multiply => "multiply by coverage".to_string(),
        Stage::Reflect { .. } => "floor reflection".to_string(),
        Stage::Wipe { .. } => "wipe".to_string(),
        Stage::ShutterAverage { opacities } => format!("average of {} shutter samples", opacities.len()),
        Stage::Composite { blend, opacity } => format!("composite {} at opacity {opacity:.2}", blend.map_or("normal".to_string(), |b| format!("{b:?}").to_lowercase())),
        Stage::Snapshot => "snapshot of the composite".to_string(),
        Stage::AdjustCoverage { layer } => format!("coverage of adjustment {}", label(layer)),
        Stage::AdjustMix => "mix adjustment through coverage".to_string(),
    }
}

fn line(g: &Graph, n: usize, p: &Pass) -> String {
    let reads: Vec<String> = p.reads.iter().map(|r| format!("i{}", r.0)).collect();
    let write = match p.write {
        Write::New(i) => format!("i{}", i.0),
        Write::Composite => "composite".to_string(),
    };
    let frame = if p.frame == 0 { String::new() } else { format!(" [shutter sample {}]", p.frame) };
    format!("  p{n:<3} {} -> {write}   {}{frame}", reads.join(", "), stage_name(g, &p.stage))
}

fn graph_text(out: &mut String, plan: &Plan, g: &Graph, frame: &Frame, depth: usize) {
    let pad = "  ".repeat(depth);
    let name = if g.comp.name.is_empty() { "root" } else { g.comp.name.as_str() };
    let _ = writeln!(out, "{pad}comp {name} at {:.3}s: {} passes, at most {} images alive at once, output {}", frame.t, g.passes.len(), g.peak_images(), g.outputs.iter().map(|o| format!("i{}", o.0)).collect::<Vec<_>>().join(", "));
    for (n, p) in g.passes.iter().enumerate() {
        let _ = writeln!(out, "{pad}{}", line(g, n, p));
        if let Stage::Paint { layer } = p.stage {
            if let Content::Precomp { frame: Some(inner) } = &g.frames[p.frame].layers[layer].content {
                graph_text(out, plan, &comp_graph(&plan.comps[&inner.comp], inner, &[]), inner, depth + 3);
            }
        }
    }
}

/// The passes and edges (`i<n>` are images) of the root composition at `t` seconds, nested compositions indented.
pub fn describe(plan: &Plan, ev: &Evaluator, t: f64) -> Result<String> {
    let frame = ev.frame(crate::plan::ROOT, t)?;
    let samples = crate::render::shutter_frames(plan, ev, t)?;
    let mut out = String::new();
    graph_text(&mut out, plan, &comp_graph(plan.root(), &frame, &samples), &frame, 0);
    Ok(out)
}
