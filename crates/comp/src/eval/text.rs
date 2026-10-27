//! Text layers at one instant: the string (or counter), size and colour, the layout, and a state per character.

use std::rc::Rc;

use anyhow::{Context, Result};
use serde::Serialize;

pub mod bounds;
mod mark;
mod source;

use super::{shape::FillState, CompPlan, Evaluator, LayerEnv};
use crate::{
    anim::prop::Rgba,
    plan::LayerPlan,
    schema::{self, Animator, Perspective, Text, TextSource, P},
    text::{
        animate::{glyph_states, GlyphState, Resolved},
        bounds::{union, Edges, TextBounds},
        layout::{Layout, RunSpec, Spec},
        place::{Margins, OnPath, OnSurface, Placement, Row},
        presets,
    },
};
use self::{bounds::BoxState, mark::mark_state, source::{counter_string, swap_string}};

#[derive(Serialize)]
pub struct TextState {
    pub text: String,
    pub size: f64,
    pub color: Rgba,
    pub stroke: Option<(Rgba, f64)>,
    pub fill: Option<FillState>,
    pub mark: Option<MarkState>,
    pub width: f32,
    pub height: f32,
    pub anchor: [f32; 2],
    pub glyphs: Vec<GlyphState>,
    /// Some animator is changing at least one character right now (`false` for a layer with no animators).
    pub animated: bool,
    /// Paint the stroke only.
    pub outline_only: bool,
    /// `[start, end]` fractions of the outline length that are drawn.
    pub trim: Option<[f64; 2]>,
    pub trim_each: bool,
    /// Where the glyphs really are (layout space `[l, t, r, b]`) when a path, surface or marquee moved them.
    pub placed_bounds: Option<[f64; 4]>,
    /// The measured bounds of the characters showing (`None` when none is): the layer's text bounds, the one
    /// measurement the `box`, this output and `source_rect_at_time` share.
    pub bounds: Option<TextBounds>,
    /// The layer's `box` at this instant.
    pub text_box: Option<BoxState>,
    #[serde(skip)]
    pub layout: Option<Rc<Layout>>,
}

/// A highlighter sweeping across one word of the line.
#[derive(Serialize, Clone)]
pub struct MarkState {
    pub rect: [f32; 4],
    pub progress: f64,
    pub color: Rgba,
    pub rough: bool,
}

/// Expression seeds of the animated properties of trim, path and surface, kept clear of the animators' (70 and up) and masks' (400 and up).
const SALT_TRIM_START: u32 = 200;
const SALT_TRIM_END: u32 = 201;
const SALT_FIRST_MARGIN: u32 = 202;
const SALT_LAST_MARGIN: u32 = 203;
const SALT_TRAVEL: u32 = 204;
const SALT_RADIUS: u32 = 205;
const SALT_TURN: u32 = 206;

/// Stroke width of outline-only type that names no stroke of its own, px.
const OUTLINE_WIDTH: f64 = 2.0;

pub fn spec_of(ev: &Evaluator, c: &CompPlan, l: &LayerPlan, tx: &Text, t: f64) -> Result<(Spec, String, f64)> {
    let lt = t - l.start;
    let env = |salt: u32| LayerEnv::new(ev, c, l.index, t, 0).with_salt(salt);
    let text = match &tx.text {
        TextSource::Fixed(s) => s.clone(),
        TextSource::Counter(k) => counter_string(k, lt)?,
        TextSource::Swap(s) => swap_string(s, lt)?,
    };
    let size = tx.size.as_ref().map_or(Ok(64.0), |p| p.0.at(lt, &env(60)))?.max(1.0);
    let tracking = tx.tracking.as_ref().map_or(Ok(0.0), |p| p.0.at(lt, &env(61)))?;
    let spec = Spec::new(&text, tx.font.as_deref(), tx.weight, tx.italic, size, tracking, tx.leading, tx.align, tx.width).with_runs(run_specs(&tx.runs, &text));
    Ok((spec, text, size))
}

/// Character range of the `nth` exact occurrence of `needle` in `text`.
fn char_range(text: &str, needle: &str, nth: usize) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }
    let mut from = 0;
    let mut hit = None;
    for _ in 0..=nth {
        let at = from + text[from..].find(needle)?;
        hit = Some(at);
        from = at + needle.len();
    }
    let byte = hit?;
    let start = text[..byte].chars().count();
    Some((start, start + needle.chars().count()))
}

fn run_specs(runs: &[schema::TextRun], text: &str) -> Vec<RunSpec> {
    runs.iter()
        .filter(|r| r.font.is_some() || r.weight.is_some() || r.italic.is_some())
        .filter_map(|r| char_range(text, &r.text, r.nth).map(|(start, end)| RunSpec { start, end, font: r.font.clone(), weight: r.weight, italic: r.italic }))
        .collect()
}

/// The ink each glyph starts from: a run's own colour, or nothing (the line's colour) outside runs.
fn run_colors<'a>(tx: &Text, text: &str, glyphs: usize, lt: f64, env: &dyn Fn(u32) -> LayerEnv<'a>) -> Result<Vec<Option<[f64; 3]>>> {
    let mut out = vec![None; glyphs];
    let visible: Vec<usize> = text.chars().enumerate().filter(|(_, c)| !c.is_whitespace()).map(|(i, _)| i).collect();
    for (k, r) in tx.runs.iter().enumerate() {
        let (Some(col), Some((a, b))) = (&r.color, char_range(text, &r.text, r.nth)) else { continue };
        let rgba = col.0.at(lt, &env(90 + k as u32))?;
        for (gi, ci) in visible.iter().enumerate() {
            if *ci >= a && *ci < b && gi < glyphs {
                out[gi] = Some([rgba[0] as f64, rgba[1] as f64, rgba[2] as f64]);
            }
        }
    }
    Ok(out)
}

pub fn layout_of(ev: &Evaluator, c: &CompPlan, l: &LayerPlan, tx: &Text, t: f64) -> Result<Rc<Layout>> {
    let (spec, _, _) = spec_of(ev, c, l, tx, t)?;
    ev.text.layout(ev.plan, &spec)
}

pub fn state(ev: &Evaluator, c: &CompPlan, l: &LayerPlan, tx: &Text, t: f64) -> Result<TextState> {
    let lt = t - l.start;
    let env = |salt: u32| LayerEnv::new(ev, c, l.index, t, 0).with_salt(salt);
    let (spec, text, size) = spec_of(ev, c, l, tx, t)?;
    let layout = ev.text.layout(ev.plan, &spec).with_context(|| format!("text {text:?}"))?;
    let color = tx.color.as_ref().map_or(Ok([255.0, 255.0, 255.0, 1.0]), |c| c.0.at(lt, &env(62)))?;

    let mut animators: Vec<Animator> = tx.animators.clone();
    if let Some(p) = &tx.preset {
        animators.extend(presets::expand(p, presets::TextFacts::of(&text, size))?);
    }
    let mut resolved = Vec::with_capacity(animators.len());
    for (k, a) in animators.iter().enumerate() {
        let salt = 70 + 6 * k as u32;
        let (mut start, mut end, mut offset) = (0.0, 1.0, 0.0);
        if let Some(sel) = &a.selector {
            start = sel.start.as_ref().map_or(Ok(0.0), |p| p.0.at(lt, &env(salt)))?;
            end = sel.end.as_ref().map_or(Ok(1.0), |p| p.0.at(lt, &env(salt + 1)))?;
            offset = sel.offset.as_ref().map_or(Ok(0.0), |p| p.0.at(lt, &env(salt + 2)))?;
        }
        let color = a.props.color.as_ref().map(|c| c.0.at(lt, &env(salt + 3))).transpose()?;
        resolved.push((a, start, end, offset, color));
    }
    let resolved: Vec<Resolved> = resolved.into_iter().map(|(anim, start, end, offset, color)| Resolved { anim, start, end, offset, color }).collect();
    let inks = run_colors(tx, &text, layout.glyphs.len(), lt, &env)?;
    let placement = placement_of(c, l, tx, &layout, lt, &env)?;
    let glyphs = glyph_states(&layout, &resolved, lt, color, &inks, &placement)?;
    let animated = !resolved.is_empty() && glyphs != glyph_states(&layout, &[], lt, color, &inks, &placement)?;
    let stroke = match (&tx.stroke, tx.outline_only) {
        (Some(s), _) => Some((s.color.0.at(lt, &env(63))?, s.width)),
        (None, true) => Some((color, OUTLINE_WIDTH)),
        (None, false) => None,
    };
    let bounds = TextBounds::of(&layout, &glyphs, stroke.map_or(0.0, |(_, w)| w));
    let placed_bounds = if placement.moves_glyphs() { bounds.as_ref().map(|b| b.block.edges()) } else { None };
    let text_box = tx.r#box.as_ref().map(|b| bounds::box_state(b, bounds.as_ref(), tx.align, lt, &env)).transpose()?;
    let span = Some(([0.0, 0.0], [layout.width as f64 * 0.12, layout.height as f64]));
    let fill = tx.fill.as_ref().map(|f| super::shape::fill_state(f, lt, &env, span)).transpose()?;
    let mark = tx.mark.as_ref().map(|m| mark_state(m, &layout, &text, lt, &env)).transpose()?.flatten();
    let trim = tx.stroke_trim.as_ref().map(|[a, b]| Ok::<_, anyhow::Error>([a.0.at(lt, &env(SALT_TRIM_START))?.clamp(0.0, 1.0), b.0.at(lt, &env(SALT_TRIM_END))?.clamp(0.0, 1.0)])).transpose()?;
    Ok(TextState {
        text,
        size,
        color,
        stroke,
        fill,
        mark,
        width: layout.width,
        height: layout.height,
        anchor: layout.anchor,
        glyphs,
        animated,
        outline_only: tx.outline_only,
        trim,
        trim_each: tx.stroke_trim_each,
        placed_bounds,
        bounds,
        text_box,
        layout: Some(layout),
    })
}

impl TextState {
    /// The box of everything the layer draws, in layer pixels: the text as laid out (where a path, surface or marquee
    /// put the glyphs, those), and any `box` round it.
    pub fn local_bounds(&self) -> Edges {
        let text = self.placed_bounds.unwrap_or([0.0, 0.0, f64::from(self.width), f64::from(self.height)]);
        self.text_box.iter().flat_map(|b| &b.rects).fold(text, |all, r| union(all, *r))
    }
}

/// Path, surface, marquee and perspective for this text at this instant.
fn placement_of<'a>(c: &CompPlan, l: &LayerPlan, tx: &Text, layout: &Layout, lt: f64, env: &dyn Fn(u32) -> LayerEnv<'a>) -> Result<Placement> {
    let mut pl = Placement::default();
    if let (Some(tp), Some(path)) = (&tx.path, &l.text_path) {
        let margin = |p: &Option<P>, salt: u32| p.as_ref().map_or(Ok(0.0), |p| p.0.at(lt, &env(salt)));
        let margins = Margins { first: margin(&tp.first_margin, SALT_FIRST_MARGIN)?, last: margin(&tp.last_margin, SALT_LAST_MARGIN)?, travel: margin(&tp.travel, SALT_TRAVEL)? };
        pl.path = Some(OnPath::new(path, tp, margins, tx.align).context("text path has no length")?);
    }
    if let Some(sf) = &tx.surface {
        let radius = sf.radius.as_ref().map_or(Ok(f64::from(layout.width) / std::f64::consts::PI), |p| p.0.at(lt, &env(SALT_RADIUS)))?;
        anyhow::ensure!(radius > 0.0, "surface radius is above 0 pixels, not {radius}");
        let turn = sf.turn.as_ref().map_or(Ok(0.0), |p| p.0.at(lt, &env(SALT_TURN)))?;
        pl.surface = Some(OnSurface { kind: sf.kind, radius, turn, back: sf.back });
    }
    if let Some(m) = &tx.marquee {
        pl.row = Some(Row::new(m, layout, lt, f64::from(c.width)));
    }
    // a three_d layer is seen through the composition camera; a flat one gets its own viewer
    pl.perspective = match (l.in_3d, tx.perspective) {
        (true, _) | (false, Some(Perspective::Orthographic)) => None,
        (false, Some(Perspective::Distance(d))) => Some(d),
        (false, None) => Some(schema::default_camera_zoom(f64::from(c.width))),
    };
    Ok(pl)
}

#[cfg(test)]
mod tests {
    use crate::plan::{Plan, ROOT};
    use serde_json::json;

    #[test]
    fn a_mark_sweeps_across_one_word_and_leaves_the_rest() {
        let plan = Plan::from_value(
            json!({"dur": 3, "layers": [{"type": "text", "id": "line", "text": "Higher price in future", "size": 48, "color": "#ffffff", "pos": [960, 540], "align": "center",
                "mark": {"word": "future", "at": 0.5, "dur": 0.4}}]}),
            std::path::Path::new("."),
        )
        .unwrap();
        let ev = crate::eval::Evaluator::new(&plan);
        let before = ev.frame(ROOT, 0.2).unwrap();
        let crate::eval::Content::Text(t) = &before.layers[0].content else { panic!() };
        let m = t.mark.as_ref().expect("mark");
        assert!(m.progress < 0.05, "{}", m.progress);
        assert!(m.rect[2] > m.rect[0], "word has width");
        let after = ev.frame(ROOT, 1.2).unwrap();
        let crate::eval::Content::Text(t) = &after.layers[0].content else { panic!() };
        assert!((t.mark.as_ref().unwrap().progress - 1.0).abs() < 1e-6);
        assert!(t.mark.as_ref().unwrap().rect[2] - t.mark.as_ref().unwrap().rect[0] < t.width);
    }

    #[test]
    fn a_verdict_picks_the_highlighter_ink() {
        let plan = Plan::from_value(
            json!({"dur": 2, "layers": [{"type": "text", "text": "The fee was disclosed", "size": 40, "color": "#ffffff", "pos": [960, 540],
                "mark": {"word": "fee", "verdict": "loss", "at": 0}}]}),
            std::path::Path::new("."),
        )
        .unwrap();
        let f = crate::eval::Evaluator::new(&plan).frame(ROOT, 0.5).unwrap();
        let crate::eval::Content::Text(t) = &f.layers[0].content else { panic!() };
        let m = t.mark.as_ref().unwrap();
        assert!((m.color[0] - 255.0).abs() < 1.0 && m.color[1] < 90.0, "{:?}", m.color);
        assert!(m.rough);
    }

    #[test]
    fn a_run_recolours_and_restyles_only_its_stretch() {
        let plan = Plan::from_value(
            json!({"dur": 2, "layers": [{"type": "text", "id": "t", "text": "Same prompt.", "size": 60, "color": "#ffffff", "pos": [960, 540], "weight": 400,
                "runs": [{"match": "prompt.", "weight": 800, "color": "#d97757"}]}]}),
            std::path::Path::new("."),
        )
        .unwrap();
        let f = crate::eval::Evaluator::new(&plan).frame(ROOT, 0.5).unwrap();
        let crate::eval::Content::Text(t) = &f.layers[0].content else { panic!() };
        assert!(t.glyphs[0].color.is_none() && t.glyphs[3].color.is_none(), "text outside the run keeps the line colour");
        let c = t.glyphs[4].color.expect("run colour on the first glyph of the run");
        assert!((c[0] - 217.0).abs() < 1.0 && (c[2] - 87.0).abs() < 1.0, "{c:?}");
        assert!(t.glyphs.last().unwrap().color.is_some());
        let plain = Plan::from_value(
            json!({"dur": 2, "layers": [{"type": "text", "id": "t", "text": "Same prompt.", "size": 60, "color": "#ffffff", "pos": [960, 540], "weight": 400}]}),
            std::path::Path::new("."),
        )
        .unwrap();
        let g = crate::eval::Evaluator::new(&plain).frame(ROOT, 0.5).unwrap();
        let crate::eval::Content::Text(p) = &g.layers[0].content else { panic!() };
        let wide = |t: &crate::eval::text::TextState| t.glyphs.last().unwrap().x;
        assert!((wide(t) - wide(p)).abs() > 0.2, "a heavier run changes the advance of its glyphs: {} vs {}", wide(t), wide(p));
    }
}
