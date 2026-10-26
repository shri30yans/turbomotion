//! The blend modes on the GPU, end to end: a layer or an effect's result laid over a backdrop in every mode shows the
//! closed-form number, linear light and display numbers differ by the amount the sRGB curve says, a dissolve keeps the
//! share of pixels it is told to, and the stencils cut what is below the layer and not only under it.

use serde_json::{json, Value};
use turbomotion_core::effects::blend::{
    closed_form::{expected, Px},
    How, Mode, MODES,
};

use super::testkit::{bgra, render, render_at};

/// What a displayed level may differ from the closed form by, in levels of 255. The pictures are half floats (a step of
/// 2^-11 of the value, so 0.5 level near white) and the result is rounded to a level, which together allow one level
/// for every mode. The worst of all modes, pairs and opacities measures 0.88 level.
const LEVELS: f64 = 1.0;

const BACKDROP: [u8; 3] = [64, 128, 192];
const SOURCE: [u8; 3] = [224, 48, 128];

fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn unit(c: [u8; 3]) -> [f64; 3] {
    c.map(|v| f64::from(v) / 255.0)
}

/// The sRGB curve, from its definition (IEC 61966-2-1): level to light, and back.
fn to_light(v: f64) -> f64 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn to_display(light: f64) -> f64 {
    if light <= 0.003_130_8 { light * 12.92 } else { 1.055 * light.powf(1.0 / 2.4) - 0.055 }
}

/// A strip of `size`-wide cells of `backdrop`, each with a small square of `source` at `opacity` in one of `modes`.
fn strip(modes: &[&Mode], backdrop: [u8; 3], source: [u8; 3], opacity: f64, color: Value) -> (Vec<u8>, usize) {
    const CELL: usize = 8;
    let mut layers = vec![json!({"type": "solid", "color": hex(backdrop)})];
    for (k, m) in modes.iter().enumerate() {
        layers.push(json!({"type": "solid", "color": hex(source), "opacity": opacity, "blend": m.name, "size": [CELL, CELL], "pos": [k * CELL + CELL / 2, CELL / 2]}));
    }
    let width = CELL * modes.len();
    let mut film = json!({"width": width, "height": CELL, "fps": 10, "dur": 1, "bg": "#000000", "layers": layers});
    if !color.is_null() {
        film["color"] = color;
    }
    (render(film), width)
}

fn cell(px: &[u8], width: usize, k: usize) -> [u8; 4] {
    bgra(px, width, k * 8 + 4, 4)
}

/// The pixel of a frame wholly covered by `backdrop` and then by `source` at `opacity` in `blend`.
fn covered(blend: &str, backdrop: [u8; 3], source: [u8; 3], opacity: f64, color: Value) -> [u8; 4] {
    let mut film = json!({"width": 8, "height": 8, "fps": 10, "dur": 1, "bg": "#000000", "layers": [
        {"type": "solid", "color": hex(backdrop)}, {"type": "solid", "color": hex(source), "opacity": opacity, "blend": blend}]});
    if !color.is_null() {
        film["color"] = color;
    }
    bgra(&render(film), 8, 4, 4)
}

/// The modes whose result at a pixel is a function of the two colours there and which a small square can show whole.
fn local_modes() -> Vec<&'static Mode> {
    MODES.iter().filter(|m| !m.whole_frame).collect()
}

#[test]
fn every_mode_over_an_opaque_backdrop_shows_the_closed_form_level_within_one() {
    for (backdrop, source) in [(BACKDROP, SOURCE), (SOURCE, BACKDROP)] {
        for opacity in [1.0, 0.5] {
            let modes = local_modes();
            let (px, width) = strip(&modes, backdrop, source, opacity, Value::Null);
            for (k, m) in modes.iter().enumerate() {
                let want = expected(m.blend, Px { c: unit(backdrop), a: 1.0 }, Px { c: unit(source), a: opacity });
                let got = cell(&px, width, k);
                for (channel, bgr) in [(0, 2), (1, 1), (2, 0)] {
                    let off = (f64::from(got[bgr]) - 255.0 * want[channel]).abs();
                    assert!(off <= LEVELS, "{} (opacity {opacity}, backdrop {backdrop:?}, source {source:?}) channel {channel}: level {} against {:.2}", m.name, got[bgr], 255.0 * want[channel]);
                }
            }
        }
    }
}

#[test]
fn the_modes_that_reach_past_their_layer_show_the_closed_form_when_the_layer_covers_the_frame() {
    for m in MODES.iter().filter(|m| m.whole_frame && !matches!(m.how, How::Dissolve { .. })) {
        let got = covered(m.name, BACKDROP, SOURCE, 0.5, Value::Null);
        let want = expected(m.blend, Px { c: unit(BACKDROP), a: 1.0 }, Px { c: unit(SOURCE), a: 0.5 });
        // the frame is shown over black, so what a stencil leaves is its premultiplied colour
        for (channel, bgr) in [(0, 2), (1, 1), (2, 0)] {
            assert!((f64::from(got[bgr]) - 255.0 * want[channel]).abs() <= LEVELS, "{} channel {channel}: {} against {:.2}", m.name, got[bgr], 255.0 * want[channel]);
        }
    }
}

#[test]
fn multiply_screen_and_overlay_in_linear_light_differ_from_display_numbers_by_what_the_curve_says() {
    let light = |c: [u8; 3]| unit(c).map(to_light);
    for name in ["multiply", "screen", "overlay"] {
        let blend = MODES.iter().find(|m| m.name == name).expect("a mode").blend;
        let (display, in_light) = (covered(name, BACKDROP, SOURCE, 1.0, Value::Null), covered(name, BACKDROP, SOURCE, 1.0, json!({"linear_light": true})));
        // the same mode on light: take the colours to light, blend, show the result
        let want_light = expected(blend, Px { c: light(BACKDROP), a: 1.0 }, Px { c: light(SOURCE), a: 1.0 });
        let want_display = expected(blend, Px { c: unit(BACKDROP), a: 1.0 }, Px { c: unit(SOURCE), a: 1.0 });
        let mut apart = 0.0f64;
        for (channel, bgr) in [(0, 2), (1, 1), (2, 0)] {
            let (l, d) = (255.0 * to_display(want_light[channel]), 255.0 * want_display[channel]);
            assert!((f64::from(in_light[bgr]) - l).abs() <= LEVELS, "{name} in light, channel {channel}: {} against {l:.2}", in_light[bgr]);
            assert!((f64::from(display[bgr]) - d).abs() <= LEVELS, "{name} on display numbers, channel {channel}: {} against {d:.2}", display[bgr]);
            apart = apart.max((l - d).abs());
        }
        // the two predictions must be farther apart than the tolerance can blur, or agreeing with each would prove nothing
        assert!(apart > 2.0 * LEVELS, "{name}: the curve puts light and display numbers only {apart:.1} levels apart");
    }
}

#[test]
fn every_mode_in_linear_light_shows_the_closed_form_of_the_light_within_one_level() {
    let light = |c: [u8; 3]| unit(c).map(to_light);
    let mut apart_from_display = 0usize;
    for m in MODES.iter().filter(|m| !matches!(m.how, How::Dissolve { .. })) {
        let got = covered(m.name, BACKDROP, SOURCE, 0.5, json!({"linear_light": true}));
        let want = expected(m.blend, Px { c: light(BACKDROP), a: 1.0 }, Px { c: light(SOURCE), a: 0.5 });
        // the frame is shown over black in light and then encoded for the display, so what a stencil leaves is the curve of its premultiplied light
        let shown = |channel: usize| 255.0 * to_display(want[channel]);
        for (channel, bgr) in [(0, 2), (1, 1), (2, 0)] {
            assert!((f64::from(got[bgr]) - shown(channel)).abs() <= LEVELS, "{} in light, channel {channel}: {} against {:.2}", m.name, got[bgr], shown(channel));
        }
        let on_display = covered(m.name, BACKDROP, SOURCE, 0.5, Value::Null);
        apart_from_display += usize::from(got != on_display);
    }
    assert!(apart_from_display >= 20, "light and display numbers give the same picture in all but {apart_from_display} modes");
}

const SIDE: usize = 512;

/// The share of a `SIDE` square a white layer at `opacity` in `blend` keeps over black, at time `t`.
fn kept(blend: &str, opacity: f64, t: f64) -> (f64, Vec<u8>) {
    let film = json!({"width": SIDE, "height": SIDE, "fps": 10, "dur": 1, "bg": "#000000", "layers": [
        {"type": "solid", "color": "#ffffff", "opacity": opacity, "blend": blend}]});
    let px = render_at(film, std::path::Path::new("."), t);
    let share = px.chunks(4).filter(|p| p[2] > 127).count() as f64 / (SIDE * SIDE) as f64;
    (share, px)
}

#[test]
fn a_dissolve_keeps_the_share_of_a_large_flat_area_that_its_opacity_says() {
    // 262144 independent pixels: a standard deviation of at most 0.001, so 0.0045 is four and a half of them
    for opacity in [0.1, 0.5, 0.9] {
        for blend in ["dissolve", "dancing_dissolve"] {
            let (share, px) = kept(blend, opacity, 0.0);
            assert!((share - opacity).abs() < 0.0045, "{blend} at {opacity}: {share:.4}");
            assert!(px.chunks(4).all(|p| p[2] == 0 || p[2] == 255), "a pixel is kept whole or dropped whole");
        }
    }
}

#[test]
fn a_dissolve_repeats_exactly_and_only_the_dancing_one_changes_from_frame_to_frame() {
    let (_, first) = kept("dissolve", 0.5, 0.0);
    assert_eq!(kept("dissolve", 0.5, 0.0).1, first, "two renders of one frame are the same bytes");
    assert_eq!(kept("dissolve", 0.5, 0.3).1, first, "a plain dissolve is the same pattern on every frame");
    let (_, dancing) = kept("dancing_dissolve", 0.5, 0.0);
    assert_eq!(kept("dancing_dissolve", 0.5, 0.0).1, dancing);
    assert_eq!(dancing, first, "frame 0 of a dancing dissolve is the plain pattern");
    let (_, next) = kept("dancing_dissolve", 0.5, 0.1);
    let same = dancing.chunks(4).zip(next.chunks(4)).filter(|(a, b)| a[2] == b[2]).count() as f64 / (SIDE * SIDE) as f64;
    assert!((same - 0.5).abs() < 0.0045, "one frame on, the pattern agrees with itself only by chance: {same:.4}");
}

const W: usize = 64;

/// A backdrop with a square of `source` (half transparent when `opacity` says) laid on it in `blend`, the square in
/// the left half of the frame; the layer is `extra` keys beyond that.
fn square(blend: &str, source: &str, opacity: f64, extra: Value) -> Vec<u8> {
    let mut layer = json!({"type": "solid", "color": source, "opacity": opacity, "blend": blend, "size": [32, 32], "pos": [16, 16]});
    for (k, v) in extra.as_object().expect("keys").iter() {
        layer[k] = v.clone();
    }
    render(json!({"width": W, "height": 32, "fps": 10, "dur": 1, "bg": "#000000", "layers": [{"type": "solid", "color": hex(BACKDROP)}, layer]}))
}

const INSIDE: usize = 16;
const OUTSIDE: usize = 48;

fn at(px: &[u8], x: usize) -> [u8; 4] {
    bgra(px, W, x, 16)
}

fn near(got: u8, want: f64, what: &str) {
    assert!((f64::from(got) - want).abs() <= LEVELS, "{what}: {got} against {want:.2}");
}

/// The red, green and blue of a pixel, which the frame shows over black: what a stencil or silhouette leaves is its
/// premultiplied colour.
fn rgb(p: [u8; 4]) -> [u8; 3] {
    [p[2], p[1], p[0]]
}

/// `BACKDROP` scaled by `by`, as levels.
fn scaled(by: f64) -> [f64; 3] {
    BACKDROP.map(|v| by * f64::from(v))
}

fn assert_rgb(got: [u8; 4], want: [f64; 3], what: &str) {
    for (c, want) in want.iter().enumerate() {
        near(rgb(got)[c], *want, &format!("{what}, channel {c}"));
    }
}

#[test]
fn the_alpha_stencil_keeps_the_backdrop_by_the_layers_alpha_and_clears_it_where_the_layer_is_not() {
    let px = square("stencil_alpha", "#ffffff", 0.5, json!({}));
    assert_rgb(at(&px, INSIDE), scaled(0.5), "under a half transparent layer half the backdrop is kept");
    assert_rgb(at(&px, OUTSIDE), scaled(0.0), "where there is no layer there is no alpha to keep what is below by");
    assert_rgb(at(&square("stencil_alpha", "#ffffff", 1.0, json!({})), INSIDE), scaled(1.0), "an opaque layer keeps it whole");
}

#[test]
fn the_alpha_silhouette_cuts_the_backdrop_by_the_layers_alpha_and_leaves_it_where_the_layer_is_not() {
    let px = square("silhouette_alpha", "#ffffff", 0.5, json!({}));
    assert_rgb(at(&px, INSIDE), scaled(0.5), "under a half transparent layer half the backdrop is cut");
    assert_rgb(at(&px, OUTSIDE), scaled(1.0), "outside the layer nothing is cut");
    assert_rgb(at(&square("silhouette_alpha", "#ffffff", 1.0, json!({})), INSIDE), scaled(0.0), "an opaque layer cuts a hole");
}

#[test]
fn the_luma_stencil_and_silhouette_weigh_the_backdrop_by_the_brightness_of_the_layer() {
    // the layer's brightness is the Rec. 709 weights of its numbers, which sum to one: #808080 is 128/255 whatever they are
    let luma = 128.0 / 255.0;
    let stencil = square("stencil_luma", "#808080", 1.0, json!({}));
    assert_rgb(at(&stencil, INSIDE), scaled(luma), "under a mid grey stencil");
    assert_rgb(at(&stencil, OUTSIDE), scaled(0.0), "no layer is no brightness: everything is cut");
    let silhouette = square("silhouette_luma", "#808080", 1.0, json!({}));
    assert_rgb(at(&silhouette, INSIDE), scaled(1.0 - luma), "under a mid grey silhouette");
    assert_rgb(at(&silhouette, OUTSIDE), scaled(1.0), "no layer is no brightness: nothing is cut");
}

#[test]
fn a_layer_with_effects_in_layer_space_is_placed_before_it_stencils_so_the_stencil_reaches_the_frame() {
    let film = |blend: &str| {
        render(json!({"width": W, "height": 32, "fps": 10, "dur": 1, "bg": "#000000", "effect_space": "layer", "layers": [
            {"type": "solid", "color": hex(BACKDROP)},
            {"type": "solid", "color": "#ffffff", "blend": blend, "size": [24, 24], "pos": [32, 16], "rotation": 30, "fx": [{"effect": "invert", "amount": 0}]}]}))
    };
    let px = film("stencil_alpha");
    assert_rgb(at(&px, 32), scaled(1.0), "the rotated square keeps what is under it");
    assert_rgb(at(&px, 2), scaled(0.0), "and the stencil clears the frame around it, as it does for a layer in screen space");
    assert_eq!(film("stencil_alpha"), px, "twice, byte for byte");
}

#[test]
fn an_effects_composite_blend_takes_every_mode_as_a_layer_does() {
    let film = |blend: &str| {
        json!({"width": W, "height": 32, "dur": 1, "layers": [{"type": "precomp", "comp": "src", "fx": [
            {"effect": "invert", "amount": 1, "composite": {"blend": blend}}]}],
            "comps": {"src": {"width": W, "height": 32, "dur": 1, "layers": [{"type": "solid", "color": "#202020", "size": [32, 32], "pos": [16, 16]}, {"type": "solid", "color": "#e0e0e0", "size": [32, 32], "pos": [48, 16]}]}}})
    };
    // the input is level 32 | 224 and the inverted result 223 | 31: input minus result is 0 | 193, and linear burn adds them less one
    let subtract = render(film("subtract"));
    near(at(&subtract, INSIDE)[2], 0.0, "left, subtract");
    near(at(&subtract, OUTSIDE)[2], 193.0, "right, subtract");
    let burn = render(film("linear_burn"));
    near(at(&burn, INSIDE)[2], 0.0, "left, linear burn");
    near(at(&burn, OUTSIDE)[2], 0.0, "right, linear burn");
    let darker = render(film("darker_color"));
    near(at(&darker, INSIDE)[2], 32.0, "left, darker colour keeps the darker of the two");
    near(at(&darker, OUTSIDE)[2], 31.0, "right, darker colour keeps the darker of the two");
}

#[test]
fn a_film_with_every_mode_renders_the_same_bytes_twice() {
    let all: Vec<&Mode> = MODES.iter().collect();
    let first = strip(&all, BACKDROP, SOURCE, 0.5, Value::Null).0;
    assert_eq!(strip(&all, BACKDROP, SOURCE, 0.5, Value::Null).0, first);
    assert_eq!(strip(&all, BACKDROP, SOURCE, 0.5, json!({"linear_light": true})).0, strip(&all, BACKDROP, SOURCE, 0.5, json!({"linear_light": true})).0);
}

#[test]
fn a_misspelt_mode_on_a_layer_is_refused_with_the_nearest_mode_and_the_list() {
    let film = json!({"width": 8, "height": 8, "dur": 1, "layers": [{"type": "solid", "color": "#ffffff", "blend": "linear_lite"}]});
    let Err(e) = crate::plan::Plan::from_value(film, std::path::Path::new(".")) else { panic!("a misspelt mode must be refused") };
    let e = format!("{e:#}");
    assert!(e.contains("linear_lite") && e.contains("Did you mean \"linear_light\"") && e.contains("stencil_alpha"), "{e}");
}
