//! Decoded media shared by every layer a renderer draws.

use crate::media::{image::Images, video::Videos};

pub struct Resources {
    pub images: Images,
    pub videos: Videos,
}

impl Resources {
    /// `managed`: the film is colour-managed, so files are decoded with the colours they declare.
    pub fn new(managed: bool) -> Self {
        Resources { images: Images::new(managed), videos: Videos::new(managed) }
    }
}

