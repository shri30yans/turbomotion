//! The words of a text layer that are not a fixed string: a counter and a swap (GSAP's TextPlugin).

use anyhow::Result;

use crate::{anim::ease::Ease, schema};

/// GSAP TextPlugin: type `to` from the start after `at`, showing `from` until then.
pub(super) fn swap_string(s: &schema::TextSwap, t: f64) -> Result<String> {
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

pub(super) fn counter_string(c: &schema::Counter, t: f64) -> Result<String> {
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
