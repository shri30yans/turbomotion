//! A box follows the characters showing: a typed reveal grows it character by character, and an animator that moves,
//! shrinks, spaces or masks characters moves its edges as the eye sees them.

use serde_json::{json, Value};

use crate::common::*;

const EACH: f64 = 0.1;
const PAD: f64 = 24.0;
const STRINGS: [&str; 3] = ["AVATAR Wave", "To Yo-yo fi", "Hamburgefonstiv"];

fn with(text: Value, font: &str, extra: Value) -> Value {
    let mut layer = text_layer(text, font, extra);
    layer["box"] = red_box(json!([PAD, 12]));
    film(1800, 600, json!([layer]))
}

/// The text up to and including its `n`th character that is not a space: what has been typed after `n` keystrokes.
fn typed(text: &str, n: usize) -> String {
    let mut seen = 0;
    text.chars().take_while(|c| { let before = seen; seen += usize::from(!c.is_whitespace()); before < n }).collect()
}

/// Types `text` a character a time and returns the widest gap, px, between the box and the same text typed alone,
/// having asserted that the box grows at every keystroke.
fn typed_error(text: &str, font: &str, tracking: f64) -> f64 {
    let film = with(json!(text), font, json!({"tracking": tracking, "preset": {"name": "typewriter", "each": EACH}}));
    let (mut previous, mut worst) = (0.0, 0.0_f64);
    for n in 1..=text.chars().filter(|c| !c.is_whitespace()).count() {
        let t = (n as f64 - 0.5) * EACH;
        let r = rects(&content_at(film.clone(), t));
        assert_eq!(r.len(), 1, "{font} {text:?} t={t}");
        let alone = tracked_width(&typed(text, n), font, tracking) + 2.0 * PAD;
        worst = worst.max((width(r[0]) - alone).abs());
        assert!(width(r[0]) > previous, "{font} {text:?} t={t}: {} px does not grow from {previous}", width(r[0]));
        previous = width(r[0]);
    }
    worst
}

#[test]
fn a_typewriter_reveal_grows_the_box_to_the_width_of_the_text_typed_so_far_at_every_sampled_time() {
    let mut samples = 0;
    for font in FONTS {
        for text in STRINGS {
            for tracking in [0.0, 6.0] {
                let worst = typed_error(text, font, tracking);
                assert!(worst <= TOLERANCE, "{font} {text:?} tracking {tracking}: the box is {worst} px from the text typed alone");
                samples += text.chars().filter(|c| !c.is_whitespace()).count();
            }
        }
    }
    // 10, 9 and 15 characters that are not spaces, with and without tracking, in each of three fonts
    assert_eq!(samples, 3 * 2 * (10 + 9 + 15));
}

#[test]
fn a_ligature_typed_half_way_is_close_to_the_text_typed_alone_and_exact_where_it_ends() {
    let film = |text: &str| with(json!(text), FONTS[1], json!({"preset": {"name": "typewriter", "each": EACH}}));
    let at = |text: &str, n: usize| width(rects(&content_at(film(text), (n as f64 - 0.5) * EACH))[0]);
    // fi and fl are clusters of two; the box is exact once the cluster is whole
    assert!((at("fi fl", 2) - (tracked_width("fi", FONTS[1], 0.0) + 2.0 * PAD)).abs() <= TOLERANCE);
    assert!((at("fi fl", 4) - (tracked_width("fi fl", FONTS[1], 0.0) + 2.0 * PAD)).abs() <= TOLERANCE);
    // half way through `ffi` the typed `ff` alone would shape as a ligature of its own width: the one place a typed box
    // differs from the text typed alone by more than half a pixel (measured: 3.65 px in the worst of the three fonts)
    let worst = FONTS.iter().map(|f| typed_error("fi fl ffi", f, 0.0)).fold(0.0, f64::max);
    assert!((0.5..=4.0).contains(&worst), "half-typed ligatures are {worst} px from the text typed alone");
}

#[test]
fn before_the_first_character_types_there_is_no_box_and_after_the_last_it_is_the_whole_text() {
    let film = with(json!("AVATAR"), FONTS[0], json!({"preset": {"name": "typewriter", "each": EACH, "at": 0.5}}));
    assert!(rects(&content_at(film.clone(), 0.25)).is_empty());
    assert_eq!(rects(&content_at(film.clone(), 0.55)).len(), 1);
    let done = rects(&content_at(film, 3.0))[0];
    assert!((width(done) - (text_width("AVATAR", FONTS[0]) + 2.0 * PAD)).abs() <= TOLERANCE);
}

#[test]
fn a_typed_swap_is_the_width_of_the_prefix_typed_so_far() {
    for font in FONTS {
        for text in STRINGS {
            let len = text.chars().count();
            let film = with(json!({"from": "", "to": text, "dur": 1.0, "ease": "none"}), font, json!({}));
            let mut previous = 0.0;
            for n in 1..=len {
                let t = (n as f64 - 0.5) / len as f64;
                let prefix: String = text.chars().take(n).collect();
                let r = rects(&content_at(film.clone(), t));
                let expected = if prefix.trim().is_empty() { 0.0 } else { text_width(&prefix, font) + 2.0 * PAD };
                assert_eq!(r.len(), usize::from(expected > 0.0));
                let drawn = r.first().map_or(0.0, |r| width(*r));
                assert!((drawn - expected).abs() <= TOLERANCE, "{font} {prefix:?}: box {drawn} px, prefix {expected} px");
                assert!(drawn >= previous, "{font} {prefix:?}: the box never shrinks");
                previous = drawn;
            }
        }
    }
}

#[test]
fn a_typewriter_over_two_lines_gives_a_box_to_each_line_as_it_is_reached() {
    let mut layer = text_layer(json!("Wave\nAVATAR"), FONTS[1], json!({"preset": {"name": "typewriter", "each": EACH}}));
    layer["box"] = json!({"fit": "text", "padding": [PAD, 12], "fill": "#e63946", "per_line": true});
    let film = film(1800, 600, json!([layer]));
    let counts: Vec<usize> = (1..=10).map(|n| rects(&content_at(film.clone(), (f64::from(n) - 0.5) * EACH)).len()).collect();
    assert_eq!(counts, [1, 1, 1, 1, 2, 2, 2, 2, 2, 2], "Wave is four characters, then AVATAR starts the second line");
}

/// The rest box of `text` in `font` with a static animator `props` (weight 1 until `at`, which is never reached).
fn animated(text: &str, font: &str, props: Value) -> [f64; 4] {
    let film = with(json!(text), font, json!({"animators": [{"by": "chars", "props": props, "at": 100.0}]}));
    rects(&content_at(film, 0.0))[0]
}

fn rest(text: &str, font: &str) -> [f64; 4] {
    rects(&content_at(with(json!(text), font, json!({})), 0.0))[0]
}

#[test]
fn an_offset_moves_the_box_and_a_scale_pulls_its_edges_in_by_the_glyph_boxes() {
    for font in FONTS {
        let (still, text) = (rest("AVATAR", font), "AVATAR");
        let moved = animated(text, font, json!({"x": 30, "y": -5}));
        assert!((moved[0] - still[0] - 30.0).abs() <= TOLERANCE && (moved[2] - still[2] - 30.0).abs() <= TOLERANCE && (moved[1] - still[1] + 5.0).abs() <= TOLERANCE, "{font}: moved {moved:?} from {still:?}");
        let chars = measured(text, font).chars;
        let (first, last) = (&chars[0], &chars[chars.len() - 1]);
        let small = animated(text, font, json!({"scale": -0.5}));
        assert!((small[0] - (still[0] + f64::from(first.w) / 4.0)).abs() <= TOLERANCE, "{font}: half-size letters start a quarter of the first letter in: {small:?} from {still:?}");
        assert!((small[2] - (still[2] - f64::from(last.w) / 4.0)).abs() <= TOLERANCE, "{font}: and end a quarter of the last letter in");
    }
}

#[test]
fn tracking_widens_the_box_by_the_space_added_between_the_characters() {
    for font in FONTS {
        let (still, tracked) = (rest("AVATAR", font), animated("AVATAR", font, json!({"tracking": 10})));
        let by = width(tracked) - width(still);
        assert!((by - 50.0).abs() <= TOLERANCE, "{font}: 6 characters, 5 gaps of 10 px: the box grew {by} px");
    }
}

#[test]
fn a_masked_rise_cuts_the_box_to_what_has_come_out_from_behind_the_mask() {
    for font in FONTS {
        let film = with(json!("AVATAR"), font, json!({"preset": {"name": "rise", "at": 0.0, "dur": 1.0}}));
        let heights: Vec<f64> = [0.05, 0.2, 0.4, 0.8, 3.0].iter().map(|&t| rects(&content_at(film.clone(), t)).first().map_or(0.0, |r| r[3] - r[1])).collect();
        let full = rest("AVATAR", font);
        assert!(heights.windows(2).all(|p| p[1] >= p[0]), "{font}: the box only grows as the line rises: {heights:?}");
        assert!(heights[0] < heights[4], "{font}: it starts smaller than it ends: {heights:?}");
        assert!((heights[4] - (full[3] - full[1])).abs() <= TOLERANCE, "{font}: and ends as the rest box: {heights:?}");
    }
}
