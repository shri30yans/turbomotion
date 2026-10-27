//! The highlighter of a text layer's `mark`: the box of one word and how far the sweep has come.

use anyhow::Result;

use super::{MarkState, LayerEnv};
use crate::{anim::ease::Ease, schema, text::layout::Layout};

pub(super) fn mark_state<'a>(m: &schema::Mark, layout: &Layout, text: &str, lt: f64, env: &dyn Fn(u32) -> LayerEnv<'a>) -> Result<Option<MarkState>> {
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
