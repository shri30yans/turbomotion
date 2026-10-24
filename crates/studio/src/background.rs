//! The scene backdrop: a radial gradient (rendered once per style), a perspective floor grid, floating dust and a slow
//! light sweep. Mirrors `Scene.background` and the `grid_apply` / `dust_apply` / `sweep_apply` functions of the reference.

use anyhow::Result;
use skia_safe::{BlendMode, Canvas, Color, Paint, PaintStyle, Rect};

use super::{cache::{Cache}, color::{skia_a}, model::{Background, Rgb, H, INK, INK2, STEEL, W}, rng::{RandomState}, shaders::{Shaders}};
use turbomotion_core::{layer::{Layer}, shader::{Uniforms}};

/// Gradient colours of a backdrop style: (centre, edge, centre point, radius), or `None` for the flat `plain` style.
fn style(name: &str) -> Option<(Rgb, Rgb, (f64, f64))> {
    match name {
        "studio" => Some((INK2, INK, (0.5, 0.46))),
        "deep" => Some(([22, 14, 16], [6, 6, 9], (0.5, 0.5))),
        "steel" => Some(([26, 38, 54], [8, 12, 18], (0.5, 0.46))),
        "paper" => Some(([60, 56, 50], [20, 18, 17], (0.5, 0.5))),
        _ => None,
    }
}

pub struct Backdrop {
    gradients: Cache<String, Layer>,
}

impl Backdrop {
    pub fn new() -> Self {
        Self { gradients: Cache::new(8) }
    }

    pub fn draw(&self, shaders: &Shaders, cv: &Canvas, bg: &Background, t: f64) -> Result<()> {
        let full = Rect::from_wh(W as f32, H as f32);
        match style(&bg.style) {
            None => {
                cv.clear(Color::from_rgb(INK[0], INK[1], INK[2]));
                return Ok(());
            }
            Some((c_in, c_out, centre)) => {
                let gradient = self.gradients.get_or_try_insert(bg.style.clone(), || {
                    let shader = shaders.radial.instance(
                        Uniforms::new().v2(f64::from(W), f64::from(H)).rgb(c_in).rgb(c_out).v2(centre.0, centre.1).f(0.9),
                        &[],
                    )?;
                    Layer::render(cv, W, H, |c| {
                        let mut paint = Paint::default();
                        paint.set_shader(shader);
                        c.draw_rect(full, &paint);
                    })
                })?;
                cv.draw_image(gradient.image(), (0, 0), None);
            }
        }
        if bg.grid {
            floor_grid(cv, t, bg.grid_a)?;
        }
        if bg.dust {
            dust(shaders, cv, t, bg.seed)?;
        }
        if bg.sweep {
            sweep(shaders, cv, t, bg.period)?;
        }
        Ok(())
    }
}

/// Perspective floor grid below the horizon: static converging verticals plus horizontals that scroll toward the viewer.
/// The lines are drawn into one mask first (a later line replaces an earlier one where they cross, they never add up) and
/// the mask is then laid over the backdrop in the grid colour.
fn floor_grid(cv: &Canvas, t: f64, alpha: f64) -> Result<()> {
    const HORIZON: f64 = 0.60;
    const SPEED: f64 = 0.12;
    const SPACING: f64 = 110.0;
    let hy = (f64::from(H) * HORIZON) as i32;
    let rows = H - hy;
    let mask = Layer::render(cv, W, rows + 12, |c| {
        let mut line = Paint::default();
        line.set_stroke_width(2.0).set_style(PaintStyle::Stroke).set_blend_mode(BlendMode::Src);
        for i in -24i32..=24 {
            let a = alpha * (1.0 - f64::from(i.abs()) / 28.0);
            line.set_anti_alias(false).set_color4f(skia_a(STEEL, a), None);
            let x_top = f64::from(W) / 2.0 + f64::from(i) * 6.0;
            let x_bottom = f64::from(W) / 2.0 + f64::from(i) * SPACING * 1.7;
            c.draw_line((x_top as f32, 0.0), (x_bottom as f32, (rows + 10) as f32), &line);
        }
        let scroll = (t * SPEED * 10.0).rem_euclid(1.0);
        line.set_anti_alias(true);
        for j in 0..22 {
            let z = (f64::from(j) + scroll) / 22.0;
            let y = (f64::from(rows) * z.powf(2.2)).round_ties_even() as f32 + 0.5;
            line.set_color4f(skia_a(STEEL, alpha * (z * 3.0).min(1.0)), None);
            c.draw_line((0.0, y), (W as f32, y), &line);
        }
    })?;
    cv.draw_image(mask.image(), (0, hy), None);
    Ok(())
}

/// Slow floating dust, seeded like the reference so the particles sit where the Python frames put them.
fn dust(shaders: &Shaders, cv: &Canvas, t: f64, seed: u32) -> Result<()> {
    const COUNT: usize = 70;
    const ALPHA: f64 = 0.35;
    const SIZE: f64 = 2.2;
    let mut rng = RandomState::new(seed);
    let (mut dots, mut extra) = (Vec::with_capacity(96), Vec::with_capacity(96));
    for _ in 0..COUNT {
        let (x0, y0) = (rng.rand() * f64::from(W), rng.rand() * f64::from(H));
        let speed = 8.0 + rng.rand() * 22.0;
        let phase = rng.rand() * 6.28;
        let size = SIZE * (0.5 + rng.rand() * 1.6);
        let x = (x0 + (t * 0.3 + phase).sin() * 40.0 + t * speed * 0.3).rem_euclid(f64::from(W));
        let y = (y0 - t * speed).rem_euclid(f64::from(H));
        let strength = ALPHA * (0.35 + 0.65 * (t * 0.7 + phase).sin().abs()) * 0.9;
        let rad = (size * 2.0).round_ties_even() / 2.0 + 0.5;
        let support = (rad * 2.0 + 4.0) as i64 | 1;
        dots.push([x.floor(), y.floor(), rad * 0.62, strength]);
        extra.push([(support / 2) as f64, 0.0, 0.0, 0.0]);
    }
    dots.resize(96, [0.0; 4]);
    extra.resize(96, [0.0; 4]);
    let shader = shaders.dust.instance(Uniforms::new().f(COUNT as f64).rows4(&dots).rows4(&extra), &[])?;
    let mut paint = Paint::default();
    paint.set_shader(shader);
    cv.draw_rect(Rect::from_wh(W as f32, H as f32), &paint);
    Ok(())
}

fn sweep(shaders: &Shaders, cv: &Canvas, t: f64, period: f64) -> Result<()> {
    const ANGLE: f64 = -18.0;
    const WIDTH: f64 = 360.0;
    const STRENGTH: f64 = 0.08 * 0.9;
    let u = t.rem_euclid(period) / period;
    let centre = u * (f64::from(W) + 1600.0) - 800.0 - f64::from(W) / 2.0;
    let (sin, cos) = ANGLE.to_radians().sin_cos();
    let shader = shaders.sweep.instance(Uniforms::new().v2(f64::from(W), f64::from(H)).v2(cos, sin).f(centre).f(WIDTH * 0.45).f(STRENGTH), &[])?;
    let mut paint = Paint::default();
    paint.set_shader(shader).set_blender(shaders.core.add_clip.blend(Uniforms::new())?);
    cv.draw_rect(Rect::from_wh(W as f32, H as f32), &paint);
    Ok(())
}
