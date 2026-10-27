//! A GPU, the shaders and the means to read a picture back as floats: what the tests of anything that blurs measure with.

use skia_safe::{gpu::{self, Budgeted, SurfaceOrigin}, AlphaType, Color4f, ColorSpace, ColorType, ISize, ImageInfo, Paint, Rect, Surface};

use crate::{gpu::Gpu, layer::Layer, shaders::Shaders};

pub(crate) const SIGMAS: [f64; 7] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0];

/// A GPU, the shaders and a half-float render target of `side` x `side` in `space` (none: untagged).
pub(crate) struct Rig {
    gpu: Gpu,
    pub shaders: Shaders,
}

impl Rig {
    pub fn new() -> Self {
        Self { gpu: Gpu::new().expect("a GPU"), shaders: Shaders::compile().expect("the shaders compile") }
    }

    pub fn target(&mut self, w: i32, h: i32, space: Option<ColorSpace>) -> Surface {
        let info = ImageInfo::new(ISize::new(w, h), ColorType::RGBAF16, AlphaType::Premul, space);
        gpu::surfaces::render_target(&mut self.gpu.context, Budgeted::Yes, &info, None, SurfaceOrigin::TopLeft, None, false, false).expect("a render target")
    }
}

/// An opaque picture: `level(x, y)` for the red, green and blue of each pixel.
pub(crate) fn painted(like: &Surface, w: i32, h: i32, level: impl Fn(i32, i32) -> f32) -> Layer {
    let mut like = like.clone();
    Layer::render(like.canvas(), w, h, |c| {
        for y in 0..h {
            for x in 0..w {
                let v = level(x, y);
                if v != 0.0 {
                    let mut paint = Paint::new(Color4f::new(v, v, v, 1.0), None);
                    paint.set_anti_alias(false);
                    c.draw_rect(Rect::from_xywh(x as f32, y as f32, 1.0, 1.0), &paint);
                }
            }
        }
    })
    .expect("a picture")
}

/// Every pixel of `layer` as the four floats it is stored as (premultiplied), row by row.
pub(crate) fn pixels(like: &mut Surface, layer: &Layer) -> Vec<[f64; 4]> {
    like.canvas().clear(Color4f::new(0.0, 0.0, 0.0, 0.0));
    like.canvas().draw_image(layer.image(), (0, 0), None);
    let info = ImageInfo::new(like.image_info().dimensions(), ColorType::RGBAF32, AlphaType::Premul, like.image_info().color_space());
    let mut bytes = vec![0u8; info.compute_min_byte_size()];
    assert!(like.read_pixels(&info, &mut bytes, info.min_row_bytes(), (0, 0)));
    bytes.chunks_exact(16).map(|px| std::array::from_fn(|c| f64::from(f32::from_ne_bytes(px[c * 4..c * 4 + 4].try_into().expect("four bytes"))))).collect()
}

/// The red channel of `layer`, row by row.
pub(crate) fn red(like: &mut Surface, layer: &Layer) -> Vec<f64> {
    pixels(like, layer).into_iter().map(|p| p[0]).collect()
}
