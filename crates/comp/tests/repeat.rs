//! A repeated layer renders `count` copies, each one step further on, through the whole pipeline.
use std::path::Path;

use serde_json::{json, Value};
use turbomotion_comp::{plan::Plan, render::Renderer};

const SIZE: usize = 400;

/// The lit pixel columns of the first frame, grouped into runs: `(first column, width)` of each run.
fn runs(film_layers: Value) -> Vec<(usize, usize)> {
    let film = json!({"width": SIZE, "height": SIZE, "fps": 30, "dur": 1, "bg": "#000000", "layers": film_layers});
    let plan = Plan::from_value(film, Path::new(".")).unwrap();
    let mut r = Renderer::new(&plan).unwrap();
    r.draw_at(0.0).unwrap();
    let mut px = vec![];
    r.pixels(&mut px).unwrap();
    let lit: Vec<bool> = (0..SIZE).map(|x| (0..SIZE).any(|y| px[(y * SIZE + x) * 4] > 128)).collect();
    let mut out: Vec<(usize, usize)> = vec![];
    for (x, on) in lit.iter().enumerate() {
        match (on, out.last_mut()) {
            (true, Some((start, w))) if *start + *w == x => *w += 1,
            (true, _) => out.push((x, 1)),
            _ => {}
        }
    }
    out
}

fn square(repeat: Value) -> Value {
    json!([{"type": "solid", "id": "sq", "color": "#ffffff", "size": [20, 20], "pos": [50, 200], "repeat": repeat}])
}

#[test]
fn offset_places_each_copy_one_step_further() {
    let r = runs(square(json!({"count": 3, "offset": [100, 0]})));
    assert_eq!(r.len(), 3, "{r:?}");
    assert_eq!(r[1].0 - r[0].0, 100, "{r:?}");
    assert_eq!(r[2].0 - r[1].0, 100, "{r:?}");
    assert!(r.iter().all(|(_, w)| *w == r[0].1), "same width: {r:?}");
}

#[test]
fn scale_shrinks_each_copy_about_the_origin() {
    // origin on the square's own centre: copies shrink in place, so the lit run is the original's
    let r = runs(square(json!({"count": 3, "scale": 0.5, "origin": [50, 200]})));
    assert_eq!(r.len(), 1, "{r:?}");
    // about a far origin, the copies close in on it
    let far = runs(square(json!({"count": 2, "scale": 0.5, "origin": [250, 200]})));
    assert_eq!(far.len(), 2, "{far:?}");
    assert!(far[1].0 > far[0].0 + far[0].1, "the copy is nearer the origin: {far:?}");
}

#[test]
fn a_layer_with_an_animated_position_repeats_without_rewriting_it() {
    let r = runs(json!([{"type": "solid", "id": "sq", "color": "#ffffff", "size": [20, 20], "pos": {"from": [50, 200], "to": [150, 200], "at": 0, "dur": 0.001}, "repeat": {"count": 2, "offset": [100, 0]}}]));
    assert_eq!(r.len(), 2, "{r:?}");
}
