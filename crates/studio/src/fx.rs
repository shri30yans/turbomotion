//! Layer effects: perspective tilt, halo glow, faux-3D extruded type and floor reflections.

use std::rc::Rc;

use anyhow::{Context, Result};
use skia_safe::{color_filters, gradient_shader::GradientShaderColors, BlendMode, Canvas, Color4f, Matrix, Paint, Point, Rect, Shader, TileMode};

use super::{color::{mix, skia}, model::{Rgb, H, W}, stage::{Stage}, typeset::{Align, TextSpec}};
use turbomotion_core::{geom::{Mat3}, layer::{draw_warped, linear, place, solid_color, Layer, Place}, shader::{Uniforms}};

/// Tilt a layer about its centre (`rx`, `ry` degrees about the x and y axes) with a perspective falloff `persp`, on a
/// canvas padded so the foreshortened corners fit (the reference `tilt`).
pub fn tilt(like: &Canvas, layer: &Layer, rx: f64, ry: f64, persp: f64) -> Result<Layer> {
    let (w, h) = (f64::from(layer.width()), f64::from(layer.height()));
    let pad = (w.max(h) * 0.35) as i32;
    let (w2, h2) = (layer.width() + 2 * pad, layer.height() + 2 * pad);
    let (cx, cy) = (f64::from(w2) / 2.0, f64::from(h2) / 2.0);
    let (ax, ay) = (rx.to_radians(), ry.to_radians());
    let p = f64::from(pad);
    let corners = [(p, p), (p + w, p), (p + w, p + h), (p, p + h)];
    let mut moved = corners;
    for (i, &(x, y)) in corners.iter().enumerate() {
        let (px, py) = (x - cx, y - cy);
        let y2 = py * ax.cos();
        let z2 = py * ax.sin();
        let x2 = px * ay.cos() + z2 * ay.sin();
        let z3 = -px * ay.sin() + z2 * ay.cos();
        let f = 1.0 / (1.0 - persp * z3);
        moved[i] = (cx + x2 * f, cy + y2 * f);
    }
    let Some(m) = Mat3::perspective(corners, moved) else { return Layer::render(like, w2, h2, |_| {}) };
    let m = m.mul(&Mat3::translate(p, p)); // layer pixels -> padded canvas, then the homography
    Layer::render(like, w2, h2, |c| draw_warped(c, layer, &m, 1.0))
}

/// Faux-3D type: `text` extruded down and to the right in `depth` stepped copies (darker toward the back), with a lit
/// face and bevel on top. Light comes from the upper left. Cached per (text, face, size, depth, tracking).
pub fn extruded_text(st: &Stage, cv: &Canvas, text: &str, font: &str, size: f64, face: Rgb, depth: f64, tracking: f64) -> Result<Rc<Layer>> {
    const EXTRUSION_DIR: (f64, f64) = (0.58, 0.81);
    const BEVEL_SIGMA: f64 = 3.0;
    let key = (text.to_string(), font.to_string(), size as i32, face, depth as i32, (tracking * 100.0) as i64);
    st.extruded.get_or_try_insert(key, || {
        let pad = depth as i32 + 40;
        let mask = st.type_.layer(cv, &TextSpec::new(text, font, size, [255, 255, 255]).tracking(tracking).pad(pad).align(Align::Left))?;
        let soft = mask.blurred(cv, BEVEL_SIGMA)?;
        let side = mix(face, [0, 0, 0], 0.62);
        let back = mix(side, [0, 0, 0], 0.5);
        let steps = (depth as i32).max(8);
        let (w, h) = (mask.width(), mask.height());
        let lit = st.shaders.extrude_face.instance(
            Uniforms::new().f(f64::from(h)).rgb(face).v2(-0.6, -0.8),
            &[shader_of(mask.image())?, shader_of(soft.image())?],
        )?;
        Layer::try_render(cv, w, h, |c| {
            for i in (1..=steps).rev() {
                let k = f64::from(i) / f64::from(steps);
                let (ox, oy) = ((EXTRUSION_DIR.0 * depth * k).round_ties_even(), (EXTRUSION_DIR.1 * depth * k).round_ties_even());
                let mut paint = Paint::default();
                paint.set_color_filter(color_filters::blend(skia(mix(side, back, k)), BlendMode::SrcIn));
                c.draw_image(mask.image(), (ox as f32, oy as f32), Some(&paint));
            }
            let mut paint = Paint::default();
            paint.set_shader(lit);
            c.draw_rect(Rect::from_wh(w as f32, h as f32), &paint);
            Ok(())
        })
    })
    .map_err(Into::into)
}

fn shader_of(image: &skia_safe::Image) -> Result<skia_safe::Shader> {
    image.to_shader((TileMode::Clamp, TileMode::Clamp), linear(), None).context("image shader")
}

/// The layer with a faded, softened mirror image beneath it, as a floor reflection.
pub fn reflect(like: &Canvas, layer: &Layer, strength: f64) -> Result<Layer> {
    const FADE: f64 = 0.55;
    const GAP: i32 = 6;
    const SOFTEN: f64 = 2.5;
    let (w, h) = (layer.width(), layer.height());
    let n = (f64::from(h) * FADE) as i32;
    let ramp = Shader::linear_gradient(
        (Point::new(0.0, 0.0), Point::new(0.0, (n - 1).max(1) as f32)),
        GradientShaderColors::ColorsInSpace(&[Color4f::new(0.0, 0.0, 0.0, strength as f32), Color4f::new(0.0, 0.0, 0.0, 0.0)], None),
        None,
        TileMode::Clamp,
        None,
        None,
    );
    let mirror = Layer::render(like, w, n, |c| {
        c.clip_rect(Rect::from_wh(w as f32, n as f32), None, false);
        c.concat(&Matrix::new_all(1.0, 0.0, 0.0, 0.0, -1.0, h as f32, 0.0, 0.0, 1.0));
        c.draw_image(layer.image(), (0, 0), None);
    })?;
    let faded = Layer::render(like, w, n, |c| {
        c.draw_image(mirror.image(), (0, 0), None);
        let mut mask = Paint::default();
        mask.set_shader(ramp).set_blend_mode(BlendMode::DstIn);
        c.draw_rect(Rect::from_wh(w as f32, n as f32), &mask);
    })?;
    let soft = faded.blurred(like, SOFTEN)?;
    Layer::render(like, w, h + n + GAP, |c| {
        c.draw_image(layer.image(), (0, 0), None);
        c.draw_image(soft.image(), (0, h + GAP), None);
    })
}

/// Additive coloured halo behind an element: the layer's silhouette in `color`, blurred by `sigma` and added to the
/// canvas at `strength`.
pub fn halo(st: &Stage, cv: &Canvas, layer: &Layer, color: Rgb, at: &Place, sigma: f64, strength: f64) -> Result<()> {
    if strength <= 0.0 {
        return Ok(());
    }
    let mut tint = Paint::default();
    tint.set_color_filter(solid_color(color, 1.0));
    let silhouette = Layer::render(cv, layer.width(), layer.height(), |c| {
        c.draw_image(layer.image(), (0, 0), Some(&tint));
    })?;
    let spread = Layer::render(cv, W, H, |c| place(c, &silhouette, at))?.blurred(cv, sigma)?;
    let mut paint = Paint::default();
    paint.set_alpha_f(strength as f32).set_blender(st.shaders.core.add_clip.blend(Uniforms::new())?);
    cv.draw_image(spread.image(), (0, 0), Some(&paint));
    Ok(())
}

/// A soft elliptical ring of light centred on `at`: radius `r`, falloff `width`, vertical squash `aspect`, added to the
/// canvas in `color`. The stamp's ink-dust burst.
pub fn light_ring(st: &Stage, cv: &Canvas, at: (f64, f64), r: f64, width: f64, aspect: f64, strength: f64, color: Rgb) -> Result<()> {
    let shader = st.shaders.ring.instance(
        Uniforms::new().v2(at.0, at.1).f(r).f(width).f(aspect).f(strength).rgb(color).f(1.0),
        &[],
    )?;
    let mut paint = Paint::default();
    paint.set_shader(shader).set_blender(st.shaders.core.add_clip.blend(Uniforms::new())?);
    let (rx, ry) = (r + 60.0, r * aspect + 60.0);
    cv.draw_rect(Rect::from_ltrb((at.0 - rx) as f32, (at.1 - ry) as f32, (at.0 + rx) as f32, (at.1 + ry) as f32), &paint);
    Ok(())
}

/// A soft round sprite (`bokeh.sksl`) drawn into the square `size` px wide whose top-left pixel is `corner`.
#[derive(Debug, Clone, Copy)]
pub struct Bokeh {
    pub color: Rgb,
    /// (core radius, core width, core amplitude, colour gain from the core)
    pub core: [f64; 4],
    /// (halo sigma, halo amplitude, ring radius, ring width)
    pub halo: [f64; 4],
    /// (ring amplitude, output exponent)
    pub ring: [f64; 2],
    pub colour_base: f64,
    pub size: i32,
}

impl Bokeh {
    /// A radial pulse: opacity `(1 - d / 18)^1.5` out to 18 px (the pulse riding an arrow).
    pub fn pulse(color: Rgb) -> Self {
        Self { color, core: [0.0, 18.0, 1.0, 0.0], halo: [1.0, 0.0, 0.0, 1.0], ring: [0.0, 1.5], colour_base: 1.0, size: 36 }
    }

    /// A travelling money packet of radius `r`: bright core, wide halo, core-brightened colour.
    pub fn packet(color: Rgb, r: f64) -> Self {
        Self { color, core: [r * 0.8, 2.0, 1.0, 0.4], halo: [r * 1.6, 0.6, 0.0, 1.0], ring: [0.0, 1.0], colour_base: 0.8, size: (r * 3.0) as i32 }
    }

    /// The glowing head of a growing line chart.
    pub fn chart_head(color: Rgb) -> Self {
        Self { color, core: [6.0, 3.0, 1.0, 0.0], halo: [16.0, 0.9, 0.0, 1.0], ring: [0.0, 1.0], colour_base: 1.0, size: 80 }
    }

    /// A packet riding an arc across the globe.
    pub fn globe_packet(color: Rgb) -> Self {
        Self { color, core: [5.0, 2.5, 1.0, 0.0], halo: [9.0, 1.0, 0.0, 1.0], ring: [0.0, 1.0], colour_base: 1.0, size: 70 }
    }

    /// A pin on the globe.
    pub fn globe_pin(color: Rgb) -> Self {
        Self { color, core: [7.0, 3.0, 1.0, 0.0], halo: [13.0, 0.9, 0.0, 1.0], ring: [0.0, 1.0], colour_base: 1.0, size: 120 }
    }

    /// A pin on a flat map: core, halo and an expanding pulse ring (`pulse` in 0..1, 0 = none).
    pub fn map_pin(color: Rgb, r: f64, pulse: f64) -> Self {
        let ring = if pulse > 0.0 { 0.8 * (1.0 - pulse) } else { 0.0 };
        Self { color, core: [r * 0.6, r * 0.5, 1.0, 0.0], halo: [r * 2.2, 0.55, r * (1.4 + 2.4 * pulse), r * 0.35], ring: [ring, 1.0], colour_base: 1.0, size: (r * 6.0) as i32 * 2 }
    }

    /// Draw centred on (`x`, `y`) with the square's top-left snapped to whole pixels like the reference.
    pub fn draw(&self, st: &Stage, cv: &Canvas, x: f64, y: f64, opacity: f64) -> Result<()> {
        if opacity <= 0.0 {
            return Ok(());
        }
        let half = f64::from(self.size) / 2.0;
        let (left, top) = ((x - half) as i32, (y - half) as i32);
        let shader = st.shaders.bokeh.instance(
            Uniforms::new()
                .v2(half, half)
                .rgb(self.color)
                .v4(self.core[0], self.core[1], self.core[2], self.core[3])
                .v4(self.halo[0], self.halo[1], self.halo[2], self.halo[3])
                .v2(self.ring[0], self.ring[1])
                .f(opacity)
                .f(self.colour_base),
            &[],
        )?;
        let mut paint = Paint::default();
        paint.set_shader(shader);
        cv.save();
        cv.translate((left as f32, top as f32));
        cv.draw_rect(Rect::from_wh(self.size as f32, self.size as f32), &paint);
        cv.restore();
        Ok(())
    }
}
