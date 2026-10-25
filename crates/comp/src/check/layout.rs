//! `check --layout`: do foreground layers clash? Every layer that is drawn as part of a `layout` composition is rendered on its
//! own, reduced to a coverage mask, and compared with the others at the moments just after something appears and at the end of
//! each shot. Two layers that cover the same cells are a clash; two that sit one cell apart are crowded. It works on pixels, so a
//! chart, a map label or text from a pack counts exactly like a card.

use std::collections::BTreeMap;

use anyhow::Result;

use super::{Finding, Severity};
use crate::{plan::Plan, render::{LayoutMask, Renderer}};

/// Overlap smaller than this many cells (a cell is 4x4 px) is anti-aliasing, not a clash.
const MIN_CELLS: usize = 6;
/// Overlap as a share of the smaller layer: above this it is an error, below it a warning.
const ERROR_SHARE: f64 = 0.08;
/// Two layers closer than this many cells (4 px each, so 16 px) without touching are crowded.
const GAP_CELLS: usize = 4;

/// The moments worth looking at: shortly after each foreground layer of a `layout` comp appears, and just before each shot ends.
pub fn times(plan: &Plan) -> Vec<f64> {
    let root = plan.root();
    let mut t = Vec::new();
    for shot in &root.layers {
        let crate::schema::Kind::Precomp(p) = &shot.def.kind else { continue };
        let Some(c) = plan.comps.get(&p.comp) else { continue };
        if !c.layout {
            continue;
        }
        for l in &c.layers {
            t.push(shot.start + l.start + ((l.end - l.start) * 0.5).min(1.2));
        }
        t.push((shot.end - 0.45).max(shot.start));
    }
    t.retain(|x| *x >= 0.0 && *x < root.dur);
    t.sort_by(f64::total_cmp);
    t.dedup_by(|a, b| (*a - *b).abs() < 0.25);
    t
}

fn overlap(a: &LayoutMask, b: &LayoutMask, grow: usize) -> (usize, [usize; 4]) {
    let (mut n, mut bb) = (0, [usize::MAX, usize::MAX, 0, 0]);
    for y in 0..a.h {
        for x in 0..a.w {
            if a.cells[y * a.w + x] == 0 {
                continue;
            }
            let hit = (y.saturating_sub(grow)..=(y + grow).min(b.h - 1)).any(|yy| (x.saturating_sub(grow)..=(x + grow).min(b.w - 1)).any(|xx| b.cells[yy * b.w + xx] != 0));
            if hit {
                n += 1;
                bb = [bb[0].min(x), bb[1].min(y), bb[2].max(x), bb[3].max(y)];
            }
        }
    }
    (n, bb)
}

pub fn run(plan: &Plan) -> Result<Vec<Finding>> {
    let mut r = Renderer::new(plan)?;
    // (comp, layer a, layer b) -> first finding and how many sampled moments showed it
    let mut seen: BTreeMap<(String, String, String), (Finding, usize)> = BTreeMap::new();
    for t in times(plan) {
        let masks = r.layout_masks(t)?;
        for (i, a) in masks.iter().enumerate() {
            for b in &masks[i + 1..] {
                if a.comp != b.comp || (a.group.is_some() && a.group == b.group) {
                    continue;
                }
                let area = |m: &LayoutMask| m.cells.iter().filter(|c| **c != 0).count().max(1);
                let small = area(a).min(area(b));
                let (hit, mut bb) = overlap(a, b, 0);
                let mut bb_near = bb;
                let (sev, rule, what) = if hit >= MIN_CELLS && hit as f64 / small as f64 > ERROR_SHARE {
                    (Severity::Error, "layout-clash", format!("cover the same area ({:.0}% of the smaller)", 100.0 * hit as f64 / small as f64))
                } else if hit >= MIN_CELLS {
                    (Severity::Warn, "layout-clash", format!("overlap slightly ({:.0}% of the smaller)", 100.0 * hit as f64 / small as f64))
                } else {
                    let (near, nb) = overlap(a, b, GAP_CELLS);
                    if near < MIN_CELLS * 2 {
                        continue;
                    }
                    bb_near = nb;
                    (Severity::Warn, "layout-crowded", "are closer than 16 px".to_string())
                };
                if hit < MIN_CELLS {
                    bb = bb_near;
                }
                let where_ = format!("around x {}-{}, y {}-{} px", bb[0].min(bb[2]) * 4, bb[2] * 4, bb[1].min(bb[3]) * 4, bb[3] * 4);
                let key = (a.comp.clone(), a.layer.clone(), b.layer.clone());
                let f = Finding {
                    severity: sev,
                    rule,
                    layer: Some(format!("{}: {} + {}", a.comp, a.layer, b.layer)),
                    t: Some(t),
                    message: format!("{} and {} {what}, {where_}", a.layer, b.layer),
                    fix: "move one of them, shorten its text, or end the earlier one before the later one appears".into(),
                };
                seen.entry(key).and_modify(|e| e.1 += 1).or_insert((f, 1));
            }
        }
    }
    Ok(seen.into_values().map(|(mut f, n)| { if n > 1 { f.message += &format!(" (seen at {n} sampled moments)"); } f }).collect())
}
