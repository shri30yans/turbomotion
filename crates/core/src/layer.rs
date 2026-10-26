//! Layers: half-float, premultiplied offscreen images that elements are built from and then placed on the scene canvas.
//! Everything that moves or fades is a layer drawn through `place`, exactly as the reference engine warps float arrays.

use anyhow::{Context, Result};
use skia_safe::{
    color_filters, image_filters, ColorFilter, AlphaType, Canvas, ColorType, FilterMode, Image, ImageInfo, ISize, MipmapMode, Paint, Rect, SamplingOptions, TileMode,
};

use super::geom::Mat3;

#[derive(Clone)]
pub struct Layer {
    image: Image,
}

pub fn linear() -> SamplingOptions {
    SamplingOptions::new(FilterMode::Linear, MipmapMode::None)
}

impl Layer {
    /// Draw into a new transparent `w` x `h` layer. The layer is compatible with `like`'s GPU context.
    pub fn render(like: &Canvas, w: i32, h: i32, draw: impl FnOnce(&Canvas)) -> Result<Self> {
        Self::try_render(like, w, h, |c| {
            draw(c);
            Ok(())
        })
    }

    /// Like `render`, for drawing that can fail.
    pub fn try_render(like: &Canvas, w: i32, h: i32, draw: impl FnOnce(&Canvas) -> Result<()>) -> Result<Self> {
        let info = ImageInfo::new(ISize::new(w.max(1), h.max(1)), ColorType::RGBAF16, AlphaType::Premul, None);
        let mut surface = like.new_surface(&info, None).with_context(|| format!("no GPU memory for a {w}x{h} layer"))?;
        surface.canvas().clear(skia_safe::Color::TRANSPARENT);
        draw(surface.canvas())?;
        Ok(Self { image: surface.image_snapshot() })
    }

    pub fn from_image(image: Image) -> Self {
        Self { image }
    }

    pub fn image(&self) -> &Image {
        &self.image
    }

    pub fn width(&self) -> i32 {
        self.image.width()
    }

    pub fn height(&self) -> i32 {
        self.image.height()
    }

    /// A Gaussian-blurred copy (transparent beyond the edges, so give the layer room for the blur).
    pub fn blurred(&self, like: &Canvas, sigma: f64) -> Result<Self> {
        self.blurred_with_edge(like, sigma, TileMode::Decal)
    }

    /// A Gaussian-blurred copy that treats the layer's edge pixels as continuing outward (`TileMode::Clamp`) or as
    /// transparent (`Decal`). Skia already filters a wide blur at reduced resolution; doing it again by hand was
    /// measured and gave no speed-up.
    pub fn blurred_with_edge(&self, like: &Canvas, sigma: f64, edge: TileMode) -> Result<Self> {
        if sigma <= 0.3 {
            return Ok(Self { image: self.image.clone() });
        }
        let blur = image_filters::blur((sigma as f32, sigma as f32), edge, None, None).context("blur filter")?;
        let mut paint = Paint::default();
        paint.set_image_filter(blur);
        Self::render(like, self.width(), self.height(), |c| {
            c.draw_image(&self.image, (0, 0), Some(&paint));
        })
    }

    /// The layer's alpha as a black silhouette of `opacity`, blurred by `sigma` and moved by (`dx`, `dy`) inside a
    /// canvas padded by `pad`, with the layer itself drawn on top: the reference `drop_shadow`.
    pub fn with_shadow(&self, like: &Canvas, shadow: &Shadow) -> Result<Self> {
        let pad = shadow.pad;
        let (w, h) = (self.width() + 2 * pad, self.height() + 2 * pad);
        let mut paint = Paint::default();
        paint.set_color_filter(solid_color(shadow.color, shadow.opacity as f32));
        paint.set_image_filter(image_filters::blur((shadow.blur as f32, shadow.blur as f32), TileMode::Decal, None, None));
        Self::render(like, w, h, |c| {
            c.draw_image(&self.image, ((pad + shadow.dx) as f32, (pad + shadow.dy) as f32), Some(&paint));
            c.draw_image(&self.image, (pad as f32, pad as f32), None);
        })
    }
}

/// A colour filter that keeps the layer's shape (alpha scaled by `alpha`) and replaces its colour with `color`.
pub fn solid_color(color: [u8; 3], alpha: f32) -> ColorFilter {
    let [r, g, b] = color.map(|v| f32::from(v) / 255.0);
    #[rustfmt::skip]
    let m = [
        0.0, 0.0, 0.0, 0.0, r,
        0.0, 0.0, 0.0, 0.0, g,
        0.0, 0.0, 0.0, 0.0, b,
        0.0, 0.0, 0.0, alpha, 0.0,
    ];
    color_filters::matrix_row_major(&m, None)
}

/// Soft drop shadow beneath a layer.
#[derive(Debug, Clone, Copy)]
pub struct Shadow {
    pub dx: i32,
    pub dy: i32,
    pub blur: f64,
    pub color: [u8; 3],
    pub opacity: f64,
    pub pad: i32,
}

impl Shadow {
    pub const fn new(dy: i32, blur: f64, opacity: f64) -> Self {
        Self { dx: 0, dy, blur, color: [0, 0, 0], opacity, pad: 60 }
    }
}

/// Where and how a layer lands on a canvas: centre `(cx, cy)`, uniform scale and rotation (degrees, clockwise on screen)
/// about the layer's `anchor` (fractions of its size), and an opacity.
#[derive(Debug, Clone, Copy)]
pub struct Place {
    pub cx: f64,
    pub cy: f64,
    pub scale: f64,
    pub rot: f64,
    pub opacity: f64,
    pub anchor: (f64, f64),
}

impl Place {
    pub fn at(cx: f64, cy: f64) -> Self {
        Self { cx, cy, scale: 1.0, rot: 0.0, opacity: 1.0, anchor: (0.5, 0.5) }
    }
    pub fn scale(mut self, s: f64) -> Self {
        self.scale = s;
        self
    }
    pub fn rot(mut self, deg: f64) -> Self {
        self.rot = deg;
        self
    }
    pub fn opacity(mut self, o: f64) -> Self {
        self.opacity = o;
        self
    }
}

/// Draw `layer` through the matrix that maps its pixel indices to canvas pixel indices, sampling bilinearly with
/// transparent edges (OpenCV `warpAffine` / `warpPerspective` with a zero border).
pub fn draw_warped(canvas: &Canvas, layer: &Layer, m: &Mat3, opacity: f64) {
    let Some(shader) = layer.image.to_shader((TileMode::Decal, TileMode::Decal), linear(), None) else { return };
    let mut paint = Paint::default();
    paint.set_shader(shader).set_alpha_f(opacity as f32).set_anti_alias(false);
    canvas.save();
    canvas.concat(&m.to_skia());
    canvas.draw_rect(Rect::from_xywh(-1.0, -1.0, layer.width() as f32 + 2.0, layer.height() as f32 + 2.0), &paint);
    canvas.restore();
}

/// The reference `place`: a similarity transform of the layer composited over the canvas.
pub fn place(canvas: &Canvas, layer: &Layer, p: &Place) {
    if p.opacity <= 0.003 || p.scale <= 0.001 {
        return;
    }
    let (ax, ay) = (f64::from(layer.width()) * p.anchor.0, f64::from(layer.height()) * p.anchor.1);
    let m = Mat3::rotation_about(ax, ay, -p.rot, p.scale).then_translate(p.cx - ax, p.cy - ay);
    draw_warped(canvas, layer, &m, p.opacity);
}

/// Composite `layer` with its top-left pixel at integer (`x`, `y`): the reference `alpha_over`.
pub fn over(canvas: &Canvas, layer: &Layer, x: i32, y: i32, opacity: f64) {
    if opacity <= 0.0 {
        return;
    }
    let mut paint = Paint::default();
    paint.set_alpha_f(opacity as f32);
    canvas.draw_image(&layer.image, (x as f32, y as f32), Some(&paint));
}
