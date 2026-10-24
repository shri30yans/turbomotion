//! The paper backdrop behind typography, diagrams and framed artwork

use std::path::Path;

use anyhow::{Context, Result};
use skia_safe::{
    gradient_shader::{self, Flags, Interpolation},
    gpu::{DirectContext, Mipmapped},
    BlendMode, Canvas, Color, Color4f, Image, Matrix, Paint, Point, Rect, SamplingOptions, TileMode,
};

use crate::{
    image::load_texture,
    palette::{NIGHT, PAPER},
};

/// Which composition draws the paper; they differ in grain strength and gradient.
#[derive(Debug, Clone, Copy)]
pub enum Paper {
    /// Behind typography cards, stats and framed artwork.
    Sheet { dark: bool },
    /// Behind diagrams and charts.
    Board { dark: bool },
}

/// Tunables of one paper look.
struct Look {
    ground: Color,
    grain_opacity: f32,
    grain_blend: BlendMode,
    /// Initial grain zoom and its growth per frame (a slow push-in).
    zoom: (f32, f32),
    /// Glow centre in CSS percent, then its inner and outer colours.
    glow: ((f32, f32), Color, Color),
}

impl Paper {
    fn look(self) -> Look {
        match self {
            Self::Sheet { dark: true } => Look {
                ground: NIGHT,
                grain_opacity: 0.08,
                grain_blend: BlendMode::Screen,
                zoom: (1.04, 0.00004),
                glow: ((50.0, 40.0), Color::from_argb(15, 255, 240, 210), Color::from_argb(128, 0, 0, 0)),
            },
            Self::Sheet { dark: false } => Look {
                ground: PAPER,
                grain_opacity: 0.55,
                grain_blend: BlendMode::Multiply,
                zoom: (1.04, 0.00004),
                glow: ((45.0, 40.0), Color::from_argb(128, 255, 250, 240), Color::from_argb(46, 120, 90, 50)),
            },
            Self::Board { dark: true } => Look {
                ground: Color::from_rgb(0x19, 0x13, 0x10),
                grain_opacity: 0.06,
                grain_blend: BlendMode::Screen,
                zoom: (1.03, 0.00003),
                glow: ((50.0, 45.0), Color::from_argb(20, 224, 169, 59), Color::from_argb(140, 0, 0, 0)),
            },
            Self::Board { dark: false } => Look {
                ground: PAPER,
                grain_opacity: 0.5,
                grain_blend: BlendMode::Multiply,
                zoom: (1.03, 0.00003),
                glow: ((50.0, 42.0), Color::from_argb(115, 255, 250, 240), Color::from_argb(41, 120, 90, 50)),
            },
        }
    }
}

/// A paper texture, ready to draw behind a shot.
pub struct Backdrop {
    look: Look,
    grain: Image,
    size: (f32, f32),
}

impl Backdrop {
    /// `assets` is the asset directory (`paper.jpg`); `size` the frame size.
    pub fn prepare(context: &mut DirectContext, assets: &Path, paper: Paper, size: (f32, f32)) -> Result<Self> {
        Ok(Self { look: paper.look(), grain: load_texture(context, &assets.join("paper.jpg"), Mipmapped::No)?, size })
    }

    /// Draw at `frame` frames into the shot.
    pub fn draw(&self, canvas: &Canvas, frame: f32) -> Result<()> {
        let (w, h) = self.size;
        let frame_rect = Rect::from_wh(w, h);
        let mut ground = Paint::default();
        ground.set_color(self.look.ground);
        canvas.draw_rect(frame_rect, &ground);

        // The grain image is 1920×1080: stretched to the frame and pushed in around its centre.
        let zoom = self.look.zoom.0 + frame * self.look.zoom.1;
        canvas.save();
        canvas.translate((w / 2.0, h / 2.0));
        canvas.scale((zoom, zoom));
        canvas.translate((-w / 2.0, -h / 2.0));
        let mut grain = Paint::default();
        grain.set_alpha_f(self.look.grain_opacity).set_blend_mode(self.look.grain_blend);
        canvas.draw_image_rect_with_sampling_options(&self.grain, None, frame_rect, SamplingOptions::default(), &grain);
        canvas.restore();

        let mut glow = Paint::default();
        glow.set_shader(self.glow_shader().context("paper glow shader")?);
        canvas.draw_rect(frame_rect, &glow);
        Ok(())
    }

    /// `radial-gradient(ellipse at x% y%, inner, outer)`: the ellipse reaches the farthest corner.
    fn glow_shader(&self) -> Option<skia_safe::Shader> {
        let (w, h) = self.size;
        let ((px, py), inner, outer) = self.look.glow;
        let (cx, cy) = (w * px / 100.0, h * py / 100.0);
        let radii = (std::f32::consts::SQRT_2 * cx.max(w - cx), std::f32::consts::SQRT_2 * cy.max(h - cy));
        let mut to_frame = Matrix::translate((cx, cy));
        to_frame.pre_scale(radii, None);
        // CSS interpolates colour stops in premultiplied space.
        let interpolation = Interpolation::from(Flags::INTERPOLATE_COLORS_IN_PREMUL);
        gradient_shader::radial_with_interpolation(
            (Point::new(0.0, 0.0), 1.0),
            (&[Color4f::from(inner), Color4f::from(outer)], None),
            None,
            TileMode::Clamp,
            interpolation,
            &to_frame,
        )
    }
}
