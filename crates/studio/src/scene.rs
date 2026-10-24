//! One scene as an image: the backdrop, every live element in plan order, the camera move and shake, then the scene's
//! own effects (an adjustment layer over everything).

use anyhow::{Context, Result};
use skia_safe::{Canvas, Image, Paint, Rect, TileMode};

use super::{elements, model::{Scene, H, W}, stage::{Stage}};
use turbomotion_core::{effects, ease::{sine_io}, geom::{Mat3}, layer::{linear, Layer}};

/// Elements are drawn from their start until `t1` (plus a hair, so exit animations finish).
fn live(e: &super::model::Element, t: f64) -> bool {
    t >= e.common.t0 - 0.001 && e.common.t1.map_or(true, |t1| t <= t1 + 0.01)
}

pub fn render(st: &Stage, like: &Canvas, scene: &Scene, t: f64) -> Result<Image> {
    let flat = Layer::try_render(like, W, H, |c| {
        st.backdrop.draw(&st.shaders, c, &scene.bg, t)?;
        scene.els.iter().filter(|e| live(e, t)).try_for_each(|el| elements::draw(st, c, el, t).with_context(|| format!("scene {:?}", scene.id)))
    })?;
    let framed = match camera(scene, t) {
        Some(m) => Layer::render(like, W, H, |c| camera_pass(c, &flat, &m))?,
        None => flat,
    };
    let adjusted = effects::apply(&st.shaders.core, like, framed, &scene.fx, t).with_context(|| format!("scene {:?}", scene.id))?;
    Ok(adjusted.image().clone())
}

/// Zoom, pan, roll and shake at `t` as the matrix the reference builds, or `None` when the camera is at rest.
fn camera(scene: &Scene, t: f64) -> Option<Mat3> {
    if scene.cam.is_empty() && scene.shakes.is_empty() {
        return None;
    }
    let (mut z, mut cx, mut cy, mut rot) = (1.0, 0.5, 0.5, 0.0);
    if let (Some(first), Some(last)) = (scene.cam.first(), scene.cam.last()) {
        let at = |k: &[f64; 5]| (k[1], k[2], k[3], k[4]);
        if t <= first[0] {
            (z, cx, cy, rot) = at(first);
        } else if t >= last[0] {
            (z, cx, cy, rot) = at(last);
        } else if let Some(pair) = scene.cam.windows(2).find(|p| p[0][0] <= t && t < p[1][0]) {
            let u = sine_io((t - pair[0][0]) / (pair[1][0] - pair[0][0]).max(1e-3));
            let mix = |i: usize| pair[0][i] + (pair[1][i] - pair[0][i]) * u;
            (z, cx, cy, rot) = (mix(1), mix(2), mix(3), mix(4));
        }
    }
    let (mut dx, mut dy) = (0.0, 0.0);
    for &[ts, amp] in &scene.shakes {
        let u = t - ts;
        if (0.0..1.2).contains(&u) {
            let k = (1.0 - u / 1.2).powi(2) * amp;
            dx += (u * 83.0).sin() * k;
            dy += (u * 71.0).cos() * k * 0.8;
            rot += (u * 47.0).sin() * k * 0.05;
        }
    }
    let at_rest = (z - 1.0).abs() < 1e-4 && (cx - 0.5).abs() < 1e-4 && (cy - 0.5).abs() < 1e-4 && rot == 0.0 && dx == 0.0 && dy == 0.0;
    if at_rest {
        return None;
    }
    let (w, h) = (f64::from(W), f64::from(H));
    Some(Mat3::rotation_about(cx * w, cy * h, -rot, z).then_translate(w / 2.0 - cx * w + dx, h / 2.0 - cy * h + dy))
}

/// Redraw `flat` moved by the camera matrix; the frame edges mirror the picture like the reference's reflect border.
fn camera_pass(c: &Canvas, flat: &Layer, m: &Mat3) {
    let Some(shader) = flat.image().to_shader((TileMode::Mirror, TileMode::Mirror), linear(), None) else { return };
    let mut paint = Paint::default();
    paint.set_shader(shader.with_local_matrix(&m.to_skia()));
    c.draw_rect(Rect::from_wh(W as f32, H as f32), &paint);
}
