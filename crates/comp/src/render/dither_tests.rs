//! `color.dither` on rendered frames: left out or `"none"` is the old rounding byte for byte, a dither moves no pixel
//! more than one level, repeats exactly (the pattern is static, so frames agree), breaks a dark gradient's bands up, and
//! works the same in a colour-managed film. The ramp's banding numbers are in `core`'s `dither::tests`.

use serde_json::{json, Value};

use super::testkit::{render, render_frames};

const W: usize = 1024;
const H: usize = 64;
const SLAB: usize = 16;

/// A grey ramp from level 5 to level 15 across a `W` x `H` frame, in a film with `color`.
fn ramp(color: Value) -> Value {
    let mut film = json!({"width": W, "height": H, "fps": 10, "dur": 1, "bg": "#000000", "layers": [
        {"type": "shape", "items": [{"rect": [0, 0, W, H], "fill": {"gradient": {"type": "linear", "from": [0, 0], "to": [W, 0], "stops": [[0, "#050505"], [1, "#0f0f0f"]]}}}]}
    ]});
    if !color.is_null() {
        film["color"] = color;
    }
    film
}

fn largest_difference(a: &[u8], b: &[u8]) -> u8 {
    a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)).max().unwrap_or(0)
}

/// How many different means the 16-column slabs of the red channel have.
fn slab_means(px: &[u8]) -> usize {
    let mut means: Vec<u64> = (0..W / SLAB).map(|s| (0..H).flat_map(|y| (s * SLAB..(s + 1) * SLAB).map(move |x| (y, x))).map(|(y, x)| u64::from(px[(y * W + x) * 4 + 2])).sum()).collect();
    means.sort_unstable();
    means.dedup();
    means.len()
}

#[test]
fn leaving_dither_out_or_saying_none_is_the_old_rounding_byte_for_byte() {
    let plain = render(ramp(Value::Null));
    assert_eq!(plain, render(ramp(json!({"dither": "none"}))));
    assert_eq!(plain, render(ramp(json!({"linear_light": false}))));
}

#[test]
fn a_dither_moves_no_pixel_more_than_a_level_and_breaks_the_bands_up() {
    let plain = render(ramp(Value::Null));
    assert!(slab_means(&plain) <= 24, "the plain ramp has {} slab means: its bands", slab_means(&plain));
    for kind in ["ordered", "blue_noise"] {
        let dithered = render(ramp(json!({"dither": kind})));
        assert_eq!(largest_difference(&plain, &dithered), 1, "{kind}: one level at most, and it does something");
        assert!(slab_means(&dithered) >= 60, "{kind}: {} slab means of a possible {}", slab_means(&dithered), W / SLAB);
    }
}

#[test]
fn the_same_film_dithers_to_the_same_bytes_in_every_frame_and_every_render() {
    let film = ramp(json!({"dither": "blue_noise"}));
    let frames = render_frames(film.clone(), &[0, 1, 5]);
    assert_eq!(frames[0], frames[1]);
    assert_eq!(frames[0], frames[2]);
    assert_eq!(frames[0], render(film));
}

#[test]
fn a_colour_managed_film_dithers_after_the_output_conversion() {
    let managed = json!({"linear_light": true, "space": "display_p3"});
    let plain = render(ramp(managed.clone()));
    let mut with = managed;
    with["dither"] = json!("blue_noise");
    let dithered = render(ramp(with));
    assert_eq!(largest_difference(&plain, &dithered), 1, "the numbers the file holds move by at most a level");
    assert!(slab_means(&dithered) > slab_means(&plain), "{} slab means against {}", slab_means(&dithered), slab_means(&plain));
}
