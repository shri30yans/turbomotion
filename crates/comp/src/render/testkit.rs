//! What the rendering tests share: draw a composition and read pixels back.

use std::path::Path;

use serde_json::Value;

use super::Renderer;
use crate::plan::Plan;

/// The BGRA pixels of the composition at `t` seconds (the frame starts black and opaque).
pub fn render_at(value: Value, dir: &Path, t: f64) -> Vec<u8> {
    let plan = Plan::from_value(value, dir).expect("the plan");
    let mut renderer = Renderer::new(&plan).expect("a renderer");
    renderer.draw_at(t).expect("the frame");
    let mut pixels = vec![];
    renderer.pixels(&mut pixels).expect("the pixels");
    pixels
}

/// The BGRA pixels of each frame number in `frames`, drawn by one renderer in the order given.
pub fn render_frames(value: Value, frames: &[u32]) -> Vec<Vec<u8>> {
    let plan = Plan::from_value(value, Path::new(".")).expect("the plan");
    let mut renderer = Renderer::new(&plan).expect("a renderer");
    frames
        .iter()
        .map(|&f| {
            renderer.draw(f).expect("the frame");
            let mut pixels = vec![];
            renderer.pixels(&mut pixels).expect("the pixels");
            pixels
        })
        .collect()
}

pub fn render(value: Value) -> Vec<u8> {
    render_at(value, Path::new("."), 0.0)
}

/// One pixel as `[b, g, r, a]` in a picture `width` wide.
pub fn bgra(px: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let i = (y * width + x) * 4;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}
