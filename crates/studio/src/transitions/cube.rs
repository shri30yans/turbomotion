//! Rotating cube: the two frames as adjacent faces turning about the vertical axis, the far one dimming.

use std::f64::consts::FRAC_PI_2;

use anyhow::{anyhow, Result};
use skia_safe::{color_filters, Canvas, Color, Image, Paint, TileMode};

use super::{
    compose::{frame, sampled},
    Heading,
};
use crate::{{model::{H, W}, stage::{Stage}}};
use turbomotion_core::{ease::{pow_io}, geom::{Mat3}, layer::{Layer}, shader::{Uniforms}};

/// Distance of the eye from the cube's centre plane, in pixels.
const EYE: f64 = 900.0;

/// The perspective matrix that lays a frame on a face turned by `angle` about the vertical axis through the cube centre.
fn face(angle: f64) -> Result<Mat3> {
    let (w, h) = (f64::from(W), f64::from(H));
    let half = w / 2.0;
    let corners = [(-half, -h / 2.0), (half, -h / 2.0), (half, h / 2.0), (-half, h / 2.0)];
    let projected = corners.map(|(x, y)| {
        let sx = x * angle.cos() + half * angle.sin();
        let depth = -x * angle.sin() + half * angle.cos();
        let f = EYE / (EYE + (half - depth));
        (w / 2.0 + sx * f, h / 2.0 + y * f)
    });
    Mat3::perspective([(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)], projected).ok_or_else(|| anyhow!("degenerate cube face"))
}

/// Add `image` laid on `m` and dimmed to `light` onto the canvas without exceeding white.
fn add_face(st: &Stage, c: &Canvas, image: &Image, m: &Mat3, light: f64) -> Result<()> {
    let shader = sampled(image, TileMode::Decal)?;
    let dim = light as f32;
    #[rustfmt::skip]
    let scale = [
        dim, 0.0, 0.0, 0.0, 0.0,
        0.0, dim, 0.0, 0.0, 0.0,
        0.0, 0.0, dim, 0.0, 0.0,
        0.0, 0.0, 0.0, 1.0, 0.0,
    ];
    let mut paint = Paint::default();
    paint.set_shader(shader).set_color_filter(color_filters::matrix_row_major(&scale, None)).set_blender(st.shaders.core.add_clip.blend(Uniforms::new())?).set_anti_alias(false);
    c.save();
    c.concat(&m.to_skia());
    c.draw_rect(frame(), &paint);
    c.restore();
    Ok(())
}

pub fn cube(st: &Stage, like: &Canvas, a: &Image, b: &Image, u: f64, heading: Heading) -> Result<Image> {
    let k = pow_io(u, 3.0);
    let turn = k * FRAC_PI_2;
    let sign = heading.sign();
    let (face_a, face_b) = (face(-turn * sign)?, face((FRAC_PI_2 - turn) * sign)?);
    Ok(Layer::try_render(like, W, H, |c| {
        c.clear(Color::BLACK);
        add_face(st, c, a, &face_a, 1.0 - 0.5 * k)?;
        add_face(st, c, b, &face_b, 0.5 + 0.5 * k)
    })?
    .image()
    .clone())
}
