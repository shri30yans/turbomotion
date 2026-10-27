//! The platform under every turbomotion renderer. It owns the GPU context, the video encoder, the shader runner,
//! half-float layers, easing and the effects library, and knows nothing about films, shots or compositions: no
//! timelines, no fonts, no maps. Renderers depend on this crate; it never depends on them.

pub mod color;
pub mod dither;
pub mod ease;
pub mod effects;
pub mod encode;
pub mod geom;
pub mod gpu;
pub mod grade;
pub mod kernel;
pub mod layer;
pub mod parallel;
pub mod shader;
pub mod shaders;
pub mod suggest;
pub mod typeface;
