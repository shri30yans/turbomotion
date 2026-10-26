//! Lighting a flat 3D layer: its painted pixels become the albedo of the lighting shader.

use anyhow::{Context, Result};
use skia_safe::{Canvas, Paint, Rect};
use turbomotion_core::{color::Encoding, layer::{linear, Layer}};

use super::Ctx;
use crate::eval::{Frame, LayerState};

const MAX_LIGHTS: usize = 4;

pub fn apply(ctx: &Ctx, like: &Canvas, img: Layer, frame: &Frame, st: &LayerState) -> Result<Layer> {
    let Some(lit) = &st.lit else { return Ok(img) };
    if frame.lights.is_empty() {
        return Ok(img);
    }
    let (w, h) = (img.width(), img.height());
    let (mut pos, mut dir, mut color, mut params) = (vec![0.0f32; 3 * MAX_LIGHTS], vec![0.0f32; 3 * MAX_LIGHTS], vec![0.0f32; 3 * MAX_LIGHTS], vec![0.0f32; 4 * MAX_LIGHTS]);
    for (i, l) in frame.lights.iter().take(MAX_LIGHTS).enumerate() {
        for k in 0..3 {
            pos[i * 3 + k] = l.pos[k] as f32;
            dir[i * 3 + k] = l.dir[k] as f32;
            color[i * 3 + k] = l.color[k] as f32;
        }
        let kind = match l.kind.as_str() {
            "ambient" => 1.0,
            "point" => 2.0,
            "spot" => 3.0,
            _ => 4.0,
        };
        params[i * 4..i * 4 + 4].copy_from_slice(&[kind, l.radius as f32, l.cone.to_radians().cos() as f32, l.shine as f32]);
    }
    let eye = frame.camera.as_ref().map_or([f64::from(frame.width) / 2.0, f64::from(frame.height) / 2.0, -1.0e5], |c| c.pos);
    // float3x3 uniforms are column-major
    let m = &lit.inverse;
    let inv: Vec<f32> = [m[0], m[3], m[6], m[1], m[4], m[7], m[2], m[5], m[8]].iter().map(|v| *v as f32).collect();
    let f3 = |v: [f64; 3]| v.map(|x| x as f32).to_vec();
    let mat: Vec<f32> = lit.material.iter().map(|v| *v as f32).collect();
    let input = img.image().to_shader((skia_safe::TileMode::Clamp, skia_safe::TileMode::Clamp), linear(), None).context("the layer image has no shader form")?;
    // lights and surfaces are authored as colours the eye sees, so the shading multiplies display numbers
    let shader = Encoding::Display.shader(like, ctx.programs.lighting.instance_named(
        &[
            ("inv", &inv),
            ("origin", &f3(lit.origin)),
            ("axis_u", &f3(lit.axis_u)),
            ("axis_v", &f3(lit.axis_v)),
            ("eye", &f3(eye)),
            ("material", &mat),
            ("light_pos", &pos),
            ("light_dir", &dir),
            ("light_color", &color),
            ("light_params", &params),
        ],
        &[input],
    )?);
    Layer::try_render(like, w, h, |c| {
        let mut p = Paint::default();
        p.set_shader(shader);
        c.draw_rect(Rect::from_wh(w as f32, h as f32), &p);
        Ok(())
    })
}
