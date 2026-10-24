//! The original shot renderer: a film is a list of shots (photograph, stat, card, painting, compare, map, diagram,
//! plan) drawn one after another on paper, cued to the narration. It owns every shot type and its look; the GPU
//! context, encoder and parallel scheduler come from `turbomotion-core`.

pub mod cli;

mod camera;
mod chrome;
mod css_filter;
mod ease;
mod figure;
mod film;
mod image;
mod layer;
mod motion;
mod overlay;
mod paper;
mod palette;
mod photo;
mod pins;
mod probe;
mod render;
mod review;
mod shots;
mod spot;
mod text;
