//! Effects that read their own past: trails and datamosh. A frame must be a pure function of time, so a frame reached by
//! seeking equals the same frame reached by playing from the start.

use super::*;
use crate::plan::Plan;
use serde_json::{json, Value};
use std::path::Path;

const W: usize = 200;
const H: usize = 32;
/// The square moves this many pixels each frame.
const STEP: f64 = 6.0;
const START_X: f64 = 10.0;
const SIZE: f64 = 4.0;

const FPS: u32 = 30;

/// A white square moving right at `STEP` px a frame, carrying `fx`; one key per frame so every frame has its exact place.
fn comp(fx: Value) -> Value {
    let keys: Vec<Value> = (0..=FPS).map(|f| json!([f64::from(f) / f64::from(FPS), [START_X + STEP * f64::from(f), 16.0], "none"])).collect();
    json!({"width": W, "height": H, "dur": 1, "fps": FPS, "layers": [
        {"type": "solid", "color": "#ffffff", "size": [SIZE, SIZE], "pos": {"keys": keys}, "fx": [fx]}
    ]})
}

/// The pixels after drawing each of `frames` in order on one renderer.
fn play(value: &Value, frames: &[u32]) -> Vec<u8> {
    let plan = Plan::from_value(value.clone(), Path::new(".")).expect("the plan");
    let mut renderer = Renderer::new(&plan).expect("a renderer");
    let mut pixels = vec![];
    for &f in frames {
        renderer.draw(f).expect("the frame");
    }
    renderer.pixels(&mut pixels).expect("the pixels");
    pixels
}

fn level(px: &[u8], x: usize) -> u8 {
    px[(16 * W + x) * 4]
}

fn centre(frame: u32) -> usize {
    (START_X + STEP * f64::from(frame)) as usize
}

#[test]
fn a_trail_keeps_a_decayed_copy_of_each_earlier_position() {
    let px = play(&comp(json!({"effect": "feedback_trail", "decay": 0.5})), &[10]);
    let now = centre(10);
    assert!(level(&px, now) > 250, "the square itself is at full strength");
    let one_back = i32::from(level(&px, now - STEP as usize));
    let two_back = i32::from(level(&px, now - 2 * STEP as usize));
    assert!((one_back - 128).abs() <= 3, "one frame back is half strength, not {one_back}");
    assert!((two_back - 64).abs() <= 3, "two frames back is a quarter, not {two_back}");
}

#[test]
fn seeking_to_a_frame_draws_what_playing_to_it_draws() {
    for effect in [json!({"effect": "feedback_trail", "decay": 0.8}), json!({"effect": "datamosh", "block": 16, "hold": 0.5, "smear": 8, "rate": 6})] {
        let name = effect["effect"].clone();
        let value = comp(effect);
        let played: Vec<u32> = (0..=20).collect();
        assert_eq!(play(&value, &played), play(&value, &[20]), "{name}: frame 20 after playing from 0 differs from frame 20 after a seek");
        assert_eq!(play(&value, &[20, 7, 20]), play(&value, &[20]), "{name}: jumping around changes the picture");
    }
}

#[test]
fn datamosh_with_every_block_held_shows_the_picture_from_depth_frames_ago() {
    const DEPTH: u32 = 12;
    let frozen = |hold: f64| play(&comp(json!({"effect": "datamosh", "hold": hold, "smear": 0})), &[DEPTH + 2]);
    let held = frozen(1.0);
    assert!(level(&held, centre(2)) > 250 && level(&held, centre(DEPTH + 2)) < 5, "every block keeps what it showed {DEPTH} frames ago");
    let fresh = frozen(0.0);
    assert!(level(&fresh, centre(DEPTH + 2)) > 250 && level(&fresh, centre(2)) < 5, "with nothing held the picture is the current one");
}

#[test]
fn history_inputs_are_refused_where_there_is_no_picture_to_remember() {
    let on_look = json!({"width": 8, "height": 8, "dur": 1, "layers": [], "look": [{"effect": "feedback_trail"}]});
    let e = format!("{:#}", Plan::from_value(on_look, Path::new(".")).err().expect("rejected"));
    assert!(e.contains("look") && e.contains("history"), "{e}");
    let on_adjustment = json!({"width": 8, "height": 8, "dur": 1, "layers": [{"type": "adjustment", "fx": [{"effect": "feedback_trail"}]}]});
    let e = format!("{:#}", Plan::from_value(on_adjustment, Path::new(".")).err().expect("rejected"));
    assert!(e.contains("adjustment") && e.contains("history"), "{e}");
}
