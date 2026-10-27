//! `add_grain` (BUILD_PLAN 5.3), measured: the standard deviation against the formula of `parity/specs/add_grain.md`, the
//! mean, the autocorrelation length, the channel correlation, the tone response, determinism, and the same statistics in
//! both effect spaces and in a linear-light film. Every expectation is computed from the spec's definition.
//!
//! The test card is a flat grey: the grain is the only thing that varies, so the measured spread *is* the grain's (plus
//! the 1/12 level squared of rounding to 8 bits, which the expectations include).

use serde_json::{json, Value};

use super::{
    stats::Plane,
    testkit::{render, render_frames},
};

const W: usize = 256;
const H: usize = 256;
/// Standard deviation of the grain at intensity 1, in levels of 255: 4% of full scale (`SIGMA_PER_INTENSITY`).
const LEVELS_PER_INTENSITY: f64 = 0.04 * 255.0;
/// What rounding to 8 bits adds to the variance of anything that is not itself on the grid.
const ROUNDING_VARIANCE: f64 = 1.0 / 12.0;

fn grain(extra: Value) -> Value {
    let mut fx = json!({"effect": "add_grain", "saturation": 0});
    fx.as_object_mut().expect("an object").extend(extra.as_object().expect("an object").clone());
    fx
}

/// A frame of `w` x `h` with a grey `level` card carrying `fx`, in a film with `extra` keys.
fn film_of(w: usize, h: usize, level: u8, fx: Value, extra: Value) -> Value {
    let mut film = json!({"width": w, "height": h, "fps": 24, "dur": 2, "bg": "#000000", "layers": [{"type": "solid", "color": format!("#{level:02x}{level:02x}{level:02x}"), "fx": [fx]}]});
    film.as_object_mut().expect("an object").extend(extra.as_object().expect("an object").clone());
    film
}

fn card(level: u8, fx: Value) -> Value {
    film_of(W, H, level, fx, json!({}))
}

fn plane(px: &[u8], w: usize, h: usize, channel: usize) -> Plane {
    Plane::of(px, w, h, channel)
}

/// The expected standard deviation of a grey's grain, in levels, with the rounding included.
fn expected_std(intensity: f64, tone: f64) -> f64 {
    ((intensity * tone * LEVELS_PER_INTENSITY).powi(2) + ROUNDING_VARIANCE).sqrt()
}

/// The spec's tone weight at brightness `l` for the three gains.
fn tone(l: f64, shadows: f64, midtones: f64, highlights: f64) -> f64 {
    let smooth = |a: f64, b: f64| {
        let t = ((l - a) / (b - a)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let (dark, light) = (1.0 - smooth(0.0, 0.5), smooth(0.5, 1.0));
    shadows * dark + midtones * (1.0 - dark - light) + highlights * light
}

fn assert_close(what: &str, got: f64, want: f64, tolerance: f64) {
    assert!((got - want).abs() <= tolerance, "{what}: {got:.4} against {want:.4} (tolerance {tolerance})");
}

#[test]
fn the_standard_deviation_follows_intensity_as_the_spec_says() {
    for intensity in [0.5, 1.0, 2.0] {
        let px = render(card(128, grain(json!({"intensity": intensity}))));
        let std = plane(&px, W, H, 2).std();
        let want = expected_std(intensity, 1.0);
        assert_close(&format!("intensity {intensity}"), std / want, 1.0, 0.03);
    }
}

#[test]
fn the_mean_stays_within_half_a_level_of_the_grey_it_is_added_to() {
    let frames = render_frames(card(128, grain(json!({"intensity": 1}))), &[0, 1, 2, 3, 4, 5, 6, 7]);
    let mean = frames.iter().map(|px| plane(px, W, H, 2).mean()).sum::<f64>() / frames.len() as f64;
    assert_close("mean over 8 frames", mean, 128.0, 0.5);
    for px in &frames {
        assert_close("mean of one frame", plane(px, W, H, 2).mean(), 128.0, 0.5);
    }
}

/// The autocorrelation of grain of correlation length `(lx, ly)` at lag `(dx, dy)` pixels, from the kernel's closed form.
fn closed_form(lag: f64, length: f64) -> f64 {
    (-(lag * lag) / (length * length)).exp()
}

#[test]
fn size_sets_the_length_over_which_the_grain_agrees_with_itself() {
    for size in [1.5, 3.0, 6.0] {
        let px = render(film_of(512, 512, 128, grain(json!({"intensity": 2, "size": size})), json!({})));
        let p = plane(&px, 512, 512, 2);
        for lag in 1..=4usize {
            let want = closed_form(lag as f64, size);
            assert_close(&format!("size {size} lag {lag} across"), p.autocorrelation(lag, 0), want, 0.05);
            assert_close(&format!("size {size} lag {lag} down"), p.autocorrelation(0, lag), want, 0.05);
        }
    }
}

#[test]
fn aspect_stretches_the_grain_sideways_and_softness_widens_it_without_weakening_it() {
    let wide = plane(&render(film_of(512, 512, 128, grain(json!({"intensity": 2, "size": 3, "aspect": 2})), json!({}))), 512, 512, 2);
    for lag in 1..=4usize {
        assert_close(&format!("aspect 2 lag {lag} across"), wide.autocorrelation(lag, 0), closed_form(lag as f64, 6.0), 0.05);
        assert_close(&format!("aspect 2 lag {lag} down"), wide.autocorrelation(0, lag), closed_form(lag as f64, 3.0), 0.05);
    }
    // softness adds to the variance of the kernel: the length is sqrt(size^2 + 4 softness^2), the strength is unchanged
    let soft = plane(&render(film_of(512, 512, 128, grain(json!({"intensity": 2, "size": 1.5, "softness": 2})), json!({}))), 512, 512, 2);
    let length = (1.5f64 * 1.5 + 4.0 * 2.0 * 2.0).sqrt();
    for lag in 1..=4usize {
        assert_close(&format!("softness 2 lag {lag}"), soft.autocorrelation(lag, 0), closed_form(lag as f64, length), 0.05);
    }
    assert_close("softness leaves the strength alone", soft.std() / expected_std(2.0, 1.0), 1.0, 0.04);
}

#[test]
fn saturation_mixes_a_shared_grain_with_one_per_channel_and_every_channel_keeps_its_strength() {
    for saturation in [0.0, 0.5, 1.0] {
        let px = render(card(128, grain(json!({"intensity": 1, "saturation": saturation}))));
        let (red, green, blue) = (plane(&px, W, H, 2), plane(&px, W, H, 1), plane(&px, W, H, 0));
        for (name, channel) in [("red", &red), ("green", &green), ("blue", &blue)] {
            assert_close(&format!("saturation {saturation} {name} strength"), channel.std() / expected_std(1.0, 1.0), 1.0, 0.04);
        }
        // the shared part carries 1 - saturation of the variance, so two channels correlate by exactly that
        assert_close(&format!("saturation {saturation} red-green correlation"), red.correlation(&green), 1.0 - saturation, 0.04);
        assert_close(&format!("saturation {saturation} red-blue correlation"), red.correlation(&blue), 1.0 - saturation, 0.04);
    }
}

#[test]
fn the_tone_response_scales_the_grain_by_the_band_the_brightness_is_in() {
    let gains = json!({"intensity": 1, "shadows": 2, "midtones": 0.5, "highlights": 1.5});
    for level in [64u8, 128, 192, 208] {
        let l = f64::from(level) / 255.0;
        let weight = tone(l, 2.0, 0.5, 1.5);
        let std = plane(&render(card(level, grain(gains.clone()))), W, H, 2).std();
        assert_close(&format!("grey {level}, weight {weight:.3}"), std / expected_std(1.0, weight), 1.0, 0.05);
    }
    let neutral = plane(&render(card(64, grain(json!({"intensity": 1})))), W, H, 2).std();
    assert_close("with all three gains at 1 the response is flat", neutral / expected_std(1.0, 1.0), 1.0, 0.04);
}

#[test]
fn a_preset_is_the_same_picture_as_its_values_written_out() {
    let preset = render(card(128, json!({"effect": "add_grain", "preset": "coarse_16mm", "seed": 4})));
    let written = render(card(128, json!({"effect": "add_grain", "intensity": 1.1, "size": 2.2, "softness": 0.3, "saturation": 0.3, "shadows": 0.9, "highlights": 0.5, "seed": 4})));
    assert_eq!(preset, written);
    assert_ne!(preset, render(card(128, json!({"effect": "add_grain", "preset": "fine_35mm", "seed": 4}))), "the presets are different looks");
}

#[test]
fn the_same_film_renders_the_same_bytes_and_a_frame_or_a_seed_changes_the_grain() {
    let film = |seed: u32| card(128, grain(json!({"intensity": 1, "seed": seed})));
    let a = render_frames(film(0), &[0, 1, 0]);
    assert_eq!(a[0], a[2], "a frame redrawn is the frame it was");
    assert_eq!(a[0], render(film(0)), "and so is a frame drawn by another renderer");
    let (first, second) = (plane(&a[0], W, H, 2), plane(&a[1], W, H, 2));
    assert_close("frame 0 against frame 1", first.correlation(&second), 0.0, 0.03);
    let other_seed = plane(&render(film(1)), W, H, 2);
    assert_close("seed 0 against seed 1", first.correlation(&other_seed), 0.0, 0.03);
    let frozen = render_frames(card(128, grain(json!({"intensity": 1, "rate": 0}))), &[0, 5]);
    assert_eq!(frozen[0], frozen[1], "a rate of 0 freezes the grain");
}

/// The BGRA pixels of the grain card placed `shift` pixels to the right, in `space`.
fn shifted(space: &str, shift: f64) -> Vec<u8> {
    let layer = json!({"type": "solid", "color": "#808080", "size": [128, 128], "pos": [96.0 + shift, 128.0], "fx": [grain(json!({"intensity": 1, "size": 2}))]});
    render(json!({"width": W, "height": H, "fps": 24, "dur": 1, "bg": "#000000", "effect_space": space, "layers": [layer]}))
}

#[test]
fn the_grain_is_anchored_to_the_layer_in_layer_space_and_to_the_frame_in_screen_space() {
    let (before, after) = (shifted("layer", 0.0), shifted("layer", 20.0));
    // in layer space the card's pixel (x, y) carries the same grain wherever the card stands
    let at = |px: &[u8], x: usize, y: usize| px[(y * W + x) * 4 + 2];
    for (x, y) in [(40, 70), (60, 100), (100, 128), (120, 160)] {
        assert_eq!(at(&before, x, y), at(&after, x + 20, y), "layer space: card pixel ({}, {}) moved with the card", x - 32, y - 64);
    }
    // in screen space the grain stays with the frame: the same screen pixel keeps its grain while the card slides over it
    let (before, after) = (shifted("screen", 0.0), shifted("screen", 20.0));
    for (x, y) in [(60, 70), (80, 100), (100, 128), (140, 160)] {
        assert_eq!(at(&before, x, y), at(&after, x, y), "screen space: frame pixel ({x}, {y}) kept its grain");
    }
}

#[test]
fn both_effect_spaces_make_grain_of_the_same_statistics() {
    let stats = |space: &str| {
        let film = film_of(512, 512, 128, grain(json!({"intensity": 1, "size": 2})), json!({"effect_space": space}));
        let p = plane(&render(film), 512, 512, 2);
        (p.std(), p.mean(), p.autocorrelation(1, 0), p.autocorrelation(2, 0))
    };
    let (screen, layer) = (stats("screen"), stats("layer"));
    assert_close("std", layer.0 / screen.0, 1.0, 0.01);
    assert_close("mean", layer.1, screen.1, 0.3);
    assert_close("autocorrelation at 1", layer.2, screen.2, 0.02);
    assert_close("autocorrelation at 2", layer.3, screen.3, 0.02);
    assert_close("and the std is the spec's", screen.0 / expected_std(1.0, 1.0), 1.0, 0.03);
}

#[test]
fn a_linear_light_film_gets_the_same_grain_because_the_strength_is_in_display_numbers() {
    let stats = |extra: Value| {
        let p = plane(&render(film_of(512, 512, 128, grain(json!({"intensity": 1})), extra)), 512, 512, 2);
        (p.std(), p.mean())
    };
    let (display, light) = (stats(json!({})), stats(json!({"color": {"linear_light": true}})));
    assert_close("std in light against display", light.0 / display.0, 1.0, 0.02);
    assert_close("mean in light against display", light.1, display.1, 0.5);
    assert_close("and the std is the spec's", light.0 / expected_std(1.0, 1.0), 1.0, 0.03);
}

#[test]
fn grain_never_fogs_what_is_transparent_and_zero_intensity_changes_nothing() {
    let small = json!({"type": "solid", "color": "#808080", "size": [64, 64], "pos": [128, 128], "fx": [grain(json!({"intensity": 3}))]});
    let px = render(json!({"width": W, "height": H, "fps": 24, "dur": 1, "bg": "#000000", "layers": [small]}));
    assert!(px[..W * 4 * 32].chunks_exact(4).all(|p| p == [0, 0, 0, 255]), "above the card the frame is its black background");
    let fx = |fx: Value| film_of(W, H, 77, fx, json!({}));
    assert_eq!(render(fx(grain(json!({"intensity": 0})))), render(json!({"width": W, "height": H, "fps": 24, "dur": 2, "bg": "#000000", "layers": [{"type": "solid", "color": "#4d4d4d"}]})));
}
