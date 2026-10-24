//! The studio film renderer: draws the scenes of a compiled, word-anchored motion-graphics film (`model`) with Skia on
//! Metal. It owns every film-specific element (kinetic type, charts, maps, globe, photo plates, panels), the Python
//! reference-parity helpers and the film transitions. Platform pieces come from `turbomotion-core`.

pub mod anim;
pub mod atlas;
pub mod elements;
pub mod background;
pub mod cache;
pub mod color;
pub mod cli;
pub mod font;
pub mod fx;
pub mod geo;
pub mod media;
pub mod model;
pub mod noise;
pub mod path;
pub mod post;
pub mod render;
pub mod pyfmt;
pub mod rng;
pub mod scene;
pub mod shaders;
pub mod stage;
pub mod transitions;
pub mod typeset;
