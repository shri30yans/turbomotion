//! SVG, pixel sprites and checked image options, and the tween forms: vectors, colours, shape items, relative values.
mod common;
use std::fs;
use serde_json::{json, Value};
use turbomotion_script::{build, Options};

const HEAD: &str = "import { comp, shape, text, image, svg, svgFile, pixelSprite, timeline } from 'turbomotion';\nconst film = comp({ width: 400, height: 300, fps: 30, dur: 3 });\nconst tl = timeline();\n";

fn run(name: &str, body: &str) -> Value {
    let dir = common::scratch("artwork", name);
    fs::write(dir.join("film.ts"), format!("{HEAD}{body}\nexport default film;")).unwrap();
    build(&dir.join("film.ts"), &Options::default()).unwrap_or_else(|e| panic!("{e:#}"))
}
fn fails(name: &str, body: &str) -> String {
    let dir = common::scratch("artwork", name);
    fs::write(dir.join("film.ts"), format!("{HEAD}{body}\nexport default film;")).unwrap();
    format!("{:#}", build(&dir.join("film.ts"), &Options::default()).unwrap_err())
}
fn plans(value: Value, name: &str) {
    turbomotion_comp::plan::Plan::from_value(value, &common::scratch("artwork", name)).unwrap_or_else(|e| panic!("{e:#}"));
}

#[test]
fn svg_markup_and_path_data_become_shape_layers_through_the_engine_importer() {
    let v = run("svg", r##"
film.add(svg('<svg viewBox="0 0 10 10"><rect width="10" height="10" fill="#ff0000"/></svg>', { id: 'box', width: 100, pos: [200, 150], fill: '#00ff00' }));
film.add(svg('M0 0 L24 0 L12 24 Z', { id: 'tri', width: 48, stroke: '#ffffff', strokeWidth: 3, pos: [50, 50] }));"##);
    let box_ = &v["layers"][0];
    assert_eq!((box_["type"].as_str(), box_["scale"].as_f64(), box_["pos"].clone()), (Some("shape"), Some(10.0), json!([200, 150])));
    assert_eq!(box_["items"][0]["fill"], "#00ff00");
    assert_eq!(v["layers"][1]["items"][0]["stroke"]["width"], 3.0);
    plans(v, "svg-plan");
}

#[test]
fn svg_file_reads_beside_the_film_and_rejects_bad_paths() {
    let dir = common::scratch("artwork", "svgfile");
    fs::write(dir.join("icon.svg"), r##"<svg viewBox="0 0 4 4"><circle cx="2" cy="2" r="2" fill="#fff"/></svg>"##).unwrap();
    fs::write(dir.join("film.ts"), format!("{HEAD}film.add(svgFile('icon.svg', {{ width: 40 }}));\nexport default film;")).unwrap();
    let v = build(&dir.join("film.ts"), &Options::default()).unwrap();
    assert_eq!(v["layers"][0]["items"][0]["ellipse"], json!([2, 2, 2, 2]));
    assert!(fails("svgfile-missing", "film.add(svgFile('nope.svg'));").contains("cannot read"));
    assert!(fails("svgfile-abs", "film.add(svgFile('/etc/x.svg'));").contains("absolute"));
    assert!(fails("svg-bad", "film.add(svg('hello'));").contains("svg()"));
}

#[test]
fn pixel_sprites_merge_runs_and_name_the_bad_pixel() {
    let v = run("sprite", "film.add(pixelSprite(['.aab', 'aaaa'], { a: '#f00', b: '#00f' }, { px: 10, bleed: 0 }));");
    let items = v["layers"][0]["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["rect"], json!([-10, -10, 20, 10]));
    assert_eq!(items[2]["rect"], json!([-20, 0, 40, 10]));
    assert!(fails("sprite-bad", "film.add(pixelSprite(['.az'], { a: '#f00' }));").contains("\"z\" at row 0, column 2"));
    assert!(fails("sprite-ragged", "film.add(pixelSprite(['aa', 'a'], { a: '#f00' }));").contains("row 1"));
}

#[test]
fn image_options_are_checked_where_they_are_written() {
    assert!(fails("img-fit", "film.add(image('a.png', { fit: 'stretch' as any }));").contains("fit must be one of"));
    assert!(fails("img-focus", "film.add(image('a.png', { focus: [2, 0] }));").contains("focus"));
    assert!(fails("img-ext", "film.add(image('a.txt'));").contains("image file"));
    assert!(fails("img-size", "film.add(image('a.png', { size: [0, 10] }));").contains("size"));
    let v = run("img-ok", "film.add(image('a.png', { fit: 'contain', sampling: 'nearest', focus: [0.5, 0.2] }));");
    assert_eq!(v["layers"][0]["fit"], "contain");
}

#[test]
fn skew_tweens_like_any_other_property_and_starts_unsheared() {
    let v = run("skew", "const t = text('a', { id: 't', pos: [0, 0] }); tl.to(t, { skew: 20, duration: 1 }, 0); tl.to(t, { skew: '-=5', duration: 1 }, 1); film.add(t);");
    let keys = v["layers"][0]["skew"]["keys"].as_array().unwrap_or_else(|| panic!("two skew tweens merge into keys: {}", v["layers"][0]["skew"]));
    assert_eq!(keys[0][1], 0);
    assert!(keys.iter().any(|k| k[1] == 20));
    assert_eq!(keys.last().unwrap()[1], 15);
}

#[test]
fn tweens_take_vector_positions_relative_values_and_shape_items() {
    let v = run("tweens", r##"
const t = text('a', { id: 't', pos: [100, 200, 0], scale: [1, 1] });
tl.to(t, { pos: [300, '+=50', null], scale: [2, '+=1'], duration: 1 }, 0);
tl.to(t, { x: '-=100', duration: 1 }, 1);
const ring = shape([{ ellipse: [0, 0, 50, 50], stroke: { color: '#fff', width: 4 }, fill: '#f00', trim: { end: 0 } }, { rect: [0, 0, 10, 10], fill: '#00f' }], { id: 'ring' });
tl.to(ring, { items: { 0: { trim_end: 1, stroke_width: '+=4', fill_opacity: 0.5, rotation: 90 }, 1: { fill_color: '#ffffff', scale: 2 } }, duration: 1 }, 0);
film.add(t, ring);"##);
    let t = &v["layers"][0];
    assert_eq!(t["pos"][1]["to"], 250);
    assert_eq!(t["pos"][1]["from"], 200);
    assert_eq!(t["scale"]["to"], json!([2, 2]));
    // x moves to 300, then 100 back: one key list, the relative step resolved from where the first tween ended
    let keys = t["pos"][0]["keys"].as_array().unwrap_or_else(|| panic!("two x tweens merge into keys: {}", t["pos"]));
    assert_eq!(keys[0][1], 100);
    assert!(keys.iter().any(|k| k[1] == 300));
    assert_eq!(keys.last().unwrap()[1], 200);
    let items = &v["layers"][1]["items"];
    assert_eq!(items[0]["trim"]["end"]["to"], 1);
    assert_eq!(items[0]["stroke"]["width"]["to"], 8);
    assert_eq!(items[0]["fill"]["opacity"]["to"], 0.5);
    assert_eq!(items[0]["fill"]["color"], "#f00");
    assert_eq!(items[1]["fill"]["color"]["to"], "#ffffff");
    assert_eq!(items[1]["scale"]["from"], 1);
    plans(v, "tweens-plan");
    assert!(fails("tw-rect", "const r = shape([{ rect: [0,0,10,10], fill: '#fff' }]); tl.to(r, { items: { 0: { width: 20 } } }); film.add(r);").contains("fixed in the schema"));
    assert!(fails("tw-rel", "const q = text('a', { pos: [0, 0] }); tl.to(q, { opacity: '+=0.5' as any, size: '+=3', duration: 1 }); film.add(q);").contains("relative"));
}
