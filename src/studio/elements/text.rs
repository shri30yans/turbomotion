//! Type elements: plain text (with wipe and typewriter reveals) and the small caps chip.

use anyhow::{bail, Context, Result};
use skia_safe::{gradient_shader::GradientShaderColors, BlendMode, Canvas, Color, Color4f, Paint, PaintStyle, Point, RRect, Rect, Shader, TileMode};

use crate::studio::{
    anim::{blit, Blit, Timing},
    color::fill,
    ease::{self, clamp, expo_out},
    fx,
    layer::{linear, over, place, Layer, Place},
    model::{Chip, Counter, Stamp, Text, Title, INK},
    noise, pyfmt,
    rng::RandomState,
    shader::Uniforms,
    stage::Stage,
    typeset::{Align, TextSpec},
};

pub fn text(st: &Stage, cv: &Canvas, e: &Text, tm: &Timing) -> Result<()> {
    let spec = TextSpec::new(&e.text, &e.font, e.size, e.fill).tracking(e.tracking).align(Align::parse(&e.align)).maxw(e.maxw);
    let layer = st.type_.layer(cv, &spec)?;
    match tm.common.enter.as_deref() {
        Some("mask") => {
            let u = expo_out(tm.prog(Some(tm.common.d.unwrap_or(0.7))));
            if u <= 0.0 {
                return Ok(());
            }
            let edge = u * (f64::from(layer.width()) + 120.0) - 60.0;
            let wiped = wipe(cv, &layer, edge, 60.0)?;
            blit(cv, &wiped, e.x, e.y, tm, Blit::new("rise").force_in("none"))
        }
        Some("type") => {
            let chars: Vec<char> = e.text.chars().collect();
            let k = (chars.len() as f64 * clamp((tm.t - tm.t0()) / tm.common.d.unwrap_or(1.0).max(0.05))) as usize;
            if k == 0 {
                return Ok(());
            }
            let left = spec.clone().align(Align::Left);
            let full = st.type_.layer(cv, &left)?;
            let typed = st.type_.layer(cv, &TextSpec { text: chars[..k].iter().collect(), ..left })?;
            let frame = Layer::render(cv, full.width(), full.height(), |c| over(c, &typed, 0, 0, 1.0))?;
            blit(cv, &frame, e.x, e.y, tm, Blit::new("rise").force_in("none"))
        }
        _ => blit(cv, &layer, e.x, e.y, tm, Blit::new("rise").rot(e.rot)),
    }
}

/// Reveal `layer` from the left: fully visible up to `edge - soft`, fading to nothing at `edge`.
fn wipe(like: &Canvas, layer: &Layer, edge: f64, soft: f64) -> Result<Layer> {
    // gradient alpha 1 -> 0 across [edge - soft, edge] in pixel-index space (+0.5 for Skia's centres)
    let from = (edge - soft + 0.5) as f32;
    let to = (edge + 0.5) as f32;
    let ramp = Shader::linear_gradient(
        (Point::new(from, 0.0), Point::new(to, 0.0)),
        GradientShaderColors::ColorsInSpace(&[Color4f::new(0.0, 0.0, 0.0, 1.0), Color4f::new(0.0, 0.0, 0.0, 0.0)], None),
        None,
        TileMode::Clamp,
        None,
        None,
    );
    Layer::render(like, layer.width(), layer.height(), |c| {
        c.draw_image(layer.image(), (0, 0), None);
        let mut mask = Paint::default();
        mask.set_shader(ramp).set_blend_mode(BlendMode::DstIn);
        c.draw_rect(Rect::from_wh(layer.width() as f32, layer.height() as f32), &mask);
    })
}

pub fn chip(st: &Stage, cv: &Canvas, e: &Chip, tm: &Timing) -> Result<()> {
    let label = e.text.to_uppercase();
    let face = st.type_.fonts().face("avn_b", e.size)?;
    let (w, h, _) = face.size(&label, 3.0);
    let (w2, h2) = (w + 44, h + 26);
    let caption = st.type_.layer(cv, &TextSpec::new(&label, "avn_b", e.size, e.fill).tracking(3.0).pad(4))?;
    let layer = Layer::render(cv, w2, h2, |c| {
        let pill = RRect::new_rect_xy(Rect::from_wh(w2 as f32, h2 as f32), h2 as f32 / 2.0, h2 as f32 / 2.0);
        let mut fill = Paint::default();
        fill.set_anti_alias(true).set_color(Color::from_argb(215, INK[0], INK[1], INK[2]));
        c.draw_rrect(pill, &fill);
        let mut border = Paint::default();
        let inset = Rect::from_wh(w2 as f32, h2 as f32).with_inset((1.5, 1.5));
        border.set_anti_alias(true).set_style(skia_safe::PaintStyle::Stroke).set_stroke_width(3.0).set_color(Color::from_rgb(e.fill[0], e.fill[1], e.fill[2]));
        c.draw_rrect(RRect::new_rect_xy(inset, h2 as f32 / 2.0 - 1.5, h2 as f32 / 2.0 - 1.5), &border);
        over(c, &caption, (w2 - caption.width()) / 2, (h2 - caption.height()) / 2, 1.0);
    })?;
    let x = e.x.unwrap_or(70.0 + f64::from(w2) / 2.0);
    blit(cv, &layer, x, e.y, tm, Blit::new("slide_l"))
}

/// Extruded headline with optional halo and floor reflection.
pub fn title(st: &Stage, cv: &Canvas, e: &Title, tm: &Timing) -> Result<()> {
    let extruded = fx::extruded_text(st, cv, &e.text, &e.font, e.size, e.fill, e.depth, e.tracking)?;
    let base_h = extruded.height();
    let reflected;
    let mut layer = &*extruded;
    if let Some(strength) = e.refl.filter(|r| *r != 0.0) {
        reflected = fx::reflect(cv, layer, strength)?;
        layer = &reflected;
    }
    let y = if e.refl.is_some_and(|r| r != 0.0) { e.y + f64::from(layer.height() - base_h) / 2.0 } else { e.y };
    halo(st, cv, layer, e.glow, &Place::at(e.x, y).scale(e.scale).rot(e.rot), tm)?;
    blit(cv, layer, e.x, y, tm, Blit::new("tilt").rot(e.rot).scale(e.scale))
}

/// A number counting from `v0` to `v1`, extruded like a title, with a small settle bump when it lands.
pub fn counter(st: &Stage, cv: &Canvas, e: &Counter, tm: &Timing) -> Result<()> {
    let Some(eased) = ease::named(&e.ease, (tm.t - tm.t0() - e.delay) / e.count) else { bail!("unknown ease {:?}", e.ease) };
    let value = e.v0 + (e.v1 - e.v0) * eased;
    let label = format!("{}{}{}", e.prefix, pyfmt::thousands(value, e.dec), e.suffix);
    let layer = fx::extruded_text(st, cv, &label, &e.font, e.size, e.fill, e.depth, 0.0)?;
    halo(st, cv, &layer, e.glow, &Place::at(e.x, e.y), tm)?;
    let landed = tm.t - tm.t0() - e.count;
    let bump = if landed > 0.0 { 1.0 + 0.05 * (-landed * 6.0).exp() } else { 1.0 };
    blit(cv, &layer, e.x, e.y, tm, Blit::new("zoom").scale(bump * e.scale))
}

/// The coloured halo behind an extruded element, grown with its entrance and faded with its exit.
fn halo(st: &Stage, cv: &Canvas, layer: &Layer, color: Option<[u8; 3]>, at: &Place, tm: &Timing) -> Result<()> {
    let Some(color) = color else { return Ok(()) };
    const SIGMA: f64 = 34.0;
    fx::halo(st, cv, layer, color, at, SIGMA, 0.9 * tm.prog(None) * (1.0 - tm.out_prog()))
}

/// A rubber stamp that slams down from large, shakes the frame it lands on and bursts an ink-dust ring.
pub fn stamp(st: &Stage, cv: &Canvas, e: &Stamp, tm: &Timing) -> Result<()> {
    const PAD: i32 = 34;
    let u = tm.t - tm.t0();
    if u < 0.0 {
        return Ok(());
    }
    let face = st.type_.fonts().face(&e.font, e.size)?;
    let (w, h, _) = face.size(&e.text, e.tracking);
    let (ww, hh) = (w + PAD * 2 + 20, h + PAD * 2);
    let lettering = st.type_.layer(cv, &TextSpec::new(&e.text, &e.font, e.size, e.fill).tracking(e.tracking).pad(4))?;
    let plate = Layer::render(cv, ww, hh, |c| {
        let mut ink = fill(e.fill);
        ink.set_style(PaintStyle::Stroke);
        for (inset, width, radius) in [(9.5, 7.0, 10.5), (19.0, 2.0, 7.0)] {
            ink.set_stroke_width(width);
            let r = Rect::from_wh(ww as f32, hh as f32).with_inset((inset, inset));
            c.draw_rrect(RRect::new_rect_xy(r, radius, radius), &ink);
        }
        over(c, &lettering, (ww - lettering.width()) / 2, (hh - lettering.height()) / 2, 1.0);
    })?;
    let worn = worn_ink(st, cv, &plate, e.seed)?;
    let k = clamp(u / 0.18);
    let slam = 1.0 + 1.6 * (1.0 - expo_out(k));
    let exit = tm.out_prog();
    if exit >= 1.0 {
        return Ok(());
    }
    let opacity = clamp(u * 12.0) * (1.0 - exit);
    let shake = if 0.18 < u && u < 0.55 { ((u - 0.18) * 90.0).sin() * 7.0 * (1.0 - (u - 0.18) / 0.37) } else { 0.0 };
    place(cv, &worn, &Place::at(e.x + shake, e.y + shake * 0.6).scale(slam * e.scale).rot(e.rot).opacity(opacity));
    if 0.17 < u && u < 0.9 {
        let r = (u - 0.17) * 520.0;
        let strength = (1.0 - (u - 0.17) / 0.73) * 0.35 * 0.6; // 0.6 = how much of the ink colour the burst adds
        fx::light_ring(st, cv, (e.x, e.y), r, 26.0, 0.6, strength, e.fill)?;
    }
    Ok(())
}

/// `layer` with its alpha modulated by seeded, smooth noise (the stamp's uneven ink).
fn worn_ink(st: &Stage, cv: &Canvas, layer: &Layer, seed: u32) -> Result<Layer> {
    let (w, h) = (layer.width(), layer.height());
    let small = noise::uniform(&mut RandomState::new(seed), (w / 3 + 1) as usize, (h / 3 + 1) as usize)?;
    let field = noise::stretch(cv, &small, w, h)?;
    let sample = |l: &Layer| l.image().to_shader((TileMode::Clamp, TileMode::Clamp), linear(), None).context("image shader");
    let wear = st.shaders.wear.instance(Uniforms::new(), &[sample(layer)?, sample(&field)?])?;
    Layer::render(cv, w, h, |c| {
        let mut paint = Paint::default();
        paint.set_shader(wear);
        c.draw_rect(Rect::from_wh(w as f32, h as f32), &paint);
    })
}
