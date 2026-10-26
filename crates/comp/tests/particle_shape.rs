//! A particle can be any outline: a path sprite is fitted to the particle's size and drawn where the particle is.
use std::path::Path;

use serde_json::{json, Value};
use turbomotion_comp::{plan::Plan, render::Renderer};

const SIZE: usize = 200;

fn burst(shape: Value) -> Value {
    json!({"width": SIZE, "height": SIZE, "fps": 30, "dur": 1, "bg": "#000000", "layers": [
        {"type": "particles", "pos": [100, 100], "rate": 0, "burst": [[0.0, 1]], "life": [2, 2], "speed": [0, 0], "size": [80, 80], "spin": [0, 0], "colors": [[0, "#ffffff"]], "shape": shape}]})
}

/// Lit pixel count and centroid of the first frame (a particle starts at a random angle, so only these are turn-proof).
fn lit_area_and_centre(film: Value) -> (usize, (f64, f64)) {
    let plan = Plan::from_value(film, Path::new(".")).unwrap();
    let mut r = Renderer::new(&plan).unwrap();
    r.draw_at(0.0).unwrap();
    let mut px = vec![];
    r.pixels(&mut px).unwrap();
    let lit: Vec<(f64, f64)> = px.chunks_exact(4).enumerate().filter(|(_, p)| p[0] > 128).map(|(i, _)| ((i % SIZE) as f64, (i / SIZE) as f64)).collect();
    let n = lit.len() as f64;
    (lit.len(), (lit.iter().map(|p| p.0).sum::<f64>() / n, lit.iter().map(|p| p.1).sum::<f64>() / n))
}

#[test]
fn a_path_sprite_is_fitted_to_the_particle_size_keeping_its_proportions() {
    // a bar twice as wide as tall: the long side takes the particle's 80 px, so it covers 80 x 40
    let (area, _) = lit_area_and_centre(burst(json!({"path": {"rect": [0, 0, 20, 10]}})));
    assert!((3000..=3400).contains(&area), "area {area}");
}

#[test]
fn the_sprite_is_centred_on_the_particle_whatever_its_own_coordinates() {
    let (area, (cx, cy)) = lit_area_and_centre(burst(json!({"path": {"rect": [500, 500, 20, 10]}})));
    assert!((3000..=3400).contains(&area), "area {area}");
    assert!((cx - 100.0).abs() < 1.5 && (cy - 100.0).abs() < 1.5, "centre ({cx}, {cy})");
}

#[test]
fn a_misspelt_shape_or_unreadable_path_is_a_plan_error() {
    let e = format!("{:#}", Plan::from_value(burst(json!("strar")), Path::new(".")).err().expect("misspelt"));
    assert!(e.contains("Did you mean \"star\"?"), "{e}");
    let e = format!("{:#}", Plan::from_value(burst(json!({"path": {"d": "not a path"}})), Path::new(".")).err().expect("unreadable"));
    assert!(e.contains("particle shape path") && e.contains("could not read"), "{e}");
}
