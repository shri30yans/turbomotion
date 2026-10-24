//! The services every element draws with: typesetting, compiled shaders and the shared backdrop. Created once per
//! renderer; elements borrow it and never reach for global state.

use anyhow::Result;

use super::{atlas::{Atlas}, background::{Backdrop}, cache::{Cache}, font::{Fonts}, media::{Media}, model::{Film, Rgb}, shaders::{Shaders}, typeset::{Typesetter}};
use turbomotion_core::{layer::{Layer}};

/// Cache key of an extruded text layer: text, font role, size, face colour, depth, tracking.
pub type ExtrudeKey = (String, String, i32, Rgb, i32, i64);

/// Cache key of a glass panel layer (see `elements::surface::Panel`).
pub type PanelKey = (i32, i32, i64, Rgb, i64, Rgb, Option<Rgb>);

pub struct Stage {
    pub type_: Typesetter,
    pub shaders: Shaders,
    pub backdrop: Backdrop,
    pub panels: Cache<PanelKey, Layer>,
    pub extruded: Cache<ExtrudeKey, Layer>,
    pub papers: Cache<(i32, i32), Layer>,
    pub atlas: Atlas,
    pub media: Media,
}

impl Stage {
    pub fn new(film: &Film) -> Result<Self> {
        Ok(Self { type_: Typesetter::new(Fonts::load(film)?), shaders: Shaders::compile()?, backdrop: Backdrop::new(), panels: Cache::new(128), extruded: Cache::new(256), papers: Cache::new(16), atlas: Atlas::new(&film.dir), media: Media::new(&film.dir) })
    }
}
