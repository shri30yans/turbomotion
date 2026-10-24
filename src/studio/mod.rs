//! The studio renderer: the native port of the Python motion-graphics engine. It draws the scenes of
//! a compiled film (`model`) with Skia on Metal, in half-float layers so the compositing matches the reference.

pub mod anim;
pub mod atlas;
pub mod elements;
pub mod background;
pub mod cache;
pub mod color;
pub mod cli;
pub mod ease;
pub mod font;
pub mod fx;
pub mod geo;
pub mod geom;
pub mod grade;
pub mod layer;
pub mod media;
pub mod model;
pub mod noise;
pub mod path;
pub mod post;
pub mod render;
pub mod pyfmt;
pub mod rng;
pub mod scene;
pub mod shader;
pub mod shaders;
pub mod stage;
pub mod transitions;
pub mod typeset;
