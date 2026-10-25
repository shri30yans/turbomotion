use std::collections::HashMap;

use serde_json::json;

use super::{glyphs, primitives, Mesh};
use crate::{eval::Evaluator, plan::{Plan, ROOT}, text::layout::{Engine, Spec}, schema::Align};

fn mesh(text: &str, depth: f32, bevel: f32) -> Mesh {
    let plan = Plan::from_value(json!({"dur": 1, "layers": []}), std::path::Path::new(".")).unwrap();
    let spec = Spec::new(text, Some("Helvetica Neue"), Some(700), false, 200.0, 0.0, None, Align::Left, None);
    let layout = Engine::new().layout(&plan, &spec).unwrap();
    Mesh::build(&glyphs::outlines(&layout), depth, bevel)
}

fn key(p: [f32; 3]) -> [i32; 3] {
    p.map(|v| (v * 100.0).round() as i32)
}

/// Every undirected edge must be walked once each way: no holes, no flipped triangles.
fn assert_closed(m: &Mesh) {
    let mut edges: HashMap<([i32; 3], [i32; 3]), i32> = HashMap::new();
    for t in &m.tris {
        let p = t.map(|i| key(m.verts[i as usize].p));
        for k in 0..3 {
            *edges.entry((p[k], p[(k + 1) % 3])).or_default() += 1;
        }
    }
    let bad: Vec<_> = edges.iter().filter(|((a, b), n)| **n != 1 || edges.get(&(*b, *a)) != Some(&1)).take(3).collect();
    assert!(bad.is_empty(), "open or inconsistent edges: {bad:?}");
}

/// Every triangle's winding agrees with its vertices' normals, and the enclosed volume is positive (normals point out).
fn assert_outward(m: &Mesh) {
    let mut volume = 0.0f64;
    for t in &m.tris {
        let [a, b, c] = t.map(|i| m.verts[i as usize]);
        let (u, w) = ([b.p[0] - a.p[0], b.p[1] - a.p[1], b.p[2] - a.p[2]], [c.p[0] - a.p[0], c.p[1] - a.p[1], c.p[2] - a.p[2]]);
        let g = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
        let n = [a.n[0] + b.n[0] + c.n[0], a.n[1] + b.n[1] + c.n[1], a.n[2] + b.n[2] + c.n[2]];
        assert!(g[0] * n[0] + g[1] * n[1] + g[2] * n[2] > 0.0, "triangle faces against its normals");
        volume += f64::from(a.p[0] * g[0] + a.p[1] * g[1] + a.p[2] * g[2]) / 6.0;
    }
    assert!(volume > 0.0, "signed volume {volume}");
}

#[test]
fn o_is_a_closed_outward_solid_with_a_hole() {
    let m = mesh("O", 40.0, 0.0);
    assert_closed(&m);
    assert_outward(&m);
    // the counter is open: no cap triangle covers the middle of the O
    let covered = m.tris.iter().any(|t| {
        let [a, b, c] = t.map(|i| m.verts[i as usize]);
        a.p[2] < -19.0 && b.p[2] < -19.0 && c.p[2] < -19.0 && (a.p[0] + b.p[0] + c.p[0]).abs() < 3.0 && (a.p[1] + b.p[1] + c.p[1]).abs() < 3.0
    });
    assert!(!covered, "the hole of the O is filled");
    assert!((m.max[2] - m.min[2] - 40.0).abs() < 0.01);
}

#[test]
fn m_is_closed_and_centred() {
    let m = mesh("M", 40.0, 0.0);
    assert_closed(&m);
    assert_outward(&m);
    assert!((m.min[0] + m.max[0]).abs() < 0.01 && (m.min[1] + m.max[1]).abs() < 0.01, "not centred: {:?} {:?}", m.min, m.max);
}

#[test]
fn a_bevel_adds_triangles_and_stays_closed() {
    let plain = mesh("O", 40.0, 0.0);
    let bevelled = mesh("O", 40.0, 5.0);
    assert!(bevelled.tris.len() > plain.tris.len());
    assert_closed(&bevelled);
    assert_outward(&bevelled);
}

#[test]
fn a_mesh_layer_has_bounds_and_renders_pixels() {
    let plan = Plan::from_value(
        json!({"width": 640, "height": 360, "dur": 1, "bg": "#000", "layers": [
            {"type": "camera", "zoom": 800},
            {"type": "light", "kind": "ambient", "intensity": 0.8},
            {"type": "mesh", "id": "m", "three_d": true, "geometry": {"kind": "text", "text": "M", "size": 200, "bevel": 4}, "color": "#ff4030", "rot_y": 30, "pos": [320, 180, 0]}
        ]}),
        std::path::Path::new("."),
    )
    .unwrap();
    let f = Evaluator::new(&plan).frame(ROOT, 0.0).unwrap();
    let b = f.layers[2].bounds.expect("bounds");
    assert!(b[0] > 100.0 && b[2] < 540.0 && b[3] - b[1] > 100.0, "{b:?}");
    let mut r = crate::render::Renderer::new(&plan).unwrap();
    r.draw(0).unwrap();
    let mut px = vec![];
    r.pixels(&mut px).unwrap();
    let red = px.chunks_exact(4).filter(|p| p[2] > 120 && p[1] < 100).count();
    assert!(red > 2000, "only {red} red pixels");
}

#[test]
fn every_solid_primitive_is_closed_and_outward() {
    let solids = [
        ("box", primitives::cuboid(100.0, 60.0, 40.0)),
        ("sphere", primitives::sphere(100.0, 24)),
        ("torus", primitives::torus(100.0, 25.0, 24)),
        ("cylinder", primitives::frustum(60.0, 60.0, 100.0, 24)),
        ("cone", primitives::frustum(60.0, 0.0, 100.0, 24)),
    ];
    for (name, m) in solids {
        assert!(!m.tris.is_empty(), "{name} is empty");
        assert_closed(&m);
        assert_outward(&m);
    }
}

#[test]
fn an_extruded_path_is_closed_outward_and_as_deep_as_asked() {
    let path = skia_safe::Path::from_svg("M0 0H10V10H0Z").expect("a square");
    let m = primitives::extruded(&path, 100.0, 30.0, 0.0).expect("mesh");
    assert_closed(&m);
    assert_outward(&m);
    assert!((m.max[2] - m.min[2] - 30.0).abs() < 0.01);
}
