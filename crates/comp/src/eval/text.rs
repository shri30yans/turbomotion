//! Text layers at one instant: the string (or counter), size and colour, the layout, and a state per character.

use std::rc::Rc;

use anyhow::{Context, Result};
use serde::Serialize;

use super::{CompPlan, Evaluator, LayerEnv};
use crate::{
    anim::{ease::Ease, prop::Rgba},
    plan::LayerPlan,
    schema::{self, Animator, Text, TextSource},
    text::{
        animate::{glyph_states, GlyphState, Resolved},
        layout::{Layout, RunSpec, Spec},
        presets,
    },
};

use super::shape::FillState;

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

/// GSAP TextPlugin: type `to` from the start after `at`, showing `from` until then.
pub fn swap_string(s: &schema::TextSwap, t: f64) -> Result<String> {
    if t < s.at - 1e-9 {
        return Ok(s.from.clone());
    }
    let ease = s.ease.as_deref().map_or(Ok(Ease::parse("none")?), Ease::parse)?;
    let u = if s.dur <= 1e-9 { 1.0 } else { ((t - s.at) / s.dur).clamp(0.0, 1.0) };
    let u = ease.at(u);
    let chars: Vec<char> = s.to.chars().collect();
    if chars.is_empty() {
        return Ok(String::new());
    }
    let n = ((u * chars.len() as f64).ceil() as usize).clamp(0, chars.len());
    Ok(chars.into_iter().take(n).collect())
}

pub fn counter_string(c: &schema::Counter, t: f64) -> Result<String> {
    let ease = c.ease.as_deref().map_or(Ok(Ease::default()), Ease::parse)?;
    let u = if c.dur <= 1e-9 { f64::from(t >= c.at) } else { ((t - c.at) / c.dur).clamp(0.0, 1.0) };
    let v = c.from + (c.to - c.from) * ease.at(u);
    let mut s = format!("{:.*}", c.decimals, v.abs());
    if !c.sep.is_empty() {
        let (int, frac) = s.split_once('.').map_or((s.as_str(), None), |(i, f)| (i, Some(f)));
        let mut grouped = String::new();
        for (i, ch) in int.chars().enumerate() {
            if i > 0 && (int.len() - i) % 3 == 0 {
                grouped.push_str(&c.sep);
            }
            grouped.push(ch);
        }
        s = frac.map_or(grouped.clone(), |f| format!("{grouped}.{f}"));
    }
    Ok(format!("{}{}{}{}", if v < 0.0 { "-" } else { "" }, c.prefix, s, c.suffix))
}

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
    let glyphs = glyph_states(&layout, &resolved, lt, color, size, &inks)?;
    let span = Some(([0.0, 0.0], [layout.width as f64 * 0.12, layout.height as f64]));
    let fill = tx.fill.as_ref().map(|f| super::shape::fill_state(f, lt, &env, span)).transpose()?;
    let mark = tx.mark.as_ref().map(|m| mark_state(m, &layout, &text, lt, &env)).transpose()?.flatten();
    Ok(TextState {
        text,
        size,
        color,
        stroke: tx.stroke.as_ref().map(|s| Ok::<_, anyhow::Error>((s.color.0.at(lt, &env(63))?, s.width))).transpose()?,
        fill,
        mark,
        width: layout.width,
        height: layout.height,
        anchor: layout.anchor,
        glyphs,
        layout: Some(layout),
    })
}

fn mark_state<'a>(m: &schema::Mark, layout: &Layout, text: &str, lt: f64, env: &dyn Fn(u32) -> LayerEnv<'a>) -> Result<Option<MarkState>> {
    let Some(rect) = word_rect(layout, text, &m.word, m.nth) else { return Ok(None) };
    let ease = m.ease.as_deref().map_or(Ok(Ease::parse("power3.out")?), Ease::parse)?;
    let u = if m.dur <= 1e-9 { f64::from(lt >= m.at) } else { ((lt - m.at) / m.dur).clamp(0.0, 1.0) };
    let progress = ease.at(u);
    let pad = m.pad.unwrap_or((layout.height * 0.08).max(3.0) as f64) as f32;
    let color = match (&m.color, m.verdict) {
        (Some(c), _) => c.0.at(lt, &env(64))?,
        (None, Some(v)) => v.color(),
        (None, None) => [255.0, 211.0, 122.0, 0.88],
    };
    Ok(Some(MarkState { rect: [rect[0] - pad, rect[1] - pad * 0.3, rect[2] + pad, rect[3] + pad * 0.15], progress, color, rough: m.rough }))
}

/// Bounding box of the `nth` occurrence of `word` in `text`, in layout space.
fn word_rect(layout: &Layout, text: &str, word: &str, nth: usize) -> Option<[f32; 4]> {
    let hay = text.to_lowercase();
    let needle = word.to_lowercase();
    if needle.is_empty() {
        return None;
    }
    let mut from = 0;
    let mut hit = None;
    for _ in 0..=nth {
        let i = hay[from..].find(&needle)?;
        let start = from + i;
        hit = Some((start, start + needle.len()));
        from = start + needle.len();
    }
    let (byte0, byte1) = hit?;
    let mut start_char = 0;
    let mut end_char = 0;
    let mut bytes = 0;
    for (ci, ch) in text.chars().enumerate() {
        if bytes == byte0 {
            start_char = ci;
        }
        bytes += ch.len_utf8();
        if bytes == byte1 {
            end_char = ci + 1;
            break;
        }
    }
    // Glyphs skip whitespace and keep source order; walk characters the same way.
    let mut char_i = 0;
    let mut gi = 0;
    let mut rect: Option<[f32; 4]> = None;
    for ch in text.chars() {
        if ch.is_whitespace() {
            char_i += 1;
            continue;
        }
        if gi >= layout.glyphs.len() {
            break;
        }
        if char_i >= start_char && char_i < end_char {
            let g = &layout.glyphs[gi];
            let r = [g.x, g.top, g.x + g.w, g.bottom];
            rect = Some(match rect {
                None => r,
                Some(a) => [a[0].min(r[0]), a[1].min(r[1]), a[2].max(r[2]), a[3].max(r[3])],
            });
        }
        gi += 1;
        char_i += 1;
    }
    rect
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
