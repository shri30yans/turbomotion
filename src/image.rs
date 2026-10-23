//! Decoding image files into GPU textures, and drawing them the way CSS does.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use skia_safe::{
    canvas::SrcRectConstraint,
    gpu::{self, Budgeted, DirectContext, Mipmapped},
    Canvas, Data, FilterMode, Image, MipmapMode, Paint, Rect, SamplingOptions,
};

/// Decode `path` and upload it to the GPU. Mipmaps make minified draws (`draw_cover`,
/// `draw_fit`) as smooth as a browser's.
pub fn load_texture(context: &mut DirectContext, path: &Path, mipmaps: Mipmapped) -> Result<Image> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let decoded = Image::from_encoded(Data::new_copy(&bytes)).with_context(|| format!("decoding {}", path.display()))?;
    gpu::images::texture_from_image(context, &decoded, mipmaps, Budgeted::Yes)
        .with_context(|| format!("uploading {}", path.display()))
}

fn minifying() -> SamplingOptions {
    SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear)
}

/// CSS `object-fit: cover; object-position: fx fy` of `image` into `dest`.
pub fn draw_cover(canvas: &Canvas, image: &Image, dest: Rect, focus: (f32, f32), paint: &Paint) {
    let (iw, ih) = (image.width() as f32, image.height() as f32);
    let scale = (dest.width() / iw).max(dest.height() / ih);
    let (sw, sh) = (dest.width() / scale, dest.height() / scale);
    let source = Rect::from_xywh((iw - sw) * focus.0, (ih - sh) * focus.1, sw, sh);
    canvas.draw_image_rect_with_sampling_options(image, Some((&source, SrcRectConstraint::Strict)), dest, minifying(), paint);
}

/// The whole image stretched to `dest` (an `<img>` given both dimensions).
pub fn draw_fit(canvas: &Canvas, image: &Image, dest: Rect, paint: &Paint) {
    canvas.draw_image_rect_with_sampling_options(image, None, dest, minifying(), paint);
}
