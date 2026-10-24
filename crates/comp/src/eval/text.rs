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
        layout::{Layout, Spec},
        presets,
    },
};

#[derive(Serialize)]
pub struct TextState {
    pub text: String,
    pub size: f64,
    pub color: Rgba,
    pub stroke: Option<(Rgba, f64)>,
    pub width: f32,
    pub height: f32,
    pub anchor: [f32; 2],
    pub glyphs: Vec<GlyphState>,
    #[serde(skip)]
    pub layout: Option<Rc<Layout>>,
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
    };
    let size = tx.size.as_ref().map_or(Ok(64.0), |p| p.0.at(lt, &env(60)))?.max(1.0);
    let tracking = tx.tracking.as_ref().map_or(Ok(0.0), |p| p.0.at(lt, &env(61)))?;
    Ok((Spec::new(&text, tx.font.as_deref(), tx.weight, tx.italic, size, tracking, tx.leading, tx.align, tx.width), text, size))
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
        animators.extend(presets::expand(p, size)?);
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
    let glyphs = glyph_states(&layout, &resolved, lt, color, size)?;
    Ok(TextState {
        text,
        size,
        color,
        stroke: tx.stroke.as_ref().map(|s| Ok::<_, anyhow::Error>((s.color.0.at(lt, &env(63))?, s.width))).transpose()?,
        width: layout.width,
        height: layout.height,
        anchor: layout.anchor,
        glyphs,
        layout: Some(layout),
    })
}
