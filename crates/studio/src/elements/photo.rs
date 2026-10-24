//! Photographs and characters: a full-frame plate with a slow push-in and soft-focus edges, and cut-out characters
//! standing on a contact shadow.

use anyhow::{anyhow, bail, Result};
use skia_safe::{gradient_shader, Canvas, Color4f, Matrix, Paint, Rect, TileMode};

use crate::{{anim::{blit, Blit, Entrance, Timing}, model::{CharEl, KenBurns, Plate, H, W}, stage::{Stage}}};
use turbomotion_core::{ease::{clamp, sine_io}, geom::{Mat3}, grade, layer::{draw_warped, linear, place, Layer, Place}};

/// The Ken Burns move used when a plate names none: a 12% push-in on the centre.
const DEFAULT_MOVE: KenBurns = [[1.0, 0.5, 0.5], [1.12, 0.5, 0.5]];
const DEFAULT_FOCUS: [f64; 2] = [0.5, 0.55];

pub fn plate(st: &Stage, cv: &Canvas, e: &Plate, tm: &Timing) -> Result<()> {
    let photo = st.media.plate(&e.name)?;
    let (pw, ph) = (f64::from(photo.width()), f64::from(photo.height()));
    let [from, to] = e.kb.unwrap_or(DEFAULT_MOVE);
    let u = sine_io(clamp((tm.t - tm.t0()) / e.dur.max(0.1)));
    let [zoom, cx, cy] = [0, 1, 2].map(|i| from[i] + (to[i] - from[i]) * u);
    let crop_w = (pw / zoom).min(pw);
    let crop_h = crop_w * 9.0 / 16.0;
    let x0 = (cx * pw - crop_w / 2.0).clamp(0.0, pw - crop_w);
    let y0 = (cy * ph - crop_h / 2.0).clamp(0.0, ph - crop_h);
    let scale = f64::from(W) / crop_w;
    let to_frame = Mat3([[scale, 0.0, -x0 * scale], [0.0, f64::from(H) / crop_h, -y0 * f64::from(H) / crop_h], [0.0, 0.0, 1.0]]);

    let sharp = Layer::render(cv, W, H, |c| draw_warped(c, &Layer::from_image((*photo).clone()), &to_frame, 1.0))?;
    let soft = sharp.blurred_with_edge(cv, e.dof, TileMode::Clamp)?;
    let focus = e.focus.unwrap_or(DEFAULT_FOCUS);
    let shader = st.shaders.plate.instance(
        grade::uniforms(&e.grade).v2(focus[0], focus[1]).v2(f64::from(W), f64::from(H)),
        &[
            sharp.image().to_shader((TileMode::Clamp, TileMode::Clamp), linear(), None).ok_or_else(|| anyhow!("plate shader"))?,
            soft.image().to_shader((TileMode::Clamp, TileMode::Clamp), linear(), None).ok_or_else(|| anyhow!("plate shader"))?,
        ],
    )?;
    let framed = Layer::render(cv, W, H, |c| {
        let mut paint = Paint::default();
        paint.set_shader(shader);
        c.draw_rect(Rect::from_wh(W as f32, H as f32), &paint);
    })?;
    blit(cv, &framed, f64::from(W) / 2.0, f64::from(H) / 2.0, tm, Blit::new("none").force_in("none"))?;
    if e.scrim > 0.0 {
        draw_scrim(cv, e.scrim);
    }
    Ok(())
}

/// A black veil fading from `strength` at the top of the frame to a fifth of that at the bottom.
fn draw_scrim(cv: &Canvas, strength: f64) {
    let (top, bottom) = (Color4f::new(0.0, 0.0, 0.0, strength as f32), Color4f::new(0.0, 0.0, 0.0, (strength * 0.2) as f32));
    let shader = gradient_shader::linear(((0.0, 0.5), (0.0, H as f32 - 0.5)), gradient_shader::GradientShaderColors::ColorsInSpace(&[top, bottom], None), None, TileMode::Clamp, None, None);
    let mut paint = Paint::default();
    paint.set_shader(shader);
    cv.draw_rect(Rect::from_wh(W as f32, H as f32), &paint);
}

/// Height of the contact-shadow strip under a character, and how far above the feet it sits.
const SHADOW_STRIP: i32 = 60;
const SHADOW_RISE: f64 = 36.0;

pub fn character(st: &Stage, cv: &Canvas, e: &CharEl, tm: &Timing) -> Result<()> {
    let Some(first) = e.poses.first() else { bail!("a character needs at least one pose") };
    let pose = e.poses.iter().rev().find(|(at, _)| *at <= tm.t).map_or(&first.1, |(_, name)| name);
    let u = tm.prog(None);
    if u <= 0.0 {
        return Ok(());
    }
    let image = st.media.character(pose, e.h as i32)?;
    let (w, h) = (image.width(), image.height());
    let sprite = Layer::render(cv, w, h, |c| {
        if e.flip {
            c.translate((w as f32, 0.0));
            c.scale((-1.0, 1.0));
        }
        c.draw_image(&*image, (0, 0), None);
    })?;
    let mut x = e.x.unwrap_or(f64::from(W) * 0.5) + (tm.t * 1.3).sin() * e.sway;
    let y = e.y.unwrap_or(f64::from(H) * 0.98);
    if e.shiver {
        x += (tm.t * 60.0).sin() * 4.0;
    }
    draw_contact_shadow(cv, x, y, w);
    let entrance = Entrance::of(tm.common.enter.as_deref().unwrap_or("rise"), u);
    place(cv, &sprite, &Place::at(x + entrance.dx, y - f64::from(h) / 2.0 + entrance.dy).scale(e.scale * entrance.sc).opacity(entrance.op * (1.0 - tm.out_prog())));
    Ok(())
}

/// A soft ellipse under the character's feet, 42% of its width across.
fn draw_contact_shadow(cv: &Canvas, x: f64, y: f64, width: i32) {
    let (rx, ry) = (f64::from(width) * 0.42, 22.0);
    let mut to_ellipse = Matrix::scale((rx as f32, ry as f32));
    to_ellipse.post_translate((width as f32 / 2.0 + 0.5, SHADOW_STRIP as f32 / 2.0 + 0.5));
    let (centre, edge) = (Color4f::new(0.0, 0.0, 0.0, 0.5), Color4f::new(0.0, 0.0, 0.0, 0.0));
    let shader = gradient_shader::radial((0.0, 0.0), 1.0, gradient_shader::GradientShaderColors::ColorsInSpace(&[centre, edge], None), None, TileMode::Clamp, None, &to_ellipse);
    let mut paint = Paint::default();
    paint.set_shader(shader);
    cv.save();
    cv.translate(((x - f64::from(width) / 2.0) as i32 as f32, (y - SHADOW_RISE) as i32 as f32));
    cv.draw_rect(Rect::from_wh(width as f32, SHADOW_STRIP as f32), &paint);
    cv.restore();
}
