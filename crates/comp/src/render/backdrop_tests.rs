//! Frosted-glass backdrop sampling.

use serde_json::json;

use super::testkit::{bgra, render};

const W: usize = 64;
const H: usize = 32;
const ROW: usize = 16;

#[test]
fn backdrop_blurs_what_is_behind_the_layer() {
    let px = render(json!({
        "width": W,
        "height": H,
        "dur": 1,
        "layers": [
            {"type": "solid", "color": "#ff0000", "size": [W, H], "pos": [W / 2, H / 2]},
            {"type": "shape", "pos": [W / 2, H / 2], "items": [{"rect": [-24, -12, 48, 24], "radius": 6, "fill": {"color": "#ffffff", "opacity": 0.15}}],
                "fx": [{"effect": "backdrop", "blur": 6}]},
        ]
    }));
    let c = bgra(&px, W, W / 2, ROW);
    assert!(c[2] > 200 && c[1] < 80, "the centre should still read red through frosted glass, got {:?}", c);
}
