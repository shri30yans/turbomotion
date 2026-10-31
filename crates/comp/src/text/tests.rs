//! Text on a path, per-character 3D, extra animator properties, outline type, marquee, cipher and surfaces. Layout
//! numbers are asserted from the evaluated frame; pixels from a small real render.
//!
//! Skia measures a circle's length by flattening it, so positions on a 100 px circle agree with the exact maths to
//! about a tenth of a pixel, not to float precision: that is `CIRCLE_TOLERANCE`.

use serde_json::{json, Value};

use crate::{
    eval::{Content, Evaluator},
    plan::{Plan, ROOT},
    text::{animate::GlyphState, place},
};

/// Pixels (and, for angles, tenths of a degree) a flattened circle may stray from the exact one.
const CIRCLE_TOLERANCE: f64 = 0.25;

fn plan(layers: Value) -> Plan {
    Plan::from_value(json!({"width": 640, "height": 360, "dur": 4, "bg": "#000000", "layers": layers}), std::path::Path::new(".")).unwrap()
}

fn text_at(p: &Plan, t: f64) -> crate::eval::text::TextState {
    let f = Evaluator::new(p).frame(ROOT, t).unwrap();
    match f.layers.into_iter().find_map(|l| if let Content::Text(t) = l.content { Some(*t) } else { None }) {
        Some(t) => t,
        None => panic!("no text layer"),
    }
}

/// Where a glyph's baseline-centre point lands in layout space.
fn baseline_centre(t: &crate::eval::text::TextState, i: usize) -> ([f64; 2], f64) {
    let layout = t.layout.as_ref().unwrap();
    let lg = &layout.glyphs[t.glyphs[i].src];
    let [a, b, c, d, e, f] = t.glyphs[i].m.0.map(f64::from);
    let (x, y) = (f64::from(lg.x + lg.w / 2.0), f64::from(lg.y));
    ([a * x + c * y + e, b * x + d * y + f], b.atan2(a).to_degrees())
}

fn width_of(t: &crate::eval::text::TextState, i: usize) -> f64 {
    let layout = t.layout.as_ref().unwrap();
    let lg = &layout.glyphs[t.glyphs[i].src];
    let g: &GlyphState = &t.glyphs[i];
    let c = place::corners(g, [lg.x, lg.top, lg.x + lg.w, lg.bottom]).expect("in front of the viewer");
    let xs = c.map(|p| p[0]);
    xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min)
}

fn render(p: &Plan, t: f64) -> (usize, usize, Vec<u8>) {
    let mut r = crate::render::Renderer::new(p).unwrap();
    r.draw_at(t).unwrap();
    let mut px = vec![];
    r.pixels(&mut px).unwrap();
    let root = p.root();
    (root.width as usize, root.height as usize, px)
}

fn lit(px: &[u8], w: usize, x: usize, y: usize) -> bool {
    let i = (y * w + x) * 4;
    px[i] > 128 || px[i + 1] > 128 || px[i + 2] > 128
}

fn count_lit(px: &[u8]) -> usize {
    px.chunks_exact(4).filter(|p| p[0] > 128 || p[1] > 128 || p[2] > 128).count()
}

fn lit_x_range(px: &[u8], w: usize) -> (usize, usize) {
    let xs: Vec<usize> = px.chunks_exact(4).enumerate().filter(|(_, p)| p[0] > 128 || p[1] > 128 || p[2] > 128).map(|(i, _)| i % w).collect();
    (*xs.iter().min().unwrap(), *xs.iter().max().unwrap())
}

/// The circumference as the engine measures it.
fn circle_length() -> f64 {
    let spec: crate::schema::PathSpec = serde_json::from_value(json!({"ellipse": [0, 0, 100, 100]})).unwrap();
    let path = crate::plan::geometry::PathGeo::build(&spec).unwrap().path;
    f64::from(skia_safe::ContourMeasureIter::new(&path, false, None).next().unwrap().length())
}

fn circle_text(extra: Value) -> Plan {
    let mut layer = json!({"type": "text", "id": "t", "text": "AB", "size": 80, "weight": 700, "color": "#ffffff", "pos": [320, 180], "path": {"ellipse": [0, 0, 100, 100]}});
    for (k, v) in extra.as_object().unwrap() {
        if k == "path" {
            for (pk, pv) in v.as_object().unwrap() {
                layer["path"][pk] = pv.clone();
            }
        } else {
            layer[k] = v.clone();
        }
    }
    plan(json!([layer]))
}

// ---- T1: text on a path -------------------------------------------------------------------------------------------

#[test]
fn text_on_a_circle_puts_the_first_glyph_at_the_arc_length_of_its_centre_and_turns_it_to_the_tangent() {
    let p = circle_text(json!({}));
    let t = text_at(&p, 0.0);
    let layout = t.layout.as_ref().unwrap();
    let lg = &layout.glyphs[0];
    let u = f64::from(lg.x + lg.w / 2.0);
    let theta = u / 100.0;
    let (pt, rot) = baseline_centre(&t, 0);
    let want = [f64::from(layout.anchor[0]) + 100.0 * theta.cos(), f64::from(layout.anchor[1]) + 100.0 * theta.sin()];
    assert!((pt[0] - want[0]).abs() < CIRCLE_TOLERANCE && (pt[1] - want[1]).abs() < CIRCLE_TOLERANCE, "{pt:?} vs {want:?}");
    // the circle starts at 3 o'clock heading down the screen: a tangent of 90 degrees, plus how far round we are
    assert!((rot - (90.0 + theta.to_degrees())).abs() < CIRCLE_TOLERANCE, "rotation {rot}");
}

#[test]
fn a_quarter_of_the_circumference_of_margin_turns_the_glyph_ninety_degrees() {
    let quarter = circle_length() / 4.0;
    let (r0, r1) = {
        let a = text_at(&circle_text(json!({"path": {"first_margin": 0}})), 0.0);
        let b = text_at(&circle_text(json!({"path": {"first_margin": quarter}})), 0.0);
        (baseline_centre(&a, 0), baseline_centre(&b, 0))
    };
    let turn = (r1.1 - r0.1).rem_euclid(360.0);
    assert!((turn - 90.0).abs() < CIRCLE_TOLERANCE, "turned {turn}");
    // a quarter lap later the glyph is still on the circle, a right angle round it
    let anchor = text_at(&circle_text(json!({})), 0.0).layout.as_ref().unwrap().anchor.map(f64::from);
    let (v0, v1) = ([r0.0[0] - anchor[0], r0.0[1] - anchor[1]], [r1.0[0] - anchor[0], r1.0[1] - anchor[1]]);
    assert!((v0[0].hypot(v0[1]) - 100.0).abs() < CIRCLE_TOLERANCE && (v1[0].hypot(v1[1]) - 100.0).abs() < CIRCLE_TOLERANCE, "{v0:?} {v1:?}");
    assert!((v0[0] * v1[0] + v0[1] * v1[1]).abs() < 100.0 * CIRCLE_TOLERANCE, "{v0:?} {v1:?}");
}

#[test]
fn a_margin_that_grows_carries_text_round_a_closed_path_and_loops() {
    let circ = circle_length();
    let at = |m: f64| baseline_centre(&text_at(&circle_text(json!({"path": {"first_margin": m}})), 0.0), 1).0;
    let (a, b) = (at(10.0), at(10.0 + circ));
    assert!((a[0] - b[0]).abs() < 0.01 && (a[1] - b[1]).abs() < 0.01, "{a:?} {b:?}");
    // `travel` is a fraction of the measured length: 0 to 1 over 2 s is one exact lap, so t=2 is where t=0 began
    let p = circle_text(json!({"path": {"travel": {"from": 0, "to": 1, "dur": 2, "ease": "none"}}}));
    let (s, m, e) = (baseline_centre(&text_at(&p, 0.0), 0).0, baseline_centre(&text_at(&p, 1.0), 0).0, baseline_centre(&text_at(&p, 2.0), 0).0);
    assert!((s[0] - e[0]).abs() < 1e-3 && (s[1] - e[1]).abs() < 1e-3, "{s:?} {e:?}");
    assert!(m[0] < s[0] - 100.0, "half way round is on the far side of the circle: {m:?} from {s:?}");
}

#[test]
fn on_an_open_path_characters_past_the_end_are_hidden_and_reverse_and_upright_work() {
    let line = |extra: Value| {
        let mut l = json!({"type": "text", "text": "ABCDEFGH", "size": 60, "color": "#fff", "pos": [320, 180], "path": {"points": [[0, 0], [150, 0]]}});
        for (k, v) in extra.as_object().unwrap() {
            l["path"][k] = v.clone();
        }
        plan(json!([l]))
    };
    let t = text_at(&line(json!({})), 0.0);
    assert!(t.glyphs[0].opacity > 0.99, "first glyph is on the path");
    assert!(t.glyphs.last().unwrap().opacity < 0.01, "last glyph runs off the end");
    // an upright run keeps every glyph unrotated
    let t = text_at(&line(json!({"perpendicular": false})), 0.0);
    assert!(t.glyphs.iter().all(|g| (g.m.0[0] - 1.0).abs() < 1e-5 && g.m.0[1].abs() < 1e-5));
    // reverse walks the path from its far end: the first glyph starts at x = 150 heading back at 180 degrees
    let t = text_at(&line(json!({"reverse": true})), 0.0);
    let (p0, rot) = baseline_centre(&t, 0);
    let layout = t.layout.as_ref().unwrap();
    assert!((rot.rem_euclid(360.0) - 180.0).abs() < 0.05, "{rot}");
    let lg = &layout.glyphs[0];
    assert!((p0[0] - (f64::from(layout.anchor[0]) + 150.0 - f64::from(lg.x + lg.w / 2.0))).abs() < 0.01, "{p0:?}");
}

#[test]
fn force_align_spreads_the_text_across_the_whole_stretch_between_the_margins() {
    let p = plan(json!([{"type": "text", "text": "AB", "size": 40, "color": "#fff", "pos": [320, 180], "path": {"points": [[0, 0], [400, 0]], "force_align": true, "first_margin": 20, "last_margin": 20}}]));
    let t = text_at(&p, 0.0);
    let layout = t.layout.as_ref().unwrap();
    let k = (400.0 - 40.0) / f64::from(layout.width);
    let lg = &layout.glyphs[1];
    let (pt, _) = baseline_centre(&t, 1);
    let want = f64::from(layout.anchor[0]) + 20.0 + f64::from(lg.x + lg.w / 2.0) * k;
    assert!((pt[0] - want).abs() < 0.05, "{} vs {want}", pt[0]);
}

#[test]
fn a_glyph_lands_on_the_circle_in_pixels_and_a_path_can_borrow_another_layers_shape() {
    let on_circle = |margin: f64| {
        plan(json!([
            {"type": "text", "id": "t", "text": "I", "size": 120, "weight": 800, "color": "#ffffff", "pos": [320, 180], "path": {"ellipse": [0, 0, 100, 100], "first_margin": margin}}
        ]))
    };
    // start the glyph's centre exactly at 3 o'clock
    let at_start = text_at(&on_circle(0.0), 0.0);
    let lg = &at_start.layout.as_ref().unwrap().glyphs[0];
    let p = on_circle(-f64::from(lg.x + lg.w / 2.0));
    let (w, _, px) = render(&p, 0.0);
    // the glyph's centre sits on 3 o'clock, its stem runs outward along +x, and the inside of the circle is empty
    let t = text_at(&p, 0.0);
    let (pt, _) = baseline_centre(&t, 0);
    let anchor = t.layout.as_ref().unwrap().anchor;
    assert!((pt[0] - f64::from(anchor[0]) - 100.0).abs() < CIRCLE_TOLERANCE && (pt[1] - f64::from(anchor[1])).abs() < CIRCLE_TOLERANCE, "{pt:?}");
    assert!(lit(&px, w, 440, 180), "the stem should cover x=440 at the circle's right edge");
    assert!(!lit(&px, w, 380, 180), "inside the circle stays black");
    // borrowing a shape layer
    let q = plan(json!([
        {"type": "shape", "id": "ring", "pos": [0, 0], "items": [{"ellipse": [0, 0, 100, 100], "stroke": {"color": "#333", "width": 1}}]},
        {"type": "text", "id": "t", "text": "AB", "size": 40, "weight": 700, "color": "#fff", "pos": [320, 180], "path": {"layer": "ring"}}
    ]));
    let a = text_at(&q, 0.0);
    let b = text_at(&circle_text(json!({"size": 40})), 0.0);
    let (pa, pb) = (baseline_centre(&a, 1).0, baseline_centre(&b, 1).0);
    assert!((pa[0] - pb[0]).abs() < 0.01 && (pa[1] - pb[1]).abs() < 0.01);
}

fn refusal(layers: Value) -> String {
    format!("{:#}", Plan::from_value(json!({"dur": 1, "layers": layers}), std::path::Path::new(".")).err().expect("refused when planned"))
}

#[test]
fn a_text_path_is_checked_when_the_composition_is_planned() {
    let text = |path: Value| json!({"type": "text", "text": "x", "path": path});
    assert!(refusal(json!([text(json!({"first_margin": 3}))])).contains("exactly one of a geometry"));
    assert!(refusal(json!([text(json!({"ellipse": [0, 0, 5, 5], "layer": "a"}))])).contains("exactly one of a geometry"));
    let missing = refusal(json!([{"type": "shape", "id": "ring", "items": [{"ellipse": [0, 0, 5, 5]}]}, text(json!({"layer": "rnig"}))]));
    assert!(missing.contains("\"rnig\" does not exist") && missing.contains("ring"), "{missing}");
    let no_shape = refusal(json!([{"type": "solid", "id": "bg", "color": "#000"}, text(json!({"layer": "bg"}))]));
    assert!(no_shape.contains("no shape item"), "{no_shape}");
    let flat = refusal(json!([text(json!({"points": [[5, 5], [5, 5]]}))]));
    assert!(flat.contains("no length"), "{flat}");
    // the layer may be declared after the text that follows it
    assert!(Plan::from_value(json!({"dur": 1, "layers": [text(json!({"layer": "ring"})), {"type": "shape", "id": "ring", "items": [{"ellipse": [0, 0, 5, 5]}]}]}), std::path::Path::new(".")).is_ok());
}

// ---- T2: per-character 3D ------------------------------------------------------------------------------------------

fn one_glyph(props: Value, perspective: Value) -> Plan {
    plan(json!([{"type": "text", "text": "M", "size": 120, "weight": 700, "color": "#ffffff", "pos": [320, 180], "align": "center", "perspective": perspective,
        "animators": [{"by": "chars", "props": props, "selector": {"start": 0, "end": 1}}]}]))
}

#[test]
fn rotation_y_ninety_collapses_a_glyph_and_forty_five_shortens_it_by_cosine() {
    let flat = width_of(&text_at(&one_glyph(json!({}), json!("orthographic")), 0.0), 0);
    let w45 = width_of(&text_at(&one_glyph(json!({"rotation_y": 45}), json!("orthographic")), 0.0), 0);
    let w90 = width_of(&text_at(&one_glyph(json!({"rotation_y": 90}), json!("orthographic")), 0.0), 0);
    assert!((w45 / flat - 45f64.to_radians().cos()).abs() < 1e-3, "{w45} / {flat}");
    assert!(w90 < 1e-6, "collapsed width {w90}");
    // with perspective a glyph near the vanishing point is edge-on too (its box is not exactly centred on the anchor)
    let w90p = width_of(&text_at(&one_glyph(json!({"rotation_y": 90}), json!(800)), 0.0), 0);
    assert!(w90p < 0.05 * flat, "{w90p} of {flat}");
    // rotation_x shortens height, not width
    let t = text_at(&one_glyph(json!({"rotation_x": 60}), json!("orthographic")), 0.0);
    assert!((width_of(&t, 0) - flat).abs() < 1e-3);
    // the opposite turn foreshortens by the same amount
    let wn = width_of(&text_at(&one_glyph(json!({"rotation_y": -45}), json!("orthographic")), 0.0), 0);
    assert!((wn - w45).abs() < 1e-6);
}

#[test]
fn per_glyph_depth_follows_the_layer_convention_and_perspective_shrinks_far_glyphs() {
    // positive z is away from the viewer, as a layer's `pos` z is: the glyph gets smaller, by d / (d + z)
    let flat = width_of(&text_at(&one_glyph(json!({}), json!(500)), 0.0), 0);
    let far = width_of(&text_at(&one_glyph(json!({"z": 500}), json!(500)), 0.0), 0);
    assert!((far / flat - 0.5).abs() < 1e-3, "{far} / {flat}");
    let near = width_of(&text_at(&one_glyph(json!({"z": -250}), json!(500)), 0.0), 0);
    assert!((near / flat - 2.0).abs() < 1e-3, "{near} / {flat}");
    // a flat layer that names no perspective is seen from where the default camera is
    let d = crate::schema::default_camera_zoom(640.0);
    let default_far = width_of(&text_at(&one_glyph(json!({"z": 500}), Value::Null), 0.0), 0);
    assert!((default_far / flat - d / (d + 500.0)).abs() < 1e-3, "{default_far} / {flat}");
    // same sign as the layer: a layer with rotation [0, 40, 0] and a glyph with rotation_y 40 turn the same way
    let m = crate::eval::mat4::Mat4::rot_y(40.0).apply(1.0, 0.0, 0.0);
    let t = text_at(&one_glyph(json!({"rotation_y": 40}), json!("orthographic")), 0.0);
    let layout = t.layout.as_ref().unwrap();
    let lg = &layout.glyphs[0];
    let m4 = crate::eval::mat4::Mat4(t.glyphs[0].m4.expect("3D glyph has a 4x4"));
    let (cx, cy) = (lg.x + lg.w / 2.0, (lg.top + lg.bottom) / 2.0);
    let o = m4.apply(f64::from(cx), f64::from(cy), 0.0);
    let r = m4.apply(f64::from(cx) + 1.0, f64::from(cy), 0.0);
    assert!(((r[2] - o[2]) - m[2]).abs() < 1e-9, "z of the glyph's x axis: {} vs the layer's {}", r[2] - o[2], m[2]);
    // a flat glyph has no 4x4 at all
    assert!(text_at(&one_glyph(json!({}), json!("orthographic")), 0.0).glyphs[0].m4.is_none());
}

#[test]
fn glyph_turns_compose_like_a_layers_with_z_reaching_the_glyph_first() {
    use crate::eval::mat4::Mat4;
    let t = text_at(&one_glyph(json!({"rotation_x": 90, "rotation_z": 90}), json!("orthographic")), 0.0);
    let lg = &t.layout.as_ref().unwrap().glyphs[0];
    let m4 = Mat4(t.glyphs[0].m4.expect("3D glyph has a 4x4"));
    let (cx, cy) = (f64::from(lg.x + lg.w / 2.0), f64::from((lg.top + lg.bottom) / 2.0));
    let o = m4.apply(cx, cy, 0.0);
    let r = m4.apply(cx + 1.0, cy, 0.0);
    // Rz(90) lays the glyph's x axis along y, then Rx(90) throws that into depth; the other order would leave it in the plane
    let want = Mat4::rot_x(90.0).mul(&Mat4::rot_z(90.0)).apply(1.0, 0.0, 0.0);
    for k in 0..3 {
        assert!(((r[k] - o[k]) - want[k]).abs() < 1e-9, "axis {k}: {:?} vs {want:?}", [r[0] - o[0], r[1] - o[1], r[2] - o[2]]);
    }
    assert!(want[2].abs() > 0.99, "{want:?}");
}

#[test]
fn a_turned_glyph_renders_narrower_on_screen() {
    let (w, _, a) = render(&one_glyph(json!({}), json!("orthographic")), 0.0);
    let (_, _, b) = render(&one_glyph(json!({"rotation_y": 60}), json!("orthographic")), 0.0);
    let (a0, a1) = lit_x_range(&a, w);
    let (b0, b1) = lit_x_range(&b, w);
    let ratio = (b1 - b0) as f64 / (a1 - a0) as f64;
    assert!((ratio - 0.5).abs() < 0.04, "pixel width ratio {ratio}");
}

// ---- T3: hue, stroke width, outline type, trim ---------------------------------------------------------------------

#[test]
fn fill_hue_turns_red_to_green_and_the_hsv_maths_round_trips() {
    use crate::text::animate::shift_hsv;
    let g = shift_hsv([255.0, 0.0, 0.0], 120.0, 0.0, 0.0);
    assert!(g[1] > 254.0 && g[0] < 1.0 && g[2] < 1.0, "{g:?}");
    let grey = shift_hsv([255.0, 0.0, 0.0], 0.0, -1.0, 0.0);
    assert!((grey[0] - grey[1]).abs() < 1e-3 && (grey[1] - grey[2]).abs() < 1e-3);
    let dark = shift_hsv([255.0, 128.0, 0.0], 0.0, 0.0, -1.0);
    assert!(dark.iter().all(|c| c.abs() < 1e-3));
    let plan_of = |hue: f64| {
        plan(json!([{"type": "text", "text": "IIII", "size": 200, "weight": 800, "color": "#ff0000", "pos": [320, 180], "align": "center",
            "animators": [{"by": "chars", "props": {"fill_hue": hue}, "selector": {}}]}]))
    };
    let (_, _, px) = render(&plan_of(120.0), 0.0);
    let green = px.chunks_exact(4).filter(|p| p[1] > 200 && p[2] < 60 && p[0] < 60).count();
    let red = px.chunks_exact(4).filter(|p| p[2] > 200 && p[1] < 60).count();
    assert!(green > 1500 && red == 0, "green {green} red {red}");
    let (_, _, px) = render(&plan_of(0.0), 0.0);
    assert!(px.chunks_exact(4).filter(|p| p[2] > 200 && p[1] < 60).count() > 1500);
}

fn outline(extra: Value) -> Plan {
    let mut l = json!({"type": "text", "text": "I", "size": 300, "weight": 800, "color": "#ffffff", "pos": [320, 180], "align": "center"});
    for (k, v) in extra.as_object().unwrap() {
        l[k] = v.clone();
    }
    plan(json!([l]))
}

#[test]
fn outline_only_type_paints_the_stroke_and_not_the_fill() {
    let (_, _, filled) = render(&outline(json!({})), 0.0);
    let (_, _, ring) = render(&outline(json!({"outline_only": true})), 0.0);
    let (a, b) = (count_lit(&filled), count_lit(&ring));
    assert!(a > 3000, "{a}");
    assert!(b > 200 && (b as f64) < a as f64 * 0.25, "outline {b} vs filled {a}");
    // the middle of the stem is empty while its edges are inked
    assert!(!lit(&ring, 640, 320, 180), "stem centre is hollow");
}

#[test]
fn stroke_trim_reveals_the_outline_along_its_length_and_an_animator_widens_the_stroke() {
    let count = |a: f64, b: f64, each: bool| count_lit(&render(&outline(json!({"outline_only": true, "stroke_trim": [a, b], "stroke_trim_each": each})), 0.0).2);
    let full = count(0.0, 1.0, false);
    let half = count(0.0, 0.5, false);
    let none = count(0.0, 0.0, false);
    assert!(none < full / 20, "{none}");
    let ratio = half as f64 / full as f64;
    assert!((ratio - 0.5).abs() < 0.15, "half the outline is about half the ink: {ratio}");
    assert!(count(0.0, 0.5, true) > 0, "each glyph trims its own outline");
    // animating `end` writes the outline on over time
    let p = outline(json!({"outline_only": true, "stroke_trim": [0, {"from": 0, "to": 1, "dur": 2, "ease": "none"}]}));
    let (c1, c2) = (count_lit(&render(&p, 0.5).2), count_lit(&render(&p, 1.5).2));
    assert!(c2 > c1 * 2, "{c1} -> {c2}");
    // stroke_width animator
    let thin = count_lit(&render(&outline(json!({"outline_only": true})), 0.0).2);
    let wide = count_lit(&render(&outline(json!({"outline_only": true, "animators": [{"by": "chars", "props": {"stroke_width": 14}, "selector": {}}]})), 0.0).2);
    assert!(wide as f64 > thin as f64 * 3.0, "{thin} -> {wide}");
}

// ---- T4: marquee ---------------------------------------------------------------------------------------------------

/// The test compositions are this wide.
const FRAME: f64 = 640.0;
/// Gap between marquee copies in these tests, px.
const MARQUEE_GAP: f64 = 60.0;

fn marquee(speed: f64) -> Plan {
    plan(json!([{"type": "text", "text": "NEWS", "size": 40, "weight": 700, "color": "#fff", "pos": [0, 180], "align": "left", "marquee": {"speed": speed, "gap": MARQUEE_GAP}}]))
}

fn centres(t: &crate::eval::text::TextState) -> Vec<f64> {
    let mut v: Vec<f64> = (0..t.glyphs.len()).map(|i| baseline_centre(t, i).0[0]).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v
}

/// Do two rows of glyph centres show the same picture once `b` is moved by `dx`, looking only at the frame?
fn same_picture(a: &[f64], b: &[f64], dx: f64, frame: f64) -> bool {
    let on_screen = |v: &[f64], d: f64| v.iter().map(|x| x + d).filter(|x| (0.0..=frame).contains(x)).collect::<Vec<_>>();
    let (a, b) = (on_screen(a, 0.0), on_screen(b, dx));
    a.len() == b.len() && a.iter().zip(&b).all(|(p, q)| (p - q).abs() < 0.01)
}

#[test]
fn a_marquee_repeats_seamlessly_and_covers_the_frame_at_every_instant() {
    let p = marquee(-100.0);
    let t0 = text_at(&p, 0.0);
    let n = t0.layout.as_ref().unwrap().glyphs.len();
    let period = f64::from(t0.width) + MARQUEE_GAP;
    assert!(t0.glyphs.len() > n * 3, "{} glyphs for {n}", t0.glyphs.len());
    // one period of travel later the picture is the same
    let a = centres(&t0);
    assert!(same_picture(&a, &centres(&text_at(&p, period / 100.0)), 0.0, FRAME));
    // it moves at the speed asked: a tenth of a second at 100 px/s is 10 px to the left
    assert!(same_picture(&a, &centres(&text_at(&p, 0.1)), 10.0, FRAME));
    // and a positive speed moves the row right
    let right = marquee(100.0);
    assert!(same_picture(&centres(&text_at(&right, 0.0)), &centres(&text_at(&right, 0.1)), -10.0, FRAME));
    // no gap in coverage: at any time the first copy begins at or before 0 and the last ends past the frame
    for t in [0.0, 0.37, 1.1, 2.9] {
        let b = text_at(&p, t).placed_bounds.expect("marquee bounds");
        assert!(b[0] <= 1.0 && b[2] >= FRAME, "t={t}: row spans {b:?}");
    }
    // `repeat` pins the number of copies
    let q = plan(json!([{"type": "text", "text": "NEWS", "size": 40, "color": "#fff", "pos": [0, 180], "marquee": {"speed": -50, "repeat": 2}}]));
    assert_eq!(text_at(&q, 0.0).glyphs.len(), 2 * n);
    // and it paints across the whole frame
    let (w, _, px) = render(&p, 0.3);
    let lit_columns = (0..w).filter(|x| (150..210).any(|y| lit(&px, w, *x, y))).count();
    assert!(lit_columns > 150, "{lit_columns} columns of type across a {FRAME} px frame");
}

// ---- T5: cipher ----------------------------------------------------------------------------------------------------

fn cipher(extra: Value, text: &str) -> Plan {
    let mut c = json!({"charset": "auto", "rate": 24, "seed": 7});
    for (k, v) in extra.as_object().unwrap() {
        c[k] = v.clone();
    }
    plan(json!([{"type": "text", "text": text, "size": 50, "weight": 700, "color": "#fff", "pos": [320, 180], "align": "center",
        "animators": [{"by": "chars", "at": 0, "dur": 0.01, "ease": "none", "stagger": {"each": 0.1}, "cipher": c}]}]))
}

#[test]
fn a_cipher_decodes_each_character_at_its_own_time_and_is_deterministic_by_seed() {
    let word = "DECODE42";
    let p = cipher(json!({}), word);
    let real: Vec<char> = word.chars().collect();
    // t=0.35: the first characters have locked on (their clocks ran out), the rest are still noise
    let t = text_at(&p, 0.35);
    let shown: Vec<char> = t.glyphs.iter().map(|g| g.ch).collect();
    assert_eq!(&shown[..3], &real[..3], "settled: {shown:?}");
    let noisy = (4..8).filter(|i| shown[*i] != real[*i]).count();
    assert!(noisy >= 2, "later characters still scramble: {shown:?}");
    // auto keeps the class: capitals stay capitals and digits stay digits
    for i in 3..8 {
        assert_eq!(shown[i].is_ascii_digit(), real[i].is_ascii_digit(), "{shown:?}");
        assert!(shown[i].is_ascii_uppercase() || shown[i].is_ascii_digit(), "{shown:?}");
    }
    // all settled at the end
    let end: Vec<char> = text_at(&p, 1.5).glyphs.iter().map(|g| g.ch).collect();
    assert_eq!(end, real);
    // deterministic: the same frame twice, and a different seed gives different noise
    let again: Vec<char> = text_at(&cipher(json!({}), word), 0.35).glyphs.iter().map(|g| g.ch).collect();
    assert_eq!(shown, again);
    let other: Vec<char> = text_at(&cipher(json!({"seed": 99}), word), 0.35).glyphs.iter().map(|g| g.ch).collect();
    assert_ne!(shown, other);
    // the noise changes with time (rate 24: a tick is 1/24 s)
    let later: Vec<char> = text_at(&p, 0.35 + 3.0 / 24.0).glyphs.iter().map(|g| g.ch).collect();
    assert_ne!(shown[4..], later[4..]);
    // a literal charset and a binary charset are respected
    let bin: Vec<char> = text_at(&cipher(json!({"charset": "binary"}), word), 0.0).glyphs.iter().map(|g| g.ch).collect();
    assert!(bin.iter().all(|c| *c == '0' || *c == '1'), "{bin:?}");
    let lit: Vec<char> = text_at(&cipher(json!({"charset": "xy"}), word), 0.0).glyphs.iter().map(|g| g.ch).collect();
    assert!(lit.iter().all(|c| *c == 'x' || *c == 'y'), "{lit:?}");
}

#[test]
fn a_selector_drives_the_cipher_and_settle_locks_on_early() {
    // AE style: a range selector sweeping across the text decodes the characters it has passed
    let word = "ABCDEFGH";
    let sel = |offset: f64| {
        plan(json!([{"type": "text", "text": word, "size": 50, "color": "#fff", "pos": [320, 180], "align": "center",
            "animators": [{"by": "chars", "selector": {"start": 0, "end": 1, "offset": offset, "shape": "ramp_down"}, "cipher": {"settle": 0.5, "seed": 3}}]}]))
    };
    // weight is above 0.5 only in the first half of the ramp window
    let shown: Vec<char> = text_at(&sel(0.0), 0.0).glyphs.iter().map(|g| g.ch).collect();
    let real: Vec<char> = word.chars().collect();
    assert_ne!(shown[0], real[0]);
    assert_eq!(&shown[4..], &real[4..], "settled past the threshold: {shown:?}");
}

#[test]
fn a_wiggly_selector_gives_each_character_its_own_seeded_amount() {
    let word = "ABCDEFGH";
    let wiggle = |w: Value| {
        plan(json!([{"type": "text", "text": word, "size": 50, "color": "#fff", "pos": [320, 180], "align": "center",
            "animators": [{"by": "chars", "props": {"rotation": 30}, "selector": {"start": 0, "end": 1, "wiggle": w}}]}]))
    };
    let turns = |p: &Plan, t: f64| -> Vec<f64> { let s = text_at(p, t); (0..word.len()).map(|i| baseline_centre(&s, i).1).collect() };
    // signed range: every turn lies in min..=max degrees * 30 and they are not all alike
    let signed = turns(&wiggle(json!({"min": -1, "max": 1, "seed": 4})), 0.0);
    assert!(signed.iter().all(|a| a.abs() <= 30.0 + 1e-3), "{signed:?}");
    assert!(signed.iter().any(|a| *a > 1.0) && signed.iter().any(|a| *a < -1.0), "both directions: {signed:?}");
    // the same seed repeats, another seed differs, and a fixed wiggle (rate 0) does not move over time
    assert_eq!(signed, turns(&wiggle(json!({"min": -1, "max": 1, "seed": 4})), 0.0));
    assert_ne!(signed, turns(&wiggle(json!({"min": -1, "max": 1, "seed": 5})), 0.0));
    assert_eq!(signed, turns(&wiggle(json!({"min": -1, "max": 1, "seed": 4})), 0.7));
    // a drifting wiggle (rate > 0) changes over time
    let drift = wiggle(json!({"min": -1, "max": 1, "seed": 4, "rate": 2}));
    assert_ne!(turns(&drift, 0.0), turns(&drift, 0.7));
    // without a wiggle every unit gets the same full amount
    let flat = turns(&wiggle(json!(null)), 0.0);
    assert!(flat.iter().all(|a| (a - 30.0).abs() < 1e-3), "{flat:?}");
}

// ---- T6: surfaces --------------------------------------------------------------------------------------------------

fn drum(kind: &str, extra: Value, text: &str) -> Plan {
    let mut s = json!({"kind": kind, "radius": 150});
    for (k, v) in extra.as_object().unwrap() {
        s[k] = v.clone();
    }
    plan(json!([{"type": "text", "text": text, "size": 40, "weight": 700, "color": "#fff", "pos": [320, 180], "align": "center", "perspective": "orthographic", "surface": s}]))
}

#[test]
fn a_cylinder_stands_each_glyph_on_the_surface_turned_to_follow_it() {
    let t = text_at(&drum("cylinder", json!({}), "ABCDEF"), 0.0);
    let layout = t.layout.as_ref().unwrap();
    for (i, g) in t.glyphs.iter().enumerate() {
        let lg = &layout.glyphs[g.src];
        let (cx, cy) = (lg.x + lg.w / 2.0, (lg.top + lg.bottom) / 2.0);
        let phi = f64::from(cx - layout.anchor[0]) / 150.0;
        let m = crate::eval::mat4::Mat4(g.m4.expect("a 4x4"));
        let p = m.apply(f64::from(cx), f64::from(cy), 0.0);
        assert!((p[0] - (f64::from(layout.anchor[0]) + 150.0 * phi.sin())).abs() < 1e-3, "glyph {i} x {}", p[0]);
        assert!((p[1] - f64::from(cy)).abs() < 1e-6 && (p[2] - 150.0 * (1.0 - phi.cos())).abs() < 1e-3);
        // the glyph's width on screen is its width times cos(phi)
        let w = width_of(&t, i);
        assert!((w / f64::from(lg.w) - phi.cos()).abs() < 1e-3, "glyph {i}: {w}/{}", lg.w);
    }
    // turning the drum half way hides everything that now faces away
    let hidden = text_at(&drum("cylinder", json!({"turn": 180}), "ABCDEF"), 0.0);
    assert!(hidden.glyphs.iter().all(|g| g.opacity < 0.01));
    let seen = text_at(&drum("cylinder", json!({"turn": 180, "back": true}), "ABCDEF"), 0.0);
    assert!(seen.glyphs.iter().all(|g| g.opacity > 0.99));
    // turn carries text right: the first glyph moves
    let a = baseline_x(&text_at(&drum("cylinder", json!({}), "ABCDEF"), 0.0), 0);
    let b = baseline_x(&text_at(&drum("cylinder", json!({"turn": 20}), "ABCDEF"), 0.0), 0);
    assert!(b > a + 20.0, "{a} -> {b}");
}

fn baseline_x(t: &crate::eval::text::TextState, i: usize) -> f64 {
    let lg = &t.layout.as_ref().unwrap().glyphs[t.glyphs[i].src];
    let m = crate::eval::mat4::Mat4(t.glyphs[i].m4.unwrap());
    m.apply(f64::from(lg.x + lg.w / 2.0), f64::from((lg.top + lg.bottom) / 2.0), 0.0)[0]
}

#[test]
fn a_sphere_also_curves_the_lines_toward_the_poles_and_a_surface_renders() {
    let t = text_at(&drum("sphere", json!({"radius": 120}), "AAA\nBBB\nCCC"), 0.0);
    let layout = t.layout.as_ref().unwrap();
    let top = &layout.glyphs[0];
    let lat = f64::from((top.top + top.bottom) / 2.0 - layout.anchor[1]) / 120.0;
    let m = crate::eval::mat4::Mat4(t.glyphs[0].m4.unwrap());
    let c = (f64::from(top.x + top.w / 2.0), f64::from((top.top + top.bottom) / 2.0));
    let p = m.apply(c.0, c.1, 0.0);
    let phi = (c.0 - f64::from(layout.anchor[0])) / 120.0;
    assert!((p[1] - (f64::from(layout.anchor[1]) + 120.0 * lat.sin())).abs() < 1e-3, "y {}", p[1]);
    assert!((p[0] - (f64::from(layout.anchor[0]) + 120.0 * lat.cos() * phi.sin())).abs() < 1e-3);
    assert!((p[2] - 120.0 * (1.0 - lat.cos() * phi.cos())).abs() < 1e-3);
    // a wrapped line is narrower on screen than the same line flat
    let flat = plan(json!([{"type": "text", "text": "ABCDEFGHIJ", "size": 40, "weight": 700, "color": "#fff", "pos": [320, 180], "align": "center"}]));
    let (w, _, a) = render(&flat, 0.0);
    let (_, _, b) = render(&drum("cylinder", json!({"radius": 80}), "ABCDEFGHIJ"), 0.0);
    let (a0, a1) = lit_x_range(&a, w);
    let (b0, b1) = lit_x_range(&b, w);
    assert!(b1 - b0 < a1 - a0, "wrapped {} px vs flat {} px", b1 - b0, a1 - a0);
}

// ---- invalid input fails loudly when the composition is read -------------------------------------------------------

#[test]
fn invalid_text_options_are_refused_with_a_clear_message() {
    let refused = |layer: Value| -> String {
        let mut l = json!({"type": "text", "text": "x"});
        for (k, v) in layer.as_object().unwrap() {
            l[k] = v.clone();
        }
        format!("{:#}", Plan::from_value(json!({"dur": 1, "layers": [l]}), std::path::Path::new(".")).err().expect("refused"))
    };
    assert!(refused(json!({"path": {"ellipse": [0, 0, 9, 9]}, "surface": {}})).contains("`path` or a `surface`"));
    assert!(refused(json!({"marquee": {"gap": -5}})).contains("marquee gap"));
    assert!(refused(json!({"marquee": {"repeat": 0}})).contains("nonzero"));
    assert!(refused(json!({"stroke_trim": [0, 1]})).contains("stroke_trim"));
    assert!(refused(json!({"perspective": 0})).contains("perspective"));
    assert!(refused(json!({"perspective": "wide"})).contains("perspective"));
    assert!(refused(json!({"animators": [{"cipher": {"rate": 0}}]})).contains("cipher rate"));
    assert!(refused(json!({"animators": [{"cipher": {"settle": 1}}]})).contains("cipher settle"));
    assert!(refused(json!({"animators": [{"cipher": {"charset": ""}}]})).contains("charset"));
    assert!(refused(json!({"animators": [{"selector": {"shape": "zigzag"}}]})).contains("zigzag"));
    assert!(refused(json!({"surface": {"kind": "cube"}})).contains("cube"));
}

#[test]
fn a_surface_radius_must_be_positive() {
    let p = plan(json!([{"type": "text", "text": "x", "surface": {"radius": 0}}]));
    assert!(Evaluator::new(&p).frame(ROOT, 0.0).is_err());
}
