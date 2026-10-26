//! Decoding still images for drawing: EXIF orientation applied, mipmapped so slow zooms and downscales do not shimmer.

use std::{cell::RefCell, collections::HashMap, fs, path::{Path, PathBuf}, rc::Rc};

use anyhow::{anyhow, Context, Result};
use skia_safe::{codec::Codec, surfaces, AlphaType, ColorType, Data, Image, ImageInfo, ISize, Paint};

use super::color::still_space;

/// Decoded stills. In a colour-managed film each is tagged with its own profile, so drawing converts it into the
/// working space; otherwise its numbers are drawn as they are.
#[derive(Default)]
pub struct Images {
    managed: bool,
    cache: RefCell<HashMap<PathBuf, Rc<Image>>>,
}

impl Images {
    pub fn new(managed: bool) -> Self {
        Images { managed, cache: Default::default() }
    }

    pub fn get(&self, path: &Path) -> Result<Rc<Image>> {
        if let Some(i) = self.cache.borrow().get(path) {
            return Ok(i.clone());
        }
        let img = Rc::new(decode(path, self.managed)?);
        let mut c = self.cache.borrow_mut();
        if c.len() > 64 {
            c.clear();
        }
        c.insert(path.to_path_buf(), img.clone());
        Ok(img)
    }
}

fn decode(path: &Path, managed: bool) -> Result<Image> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let mut codec = Codec::from_data(Data::new_copy(&bytes)).ok_or_else(|| anyhow!("{} is not a decodable image", path.display()))?;
    let origin = codec.origin();
    let dims = codec.dimensions();
    // Decode the stored pixels ourselves; Skia's generator path would apply the orientation by itself and then disagree
    // about the size. Orientation is applied below, once.
    let space = managed.then(|| still_space(&codec));
    let info = ImageInfo::new(dims, ColorType::RGBA8888, AlphaType::Premul, space.clone());
    let mut pixels = vec![0u8; info.compute_min_byte_size()];
    let result = codec.get_pixels_with_options(&info, &mut pixels, info.min_row_bytes(), None);
    anyhow::ensure!(matches!(result, skia_safe::codec::Result::Success | skia_safe::codec::Result::IncompleteInput), "decoding {}: {result:?}", path.display());
    let raw = skia_safe::images::raster_from_data(&info, Data::new_copy(&pixels), info.min_row_bytes()).ok_or_else(|| anyhow!("no memory for {}", path.display()))?;
    let img = if origin == skia_safe::EncodedOrigin::TopLeft {
        raw
    } else {
        let (w, h) = if origin.swaps_width_height() { (dims.height, dims.width) } else { (dims.width, dims.height) };
        let info = ImageInfo::new(ISize::new(w, h), ColorType::RGBA8888, AlphaType::Premul, space);
        let mut s = surfaces::raster(&info, None, None).ok_or_else(|| anyhow!("no memory for {w}x{h}"))?;
        s.canvas().concat(&origin.to_matrix(dims));
        s.canvas().draw_image(&raw, (0, 0), Some(&Paint::default()));
        s.image_snapshot()
    };
    img.with_default_mipmaps().ok_or_else(|| anyhow!("could not build mipmaps for {}", path.display()))
}
