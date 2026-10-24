//! Panel elements: glass cards and flat rectangles.

use std::rc::Rc;

use anyhow::Result;
use skia_safe::{Canvas, Color, Paint, RRect, Rect};

use crate::{{anim::{blit, Blit, Timing}, color::{fill, fill_a}, model::{Card, Doc, List, RectEl, Rgb, Ticker, GREEN, INK, RED, STEEL, WHITE}, pyfmt, stage::{Stage}, typeset::{Align, TextSpec}}};
use turbomotion_core::{ease::{expo_out, keyed}, layer::{over, Layer, Shadow}};

use super::surface::{panel, paper, Panel};

pub fn card(st: &Stage, cv: &Canvas, e: &Card, tm: &Timing) -> Result<()> {
    let w = e.w as i32;
    let left = e.align == "left";
    let align = Align::parse(&e.align);
    let mut parts = Vec::new();
    let mut y = 26;
    if let Some(title) = &e.title {
        let caption = st.type_.layer(cv, &TextSpec::new(title.to_uppercase(), "avn_b", e.tsize, e.tfill).tracking(3.0).align(Align::Left).maxw(Some(f64::from(w - 60))).pad(4))?;
        y = 18 + caption.height() + 6;
        parts.push((caption, 28, 18));
    }
    for line in &e.lines {
        let fill = line.3.unwrap_or(WHITE);
        let lt = st.type_.layer(cv, &TextSpec::new(&line.0, &line.1, line.2, fill).align(align).maxw(Some(f64::from(w - 56))).pad(6))?;
        let x = if left { 24 } else { (w - lt.width()) / 2 };
        let step = lt.height() - 8;
        parts.push((lt, x, y));
        y += step;
    }
    let need = y + 26;
    let h = (e.h as i32).max(need);
    let shift = if e.title.is_none() { (h - need) / 2 } else { 0 };
    let base = panel(st, cv, &Panel { w, h, radius: e.radius, fill: e.bg, fill_a: e.bg_a, border: e.border, accent: e.accent })?;
    let layer = Layer::render(cv, w, h, |c| {
        over(c, &base, 0, 0, 1.0);
        for (lt, x, y) in &parts {
            over(c, lt, *x, y + shift, 1.0);
        }
    })?;
    blit(cv, &layer, e.x, e.y, tm, Blit::new("rise").rot(e.rot).shadow(Shadow::new(22, 26.0, 0.5)))
}

pub fn rect(cv: &Canvas, e: &RectEl, tm: &Timing) -> Result<()> {
    let (w, h) = (e.w as i32, e.h as i32);
    let layer = Layer::render(cv, w, h, |c| {
        let mut paint = Paint::default();
        paint.set_anti_alias(true).set_color(Color::from_argb((e.a * 255.0).round() as u8, e.fill[0], e.fill[1], e.fill[2]));
        let r = e.radius as f32;
        if r > 0.0 {
            c.draw_rrect(RRect::new_rect_xy(Rect::from_wh(w as f32, h as f32), r, r), &paint);
        } else {
            c.draw_rect(Rect::from_wh(w as f32, h as f32), &paint);
        }
    })?;
    if tm.common.enter.as_deref() == Some("wipe") {
        let u = expo_out(tm.prog(Some(tm.common.d.unwrap_or(0.5))));
        if u <= 0.0 {
            return Ok(());
        }
        let cut = ((e.w * u) as i32).max(1);
        let wiped = Layer::render(cv, w, h, |c| {
            c.clip_rect(Rect::from_wh(cut as f32, h as f32), None, false);
            c.draw_image(layer.image(), (0, 0), None);
        })?;
        return blit(cv, &wiped, e.x, e.y, tm, Blit::new("fade").force_in("none"));
    }
    blit(cv, &layer, e.x, e.y, tm, Blit::new("fade"))
}


/// Pen rule under a document heading: the ink colour at 70%.
const RULE: Rgb = [6, 8, 13];

enum DocPart {
    Text(Rc<Layer>, i32, i32),
    Rule(i32),
    Highlight { x: i32, y: i32, w: i32, h: i32, color: Rgb },
}

/// A sheet of paper with a heading, a rule and text rows, some of which get a highlighter swipe.
pub fn doc(st: &Stage, cv: &Canvas, e: &Doc, tm: &Timing) -> Result<()> {
    let (w, h) = (e.w as i32, e.h as i32);
    let mut parts = Vec::new();
    let mut y = 30;
    if let Some(head) = &e.head {
        let caption = st.type_.layer(cv, &TextSpec::new(head.to_uppercase(), "avn_b", 26.0, [110, 100, 90]).tracking(3.0).align(Align::Left).maxw(Some(f64::from(w - 60))).pad(4))?;
        y += caption.height() + 8;
        parts.push(DocPart::Text(caption, 30, 30));
        parts.push(DocPart::Rule(y));
        y += 18;
    }
    for row in &e.rows {
        let text = st.type_.layer(cv, &TextSpec::new(&row.text, &row.font, row.size, row.fill.unwrap_or(INK)).align(Align::Left).maxw(Some(f64::from(w - 70))).pad(4))?;
        if let Some(hl) = row.hl {
            let u = expo_out((tm.t - hl) / 0.6);
            if u > 0.0 {
                parts.push(DocPart::Highlight { x: 28, y: y + 4, w: (f64::from(w - 56) * u) as i32, h: text.height() - 6, color: row.hlc.unwrap_or([255, 214, 120]) });
            }
        }
        let step = text.height() - 4;
        parts.push(DocPart::Text(text, 30, y));
        y += step;
    }
    let sheet = paper(st, cv, w, h)?;
    let layer = Layer::render(cv, w, h, |c| {
        over(c, &sheet, 0, 0, 1.0);
        for part in &parts {
            match part {
                DocPart::Text(text, x, y) => over(c, text, *x, *y, 1.0),
                DocPart::Rule(y) => {
                    c.draw_rect(Rect::from_xywh(30.0, *y as f32, (w - 60) as f32, 3.0), &fill(RULE));
                }
                DocPart::Highlight { x, y, w, h, color } => {
                    c.draw_rect(Rect::from_xywh(*x as f32, *y as f32, *w as f32, *h as f32), &fill_a(*color, 0.8));
                }
            }
        }
    })?;
    blit(cv, &layer, e.x, e.y, tm, Blit::new("rise").rot(e.rot).shadow(Shadow::new(26, 30.0, 0.55)))
}

/// A vertical list of account rows, one of which is swiped in red.
pub fn list(st: &Stage, cv: &Canvas, e: &List, tm: &Timing) -> Result<()> {
    let (w, rh) = (e.w as i32, e.rh as i32);
    let h = rh * e.rows.len() as i32 + 30;
    let base = panel(st, cv, &Panel { w, h, radius: 18.0, fill: [18, 26, 38], fill_a: 0.94, border: [84, 106, 134], accent: None })?;
    let mut rows = Vec::new();
    for (i, row) in e.rows.iter().enumerate() {
        let y = 15 + i as i32 * rh;
        let lit = i as i64 == e.hi && e.hi_at.is_some();
        let landed = lit && e.hi_at.is_some_and(|at| tm.t >= at);
        let bar = if lit { expo_out((tm.t - e.hi_at.unwrap_or(0.0)) / 0.5) } else { 0.0 };
        let name = st.type_.layer(cv, &TextSpec::new(row.first().map_or("", String::as_str), "mono_b", 30.0, WHITE).align(Align::Left).pad(4))?;
        let value = match row.get(1) {
            Some(v) => Some(st.type_.layer(cv, &TextSpec::new(v, "avn_m", 26.0, if landed { WHITE } else { STEEL }).align(Align::Right).pad(4))?),
            None => None,
        };
        rows.push((y, bar, name, value));
    }
    let layer = Layer::render(cv, w, h, |c| {
        over(c, &base, 0, 0, 1.0);
        for (y, bar, name, value) in &rows {
            if *bar > 0.0 {
                c.draw_rect(Rect::from_xywh(12.0, (*y + 4) as f32, (f64::from(w - 24) * bar) as i32 as f32, (rh - 8) as f32), &fill_a(RED, 0.9));
            }
            over(c, name, 26, y + (rh - name.height()) / 2, 1.0);
            if let Some(v) = value {
                over(c, v, w - v.width() - 26, y + (rh - v.height()) / 2, 1.0);
            }
        }
    })?;
    blit(cv, &layer, e.x, e.y, tm, Blit::new("slide_r").shadow(Shadow::new(20, 24.0, 0.5)))
}

/// A stock-ticker card: name, big price, signed change and share count; the accent turns red when the price is under its base.
pub fn ticker(st: &Stage, cv: &Canvas, e: &Ticker, tm: &Timing) -> Result<()> {
    let (w, h) = (e.w as i32, e.h as i32);
    let price = keyed(&e.price, tm.t);
    let base = e.base.unwrap_or_else(|| e.price.first().map_or(0.0, |k| k.1));
    let delta = price - base;
    let up = delta >= -1e-6;
    let accent = if up { GREEN } else { RED };
    let card = panel(st, cv, &Panel { w, h, radius: 24.0, fill: [16, 22, 33], fill_a: 0.95, border: [84, 106, 134], accent: Some(accent) })?;
    let line = |text: String, font: &str, size: f64, fill: Rgb, tracking: f64, align: Align| st.type_.layer(cv, &TextSpec::new(text, font, size, fill).tracking(tracking).align(align).pad(4));
    let name = line(e.name.clone(), "avn_b", 28.0, STEEL, 3.0, Align::Left)?;
    let big = line(format!("${}", pyfmt::thousands(price, 2)), "dinc", 138.0, WHITE, 0.0, Align::Left)?;
    let change = line(format!("{} {}${}", if up { "▲" } else { "▼" }, if delta >= 0.0 { "+" } else { "−" }, pyfmt::thousands(delta.abs(), 2)), "avn_b", 36.0, accent, 0.0, Align::Left)?;
    let shares = e.shares.as_ref().map(|s| line(s.clone(), "avn_m", 28.0, STEEL, 1.0, Align::Right)).transpose()?;
    let layer = Layer::render(cv, w, h, |c| {
        over(c, &card, 0, 0, 1.0);
        over(c, &name, 34, 20, 1.0);
        over(c, &big, 28, 56, 1.0);
        over(c, &change, 34, h - change.height() - 18, 1.0);
        if let Some(s) = &shares {
            over(c, s, w - s.width() - 28, h - s.height() - 22, 1.0);
        }
    })?;
    blit(cv, &layer, e.x, e.y, tm, Blit::new("rise").shadow(Shadow::new(22, 26.0, 0.5)))
}
