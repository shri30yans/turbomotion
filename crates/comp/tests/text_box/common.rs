//! What the box tests share: the three bundled fonts, a film with one text layer, and the engine's own measurement.

use std::{collections::BTreeMap, path::{Path, PathBuf}};

use serde_json::{json, Value};
use turbomotion_comp::{
    api,
    plan::Plan,
    render::Renderer,
    text::{
        layout::{Engine, Fonts},
        measure::{measure, CharBox, Measured, Request},
    },
};

/// The three bundled faces the tests use: a variable sans (kerning), a serif (kerning, ligatures) and a monospace.
pub const FONTS: [&str; 3] = ["Manrope-Variable.ttf", "InstrumentSerif-Regular.woff", "JetBrainsMono-Regular.woff"];
pub const SIZE: f64 = 80.0;
pub const LAYER: &str = "t";
/// The farthest a drawn or measured edge may be from the expected one, px.
pub const TOLERANCE: f64 = 0.5;

pub fn fonts_dir() -> PathBuf {
    std::env::var_os("TURBOMOTION_TEST_FONTS").map_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../motions/lib/fonts"), PathBuf::from)
}

/// A film `width` x `height` with `layers` on a black ground.
pub fn film(width: u32, height: u32, layers: Value) -> Value {
    json!({"width": width, "height": height, "fps": 30, "dur": 4, "bg": "#000000", "layers": layers})
}

/// A white text layer `t` in `font` at `size`, its left edge at (40, 200), with `extra` keys merged over it.
pub fn text_layer(text: Value, font: &str, extra: Value) -> Value {
    let mut layer = json!({"id": LAYER, "type": "text", "text": text, "font": font, "weight": 400, "size": SIZE, "color": "#ffffff", "pos": [40, 200]});
    layer.as_object_mut().expect("an object literal").extend(extra.as_object().cloned().unwrap_or_default());
    layer
}

pub fn red_box(padding: Value) -> Value {
    json!({"fit": "text", "padding": padding, "fill": "#e63946"})
}

/// The text layer as `comp inspect` reports it at `t`.
pub fn layer_at(film: Value, t: f64) -> Value {
    let frame = api::inspect(film, &fonts_dir(), t, Some(LAYER)).unwrap_or_else(|e| panic!("inspect at {t}: {e:#}"));
    frame["layers"][0].clone()
}

/// The text layer's evaluated `content` at `t`: its measured `bounds` and its `text_box`.
pub fn content_at(film: Value, t: f64) -> Value {
    layer_at(film, t)["content"].clone()
}

pub fn inspect_error(film: Value, t: f64) -> String {
    format!("{:#}", api::inspect(film, &fonts_dir(), t, Some(LAYER)).expect_err("the layer should not evaluate"))
}

pub fn rects(content: &Value) -> Vec<[f64; 4]> {
    content["text_box"]["rects"].as_array().expect("a text_box with rects").iter().map(|r| std::array::from_fn(|i| r[i].as_f64().expect("a number"))).collect()
}

pub fn width(r: [f64; 4]) -> f64 {
    r[2] - r[0]
}

/// The engine's own shaping of `text` in `font` at the film's size: what a script's `measure()` returns.
pub fn measured(text: &str, font: &str) -> Measured {
    measured_tracked(text, font, 0.0)
}

/// `measured` with `tracking` px after every character.
pub fn measured_tracked(text: &str, font: &str, tracking: f64) -> Measured {
    let book = Fonts { roles: BTreeMap::new(), root: fonts_dir() };
    let request = Request { text: text.into(), font: Some(font.into()), size: SIZE, tracking, ..Request::default() };
    measure(&Engine::new(), &book, &request).unwrap_or_else(|e| panic!("measuring {text:?} in {font}: {e:#}"))
}

/// `[left, right]` of the characters that are not whitespace, on each line that has any, from the measured boxes.
pub fn line_extents(chars: &[CharBox]) -> Vec<[f64; 2]> {
    let mut lines: BTreeMap<usize, [f64; 2]> = BTreeMap::new();
    for c in chars.iter().filter(|c| !c.ch.is_whitespace()) {
        let (left, right) = (f64::from(c.x), f64::from(c.x + c.w));
        lines.entry(c.line).and_modify(|e| *e = [e[0].min(left), e[1].max(right)]).or_insert([left, right]);
    }
    lines.into_values().collect()
}

/// The width of the characters of `text` from the first to the last of any line, as measured.
pub fn text_width(text: &str, font: &str) -> f64 {
    tracked_width(text, font, 0.0)
}

pub fn tracked_width(text: &str, font: &str, tracking: f64) -> f64 {
    let extents = line_extents(&measured_tracked(text, font, tracking).chars);
    extents.iter().map(|e| e[1]).fold(f64::MIN, f64::max) - extents.iter().map(|e| e[0]).fold(f64::MAX, f64::min)
}

/// The BGRA pixels of `film` at `t`.
pub fn render(film: Value, t: f64) -> Vec<u8> {
    let plan = Plan::from_value(film, &fonts_dir()).expect("the plan");
    let mut renderer = Renderer::new(&plan).expect("a renderer");
    renderer.draw_at(t).expect("the frame");
    let mut px = vec![];
    renderer.pixels(&mut px).expect("the pixels");
    px
}

/// How much of the pixel is box (0..1): the box and the text are white on black, so it is the red channel.
pub fn cover(px: &[u8], width: usize, x: usize, y: usize) -> f64 {
    f64::from(px[(y * width + x) * 4 + 2]) / 255.0
}

/// The two edges of the run of box along a line of `len` samples read through `at`: whole pixels plus the part of each edge pixel.
pub fn run(len: usize, at: impl Fn(usize) -> f64) -> [f64; 2] {
    let first = (0..len).find(|&i| at(i) > 0.001).expect("the line crosses the box");
    let last = (first..len).take_while(|&i| at(i) > 0.001).last().expect("a run");
    [first as f64 + 1.0 - at(first), last as f64 + at(last)]
}

/// The two places a soft-edged run of box crosses half cover along a line, by interpolating between pixel centres.
pub fn half_crossings(len: usize, at: impl Fn(usize) -> f64) -> [f64; 2] {
    let at_half = |up: bool| {
        let i = (0..len - 1).find(|&i| if up { at(i) < 0.5 && at(i + 1) >= 0.5 } else { at(i) >= 0.5 && at(i + 1) < 0.5 }).expect("the line crosses half cover");
        i as f64 + 0.5 + (0.5 - at(i)) / (at(i + 1) - at(i))
    };
    [at_half(true), at_half(false)]
}

/// The summed cover of a frame `width` x `height`: the area of the box in pixels.
pub fn area(px: &[u8], width: usize, height: usize) -> f64 {
    (0..height).flat_map(|y| (0..width).map(move |x| (x, y))).map(|(x, y)| cover(px, width, x, y)).sum()
}

/// The centre of the cover of a frame.
pub fn centroid(px: &[u8], width: usize, height: usize) -> [f64; 2] {
    let total = area(px, width, height);
    let sum = |axis: usize| (0..height).flat_map(|y| (0..width).map(move |x| (x, y))).map(|(x, y)| cover(px, width, x, y) * ([x, y][axis] as f64 + 0.5)).sum::<f64>() / total;
    [sum(0), sum(1)]
}
