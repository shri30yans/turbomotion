//! Data-driven diagrams: family trees, flows and timelines laid out from a JSON file, so a new film
//! needs data, not Rust. `graph <file.json> | step=n` (the file is relative to the project's `public/`).
//!
//! Elements carry a `step` (default 1) and optionally an `at` cue spec (`"t1.2"`, `"0"`, `"payoff+0.3"`).
//! Without `at`, the k-th element of a step lands on the step's k-th cue, which is what the narration
//! of a build-up ("and then… and then…") wants. Earlier steps are already on the board, later ones hidden.

use std::{collections::HashMap, fs};

use anyhow::{bail, ensure, Context, Result};
use serde::Deserialize;
use skia_safe::Color;

use super::{
    kit::{halo, label, mono, Kit},
    node::{Anim, Node},
    Sheet,
};
use crate::{
    film::Shot,
    palette::{hex, INK, INK_SOFT, LINE, OCHRE, PAPER, TEAL, TERRACOTTA},
    text::{Family, Style},
};

pub const PARAMS: &[&str] = &["step"];

const CARD: Color = hex(0xFBF4E6);
const W: f32 = 1920.0;
const MARGIN: f32 = 110.0;

#[derive(Deserialize)]
struct Spec {
    #[serde(rename = "type")]
    kind: String,
    tag: Option<String>,
    legend: Option<String>,
    #[serde(default)]
    dark: bool,
    #[serde(default)]
    nodes: Vec<Person>,
    #[serde(default)]
    edges: Vec<Edge>,
    #[serde(default)]
    events: Vec<Event>,
}

#[derive(Deserialize)]
struct Person {
    id: String,
    name: String,
    sub: Option<String>,
    parent: Option<String>,
    /// Drawn beside this person and joined by a double line (a marriage, an alliance).
    partner: Option<String>,
    /// Label on the partner line ("1562").
    link: Option<String>,
    step: Option<u32>,
    at: Option<String>,
    /// terracotta | ochre | teal
    accent: Option<String>,
}

#[derive(Deserialize)]
struct Edge {
    from: String,
    to: String,
    label: Option<String>,
}

#[derive(Deserialize)]
struct Event {
    year: f32,
    label: String,
    sub: Option<String>,
    step: Option<u32>,
    at: Option<String>,
    accent: Option<String>,
}

fn leak(s: Option<String>) -> Option<&'static str> {
    s.map(|s| &*Box::leak(s.into_boxed_str()))
}

fn accent(name: &Option<String>) -> Result<Option<Color>> {
    Ok(match name.as_deref() {
        None => None,
        Some("terracotta") => Some(TERRACOTTA),
        Some("ochre") => Some(OCHRE),
        Some("teal") => Some(TEAL),
        Some(other) => bail!("unknown accent {other:?} (terracotta, ochre, teal)"),
    })
}

/// Hands out each element's cue: `at` if given, else the next cue of its step.
#[derive(Default)]
struct Cues(HashMap<u32, usize>);

impl Cues {
    /// (spec, spec a half-second earlier for the stroke leading into it)
    fn next(&mut self, step: u32, at: &Option<String>) -> (String, String) {
        match at {
            Some(at) => (format!("{step}:{at}"), format!("{step}:{}", lead(at))),
            None => {
                let k = self.0.entry(step).or_insert(0);
                let out = (format!("{step}:{k}"), format!("{step}:{k}-0.3"));
                *k += 1;
                out
            }
        }
    }
}

/// A cue spec shifted 0.3 s earlier ("1" -> "1-0.30", "payoff+0.6" -> "payoff+0.30", "t1.2" -> "t0.90").
fn lead(at: &str) -> String {
    if let Some(t) = at.strip_prefix('t').and_then(|v| v.parse::<f32>().ok()) {
        return format!("t{:.2}", (t - 0.3).max(0.05));
    }
    let split = at.char_indices().skip(1).find(|(_, c)| matches!(c, '+' | '-')).map_or(at.len(), |(i, _)| i);
    let (cue, offset) = at.split_at(split);
    format!("{cue}{:+.2}", offset.parse::<f32>().unwrap_or(0.0) - 0.3)
}

fn display(size: f32, weight: f32) -> Style {
    Style { letter_spacing: -1.0, ..Style::new(Family::Fraunces, size, weight, INK) }
}

/// A person/event card: rounded paper rectangle, name that shrinks to fit, optional caption.
fn card(k: &Kit, x: f32, y: f32, w: f32, h: f32, name: &str, sub: Option<&str>, highlight: Option<Color>) -> Result<Node> {
    let mut style = display(46.0, 500.0);
    let wide = k.fonts.measure(name, &style);
    // a long name breaks at the space nearest its middle rather than shrinking past reading size
    let lines: Vec<&str> = match name.match_indices(' ').map(|(i, _)| i).min_by_key(|&i| (i as i64 - name.len() as i64 / 2).abs()) {
        Some(i) if wide > w - 28.0 => vec![&name[..i], &name[i + 1..]],
        _ => vec![name],
    };
    let widest = lines.iter().map(|l| k.fonts.measure(l, &style)).fold(0.0, f32::max);
    if widest > w - 28.0 {
        style.size = (style.size * (w - 28.0) / widest).max(22.0);
    }
    let frame = Node::rect(x - w / 2.0, y - h / 2.0, w, h).rounded(10.0).fill(CARD).stroke(highlight.unwrap_or(LINE), if highlight.is_some() { 5.0 } else { 2.5 });
    let lh = style.size * 1.3;
    let top = y + if sub.is_some() { -h * 0.08 } else { h * 0.11 } - (lines.len() as f32 - 1.0) * lh / 2.0;
    let mut parts = vec![frame];
    for (n, l) in lines.iter().enumerate() {
        parts.push(k.centred(l, x, top + n as f32 * lh, style.clone())?);
    }
    if let Some(sub) = sub {
        let mut cap = mono(22.0, TERRACOTTA);
        let m = k.fonts.measure(sub, &cap);
        if m > w - 24.0 {
            cap.size = (cap.size * (w - 24.0) / m).max(15.0);
        }
        parts.push(k.centred(sub, x, y + h * 0.34, cap)?);
    }
    Ok(Node::group(parts))
}

pub fn build(k: &Kit, shot: &Shot, json: &str) -> Result<Sheet> {
    let spec: Spec = serde_json::from_str(json).with_context(|| format!("{}: graph data", shot.id))?;
    let step = super::step_param(shot, 1)?;
    let nodes = match spec.kind.as_str() {
        "tree" => tree(k, &spec)?,
        "flow" => flow(k, &spec)?,
        "timeline" => timeline(k, &spec)?,
        other => bail!("{}: graph type {other:?} is not tree, flow or timeline", shot.id),
    };
    Ok(Sheet { dark: spec.dark, tag: leak(spec.tag).unwrap_or(""), legend: leak(spec.legend), step, nodes })
}

pub fn load(public: &std::path::Path, shot: &Shot) -> Result<String> {
    let file = shot.args.first().with_context(|| format!("{}: graph shot needs a data file", shot.id))?;
    fs::read_to_string(public.join(file)).with_context(|| format!("{}: reading graph data {file}", shot.id))
}

/* ------------------------------------------------------------------ tree */

struct Unit<'a> {
    first: &'a Person,
    second: Option<&'a Person>,
    kids: Vec<usize>,
    level: usize,
    width: f32,
    centre: f32,
}

fn tree(k: &Kit, spec: &Spec) -> Result<Vec<Node>> {
    ensure!(!spec.nodes.is_empty(), "tree has no nodes");
    let by_id: HashMap<&str, &Person> = spec.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    ensure!(by_id.len() == spec.nodes.len(), "tree has duplicate ids");
    for n in &spec.nodes {
        for r in [&n.parent, &n.partner].into_iter().flatten() {
            ensure!(by_id.contains_key(r.as_str()), "{} refers to unknown {r:?}", n.id);
        }
    }
    // A unit is a person plus their partner; the partner never has a parent of their own.
    let mut units: Vec<Unit> = Vec::new();
    let mut unit_of: HashMap<&str, usize> = HashMap::new();
    // n.partner = X means n sits beside X. So X owns the unit, n is its second member.
    for n in &spec.nodes {
        if n.partner.is_some() {
            continue;
        }
        let second = spec.nodes.iter().find(|m| m.partner.as_deref() == Some(n.id.as_str()));
        unit_of.insert(&n.id, units.len());
        if let Some(s) = second {
            unit_of.insert(&s.id, units.len());
        }
        units.push(Unit { first: n, second, kids: vec![], level: 0, width: 0.0, centre: 0.0 });
    }
    let mut roots = Vec::new();
    for i in 0..units.len() {
        let parents: Vec<&str> = [units[i].first.parent.as_deref(), units[i].second.and_then(|s| s.parent.as_deref())].into_iter().flatten().collect();
        match parents.first() {
            Some(p) => {
                let up = *unit_of.get(p).with_context(|| format!("parent {p:?} has no unit"))?;
                units[up].kids.push(i);
            }
            None => roots.push(i),
        }
    }
    ensure!(!roots.is_empty(), "tree has no root (a parent cycle?)");
    fn measure(units: &mut Vec<Unit>, i: usize, level: usize, depth: &mut usize) -> f32 {
        units[i].level = level;
        *depth = (*depth).max(level);
        let own: f32 = if units[i].second.is_some() { 2.0 } else { 1.0 };
        let kids = units[i].kids.clone();
        let sum: f32 = kids.iter().map(|&c| measure(units, c, level + 1, depth)).sum();
        units[i].width = own.max(sum);
        units[i].width
    }
    fn place(units: &mut Vec<Unit>, i: usize, start: f32) {
        let kids = units[i].kids.clone();
        let w = units[i].width;
        if kids.is_empty() {
            units[i].centre = start + w / 2.0;
            return;
        }
        let kids_w: f32 = kids.iter().map(|&c| units[c].width).sum();
        let mut x = start + (w - kids_w) / 2.0;
        for &c in &kids {
            place(units, c, x);
            x += units[c].width;
        }
        units[i].centre = (units[*kids.first().unwrap()].centre + units[*kids.last().unwrap()].centre) / 2.0;
    }
    let mut depth = 0;
    let total: f32 = roots.iter().map(|&r| measure(&mut units, r, 0, &mut depth)).sum();
    let mut x = 0.0;
    for &r in &roots {
        place(&mut units, r, x);
        x += units[r].width;
    }
    let slot = ((W - 2.0 * MARGIN) / total).min(430.0);
    let card_w = (slot * 0.8).min(400.0);
    ensure!(card_w >= 150.0, "{total} people in one row do not fit; split the tree into steps or two graphs");
    let card_h = 132.0;
    let x_of = |c: f32| (W - total * slot) / 2.0 + c * slot;
    let y_of = |level: usize| if depth == 0 { 540.0 } else { 240.0 + level as f32 * (600.0 / depth as f32).min(300.0) };

    let mut cues = Cues::default();
    let mut specs: HashMap<&str, (u32, (String, String))> = HashMap::new();
    for n in &spec.nodes {
        let step = n.step.unwrap_or(1);
        specs.insert(&n.id, (step, cues.next(step, &n.at)));
    }
    let mut out = Vec::new();
    // Connectors first so cards sit on top.
    for u in &units {
        let (ux, uy) = (x_of(u.centre), y_of(u.level));
        for &c in &u.kids {
            let kid = &units[c];
            let (kx, ky) = (x_of(kid.centre), y_of(kid.level));
            let mid = (uy + ky) / 2.0;
            let owner = if units[c].first.parent.is_some() { units[c].first } else { units[c].second.unwrap_or(units[c].first) };
            let (_, (_, lead)) = &specs[owner.id.as_str()];
            out.push(Node::path(&format!("M{ux} {} V{mid} H{kx} V{}", uy + card_h / 2.0, ky - card_h / 2.0))?.no_fill().stroke(INK_SOFT, 4.0).reveal(lead, Anim::draw(0.7)));
        }
    }
    for u in &units {
        let (cx, y) = (x_of(u.centre), y_of(u.level));
        let xs: Vec<(f32, &Person)> = match u.second {
            Some(s) => vec![(cx - slot / 2.0, u.first), (cx + slot / 2.0, s)],
            None => vec![(cx, u.first)],
        };
        if let Some(s) = u.second {
            let (_, (_, lead)) = &specs[s.id.as_str()];
            let (a, b) = (xs[0].0 + card_w / 2.0, xs[1].0 - card_w / 2.0);
            out.push(Node::path(&format!("M{a} {} H{b} M{a} {} H{b}", y - 9.0, y + 9.0))?.no_fill().stroke(TERRACOTTA, 4.0).reveal(lead, Anim::draw(0.6)));
            if let Some(link) = &s.link {
                let (_, (spec_s, _)) = &specs[s.id.as_str()];
                out.push(k.centred(link, (a + b) / 2.0, y - card_h / 2.0 - 16.0, halo(mono(20.0, TERRACOTTA), PAPER))?.reveal(spec_s, Anim::fade()));
            }
        }
        for (x, p) in xs {
            let (_, (spec_p, _)) = &specs[p.id.as_str()];
            out.push(card(k, x, y, card_w, card_h, &p.name, p.sub.as_deref(), accent(&p.accent)?)?.reveal(spec_p, Anim::up()));
        }
    }
    Ok(out)
}

/* ------------------------------------------------------------------ flow */

fn flow(k: &Kit, spec: &Spec) -> Result<Vec<Node>> {
    ensure!(!spec.nodes.is_empty(), "flow has no nodes");
    let idx: HashMap<&str, usize> = spec.nodes.iter().enumerate().map(|(i, n)| (n.id.as_str(), i)).collect();
    ensure!(idx.len() == spec.nodes.len(), "flow has duplicate ids");
    for e in &spec.edges {
        ensure!(idx.contains_key(e.from.as_str()) && idx.contains_key(e.to.as_str()), "edge {} -> {} names an unknown node", e.from, e.to);
    }
    let mut layer = vec![0usize; spec.nodes.len()];
    for _ in 0..spec.nodes.len() {
        for e in &spec.edges {
            let (a, b) = (idx[e.from.as_str()], idx[e.to.as_str()]);
            layer[b] = layer[b].max(layer[a] + 1);
        }
    }
    ensure!(layer.iter().all(|&l| l < spec.nodes.len()), "flow contains a cycle");
    let layers = layer.iter().max().copied().unwrap_or(0) + 1;
    let mut rows: Vec<Vec<usize>> = vec![vec![]; layers];
    for (i, &l) in layer.iter().enumerate() {
        rows[l].push(i);
    }
    let tallest = rows.iter().map(Vec::len).max().unwrap_or(1) as f32;
    let col = (W - 2.0 * MARGIN) / layers as f32;
    let card_w = (col * if spec.edges.iter().any(|e| e.label.is_some()) { 0.72 } else { 0.78 }).min(380.0);
    ensure!(card_w >= 150.0, "{layers} columns do not fit; split the flow");
    let card_h = 170.0;
    let row = (700.0 / tallest).min(200.0);
    let pos = |i: usize| {
        let l = layer[i];
        let r = rows[l].iter().position(|&j| j == i).unwrap_or(0) as f32;
        let n = rows[l].len() as f32;
        (MARGIN + col * (l as f32 + 0.5), 560.0 + (r - (n - 1.0) / 2.0) * row)
    };
    let mut cues = Cues::default();
    let specs: Vec<(u32, (String, String))> = spec.nodes.iter().map(|n| { let s = n.step.unwrap_or(1); (s, cues.next(s, &n.at)) }).collect();
    let mut out = Vec::new();
    for e in &spec.edges {
        let (a, b) = (idx[e.from.as_str()], idx[e.to.as_str()]);
        let ((ax, ay), (bx, by)) = (pos(a), pos(b));
        let (x1, x2) = (ax + card_w / 2.0, bx - card_w / 2.0);
        let mx = (x1 + x2) / 2.0;
        let (_, (_, lead)) = &specs[b];
        let mut parts = vec![Node::path(&format!("M{x1} {ay} C{mx} {ay} {mx} {by} {x2} {by}"))?.no_fill().stroke(INK_SOFT, 4.0).reveal(lead, Anim::draw(0.7))];
        parts.push(Node::path(&format!("M{} {} L{x2} {by} L{} {} Z", x2 - 16.0, by - 9.0, x2 - 16.0, by + 9.0))?.fill(INK_SOFT).reveal(&specs[b].1 .0, Anim::fade()));
        if let Some(text) = &e.label {
            parts.push(k.centred(text, mx, ay.min(by) - card_h / 2.0 - 18.0, halo(label(26.0, TERRACOTTA, 700.0), PAPER))?.reveal(&specs[b].1 .0, Anim::fade()));
        }
        out.extend(parts);
    }
    for (i, n) in spec.nodes.iter().enumerate() {
        let (x, y) = pos(i);
        out.push(card(k, x, y, card_w, card_h, &n.name, n.sub.as_deref(), accent(&n.accent)?)?.reveal(&specs[i].1 .0, Anim::up()));
    }
    Ok(out)
}

/* ------------------------------------------------------------------ timeline */

fn timeline(k: &Kit, spec: &Spec) -> Result<Vec<Node>> {
    ensure!(spec.events.len() >= 2, "a timeline needs at least two events");
    let (lo, hi) = spec.events.iter().fold((f32::MAX, f32::MIN), |(l, h), e| (l.min(e.year), h.max(e.year)));
    ensure!(hi > lo, "timeline events share one year");
    let pad = (hi - lo) * 0.06;
    let (x0, x1, axis) = (150.0, W - 150.0, 560.0);
    let x_of = |y: f32| x0 + (y - lo + pad) / (hi - lo + 2.0 * pad) * (x1 - x0);
    let mut cues = Cues::default();
    let mut out = vec![Node::path(&format!("M{x0} {axis} H{x1}"))?.no_fill().stroke(INK, 5.0).cap(skia_safe::PaintCap::Round).reveal("1:t0.1", Anim::draw(1.0))];
    // tick marks at round years
    let span = hi - lo;
    let unit = [1.0, 5.0, 10.0, 25.0, 50.0, 100.0].into_iter().find(|u| span / u <= 12.0).unwrap_or(100.0);
    let mut y = (lo / unit).ceil() * unit;
    while y <= hi {
        let x = x_of(y);
        out.push(Node::path(&format!("M{x} {} V{}", axis - 9.0, axis + 9.0))?.no_fill().stroke(INK_SOFT, 2.5).reveal("1:t0.4", Anim::fade()));
        if spec.events.iter().all(|e| (x_of(e.year) - x).abs() > 44.0) {
            out.push(k.centred(&format!("{}", y as i32), x, axis + 40.0, mono(15.0, INK_SOFT))?.reveal("1:t0.4", Anim::fade()));
        }
        y += unit;
    }
    let mut order: Vec<usize> = (0..spec.events.len()).collect();
    order.sort_by(|&a, &b| spec.events[a].year.total_cmp(&spec.events[b].year));
    let mut last_end = [f32::MIN; 4];
    for (n, &i) in order.iter().enumerate() {
        let e = &spec.events[i];
        let step = e.step.unwrap_or(1);
        let (cue, lead) = cues.next(step, &e.at);
        let x = x_of(e.year);
        let w = (k.fonts.measure(&e.label, &display(34.0, 500.0)) + 44.0).clamp(230.0, 400.0);
        // four rows (two above, two below the axis); take the first whose card does not collide
        let row = [0usize, 1, 2, 3].into_iter().cycle().skip(n % 4).take(4).find(|&r| x - w / 2.0 > last_end[r] + 16.0).unwrap_or(n % 4);
        last_end[row] = x + w / 2.0;
        let up = row % 2 == 0;
        let tier = (row / 2) as f32;
        let h = 128.0;
        let cy = if up { axis - 150.0 - tier * 170.0 } else { axis + 150.0 + tier * 170.0 };
        let cx = x.clamp(MARGIN + w / 2.0 - 40.0, W - MARGIN - w / 2.0 + 40.0);
        let colour = accent(&e.accent)?;
        let stem_from = if up { cy + h / 2.0 } else { cy - h / 2.0 };
        out.push(Node::path(&format!("M{x} {axis} V{stem_from}"))?.no_fill().stroke(INK_SOFT, 3.5).reveal(&lead, Anim::draw(0.5)));
        out.push(Node::circle(x, axis, 12.0).fill(colour.unwrap_or(TERRACOTTA)).stroke(INK, 3.0).reveal(&cue, Anim::pop()));
        let mut parts = vec![
            Node::rect(cx - w / 2.0, cy - h / 2.0, w, h).rounded(10.0).fill(CARD).stroke(colour.unwrap_or(LINE), if colour.is_some() { 5.0 } else { 2.5 }),
            k.centred(&format!("{}", e.year as i32), cx, cy - h * 0.2, mono(22.0, TERRACOTTA))?,
        ];
        let mut style = display(34.0, 500.0);
        let m = k.fonts.measure(&e.label, &style);
        if m > w - 28.0 {
            style.size = (style.size * (w - 28.0) / m).max(18.0);
        }
        parts.push(k.centred(&e.label, cx, cy + h * 0.14, style)?);
        if let Some(sub) = &e.sub {
            parts.push(k.centred(sub, cx, cy + h * 0.38, mono(15.0, INK_SOFT))?);
        }
        out.push(Node::group(parts).reveal(&cue, Anim::up()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lead_shifts_every_spec_form_earlier() {
        assert_eq!(lead("1"), "1-0.30");
        assert_eq!(lead("payoff+0.6"), "payoff+0.30");
        assert_eq!(lead("2-0.3"), "2-0.60");
        assert_eq!(lead("t1.2"), "t0.90");
    }
}
