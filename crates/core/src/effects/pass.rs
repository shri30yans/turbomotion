//! Running a shader over a layer: the one way every effect turns a layer into a new layer.

use anyhow::{anyhow, Result};
use skia_safe::{Canvas, Matrix, Paint, Rect, Shader, TileMode};

use crate::{
    layer::{linear, Layer},
    shader::{Program, Uniforms},
    shaders::Shaders,
};

/// Where the picture an effect chain works on sits in the layer's own pixel space.
///
/// In screen space the picture *is* the layer: its top-left pixel is layer pixel `(0, 0)` and the content fills it. In
/// layer space the picture is a window onto the layer that can reach past the content (room for a blur to spread): its
/// top-left pixel is layer pixel `origin` (negative when it grows up or left) and the content is the box inside it
/// that the layer's own pixels fill. Shaders see **layer pixels** (`xy` is `buffer pixel + origin`), and what an effect
/// calls "the frame" (a centre given as a fraction, the corners of a vignette) is the content box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Space {
    /// The layer pixel at the picture's top-left corner.
    pub origin: [f64; 2],
    /// `[left, top, right, bottom]` of the layer's content in layer pixels.
    pub content: [f64; 4],
}

impl Space {
    /// A picture that is the whole layer.
    pub fn whole(layer: &Layer) -> Self {
        Self { origin: [0.0, 0.0], content: [0.0, 0.0, f64::from(layer.width()), f64::from(layer.height())] }
    }

    pub fn width(&self) -> f64 {
        self.content[2] - self.content[0]
    }

    pub fn height(&self) -> f64 {
        self.content[3] - self.content[1]
    }

    /// The layer pixel at fractions `(x, y)` of the content box.
    pub fn at(&self, x: f64, y: f64) -> (f64, f64) {
        (self.content[0] + x * self.width(), self.content[1] + y * self.height())
    }

    /// The content box's centre.
    pub fn centre(&self) -> (f64, f64) {
        self.at(0.5, 0.5)
    }

    /// Whether picture pixels are layer pixels (no shift to apply).
    fn is_whole(&self) -> bool {
        self.origin == [0.0, 0.0]
    }

    /// Moves a layer-pixel coordinate system onto the picture's: the matrix a shader's local space needs so that its
    /// `xy` is a layer pixel while the canvas draws picture pixels.
    pub fn to_layer(&self) -> Matrix {
        Matrix::translate((-self.origin[0] as f32, -self.origin[1] as f32))
    }
}

/// What an effect may use while it runs: the shared services, the canvas that provides the GPU context, the time, and
/// where its picture sits in layer space (none: the picture is the whole layer).
pub struct Ctx<'a> {
    pub shaders: &'a Shaders,
    pub like: &'a Canvas,
    pub t: f64,
    pub space: Option<Space>,
}

impl<'a> Ctx<'a> {
    /// A context for a picture that is the whole layer.
    pub fn new(shaders: &'a Shaders, like: &'a Canvas, t: f64) -> Self {
        Self { shaders, like, t, space: None }
    }

    /// Where `layer` sits in layer space.
    pub fn space_of(&self, layer: &Layer) -> Space {
        self.space.unwrap_or_else(|| Space::whole(layer))
    }
}

/// `layer` as a bilinear shader that is transparent beyond its edges, with its pixels at their layer position.
pub fn source(ctx: &Ctx, layer: &Layer) -> Result<Shader> {
    let space = ctx.space_of(layer);
    let local = (!space.is_whole()).then(|| Matrix::translate((space.origin[0] as f32, space.origin[1] as f32)));
    layer.image().to_shader((TileMode::Decal, TileMode::Decal), linear(), local.as_ref()).ok_or_else(|| anyhow!("could not sample a layer"))
}

/// Run `program` over `layer`: the layer is the first child shader, `more` follow, and the result has the layer's size.
/// The shader's `p` is a layer pixel.
pub fn run(ctx: &Ctx, layer: &Layer, program: &Program, uniforms: Uniforms, more: &[Shader]) -> Result<Layer> {
    let mut children = vec![source(ctx, layer)?];
    children.extend_from_slice(more);
    let mut shader = program.instance(uniforms, &children)?;
    let space = ctx.space_of(layer);
    if !space.is_whole() {
        shader = shader.with_local_matrix(&Matrix::translate((-space.origin[0] as f32, -space.origin[1] as f32)));
    }
    let mut paint = Paint::default();
    paint.set_shader(shader);
    Layer::render(ctx.like, layer.width(), layer.height(), |c| {
        c.draw_rect(Rect::from_wh(layer.width() as f32, layer.height() as f32), &paint);
    })
}
