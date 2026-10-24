//! Decoded media shared by every layer a renderer draws.

use crate::media::{image::Images, video::Videos};

#[derive(Default)]
pub struct Resources {
    pub images: Images,
    pub videos: Videos,
}
