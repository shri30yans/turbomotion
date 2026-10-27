//! A box is exactly as wide as the shaped characters plus the padding: ten strings, three fonts, kerning pairs, ligatures,
//! blank edges and several lines.

use serde_json::json;

use crate::common::*;

/// Kerning pairs (AV, AT, To, Wa, Yo), ligature candidates, narrow and wide letters, edge spaces and line breaks.
const STRINGS: [&str; 12] = [
    "AVATAR",
    "To Wave",
    "Yo-yo",
    "WAVE",
    "iiii MMMM",
    "Hamburgefonstiv",
    "fi fl ffi",
    "The quick brown fox",
    "  padded  ",
    "Wave\nAVATAR to",
    "one\nTo\nWAVE yo",
    "a\n \nb",
];
const PAD: [f64; 2] = [24.0, 12.0];

fn boxed(text: &str, font: &str, per_line: bool) -> serde_json::Value {
    let mut b = red_box(json!(PAD));
    b["per_line"] = json!(per_line);
    content_at(film(1800, 700, json!([text_layer(json!(text), font, json!({"box": b}))])), 0.0)
}

#[test]
fn one_box_is_the_widest_extent_of_the_characters_plus_twice_the_padding_in_every_font() {
    let mut cases = 0;
    for font in FONTS {
        for text in STRINGS {
            let c = boxed(text, font, false);
            let r = rects(&c);
            assert_eq!(r.len(), 1, "{font} {text:?}: one box for the block");
            let m = measured(text, font);
            let extents = line_extents(&m.chars);
            let (left, right) = (extents.iter().map(|e| e[0]).fold(f64::MAX, f64::min), extents.iter().map(|e| e[1]).fold(f64::MIN, f64::max));
            assert!((width(r[0]) - (right - left + 2.0 * PAD[0])).abs() <= TOLERANCE, "{font} {text:?}: box {} px, text {} px + 2 x {}", width(r[0]), right - left, PAD[0]);
            assert!((r[0][0] - (left - PAD[0])).abs() <= TOLERANCE, "{font} {text:?}: the box starts {} px left of the first character, not {}", left - r[0][0], PAD[0]);
            let lines = m.bounds.as_ref().expect("bounds").lines.len() as f64;
            assert!(r[0][3] - r[0][1] > lines * (PAD[1] + 20.0), "{font} {text:?}: tall enough for {lines} lines");
            assert_eq!(c["bounds"]["block"]["width"].as_f64().map(|w| (w * 1000.0).round()), Some(((right - left) * 1000.0).round()), "{font} {text:?}: `comp inspect` reports the text bounds the box is fitted to");
            cases += 1;
        }
    }
    assert_eq!(cases, 36);
}

#[test]
fn a_single_line_is_never_a_pixel_narrower_or_wider_than_the_layout_width() {
    for font in FONTS {
        for text in ["AVATAR", "To Wave", "Yo-yo", "WAVE", "Hamburgefonstiv", "The quick brown fox"] {
            let w = text_width(text, font);
            let layout = f64::from(measured(text, font).width);
            assert!((0.0..1.0).contains(&(layout - w)), "{font} {text:?}: the layout width {layout} is the characters' {w} px rounded up");
        }
    }
}

#[test]
fn kerning_pairs_make_a_proportional_font_narrower_than_its_letters_alone_and_a_monospace_font_the_same() {
    let advance = |c: char, font: &str| f64::from(measured(&c.to_string(), font).chars[0].w);
    let alone = |text: &str, font: &str| -> f64 { text.chars().map(|c| advance(c, font)).sum() };
    for font in [FONTS[0], FONTS[1]] {
        for text in ["AVATAR", "WAVE", "ToYo"] {
            let saved = alone(text, font) - text_width(text, font);
            assert!(saved > 3.0, "{font} {text:?}: kerning pulls the letters together by {saved} px");
        }
    }
    for text in ["AVATAR", "WAVE", "ToYo"] {
        let (letters, together) = (alone(text, FONTS[2]), text_width(text, FONTS[2]));
        assert!((letters - together).abs() <= TOLERANCE, "{} {text:?}: {} characters of {} px make {together} px", FONTS[2], text.len(), letters / text.len() as f64);
    }
}

#[test]
fn a_box_per_line_is_as_wide_as_each_line_and_one_box_is_as_wide_as_the_longest() {
    for font in FONTS {
        for text in ["Wave\nAVATAR to", "one\nTo\nWAVE yo", "a\n \nb"] {
            let extents = line_extents(&measured(text, font).chars);
            let each = rects(&boxed(text, font, true));
            assert_eq!(each.len(), extents.len(), "{font} {text:?}: a box for every line that has characters");
            for (r, e) in each.iter().zip(&extents) {
                assert!((width(*r) - (e[1] - e[0] + 2.0 * PAD[0])).abs() <= TOLERANCE, "{font} {text:?}: a line {} px wide, a box {}", e[1] - e[0], width(*r));
                assert!((r[0] - (e[0] - PAD[0])).abs() <= TOLERANCE);
            }
            assert!(each.windows(2).all(|p| p[1][1] > p[0][1] && p[1][3] > p[0][3]), "{font} {text:?}: the boxes follow the lines down");
            let one = rects(&boxed(text, font, false));
            let widest = extents.iter().map(|e| e[1] - e[0]).fold(f64::MIN, f64::max);
            let block = extents.iter().map(|e| e[1]).fold(f64::MIN, f64::max) - extents.iter().map(|e| e[0]).fold(f64::MAX, f64::min);
            assert!(width(one[0]) - 2.0 * PAD[0] >= widest - TOLERANCE && (width(one[0]) - 2.0 * PAD[0] - block).abs() <= TOLERANCE, "{font} {text:?}: the block's box covers every line");
        }
    }
}

#[test]
fn a_per_line_box_is_as_tall_as_its_line_and_the_lines_do_not_overlap_without_padding() {
    let c = content_at(film(1800, 700, json!([text_layer(json!("one\ntwo\nthree"), FONTS[0], json!({"box": {"fit": "text", "padding": [0, 0], "fill": "#fff", "per_line": true}}))])), 0.0);
    let r = rects(&c);
    assert_eq!(r.len(), 3);
    let height = r[0][3] - r[0][1];
    assert!(r.iter().all(|b| ((b[3] - b[1]) - height).abs() <= TOLERANCE), "every line is {height} px tall");
    assert!(r.windows(2).all(|p| (p[1][1] - p[0][3]).abs() <= TOLERANCE + 40.0 && p[1][1] >= p[0][1] + height - 1.0), "{r:?}");
}
