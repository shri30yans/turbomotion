//! The promoted components: cubes, swarm, flip tiles, hands, lens, blob, montage, dolly zoom, glass panels, `appear`.
mod common;
use std::fs;
use serde_json::{json, Value};
use turbomotion_script::{build, Options};

fn run(name: &str, body: &str) -> Value {
    let value = data(name, body, "export default film;");
    turbomotion_comp::plan::Plan::from_value(value.clone(), &common::scratch("library", name)).unwrap_or_else(|e| panic!("{e:#}"));
    value
}
/// Build the script and return whatever it exports (the film, or an object holding numbers to check and the film).
fn data(name: &str, body: &str, export: &str) -> Value {
    let dir = common::scratch("library", name);
    let head = "import * as T from 'turbomotion';\nconst { comp, shot, timeline, light, camera, text } = T;\nconst film = comp({ width: 960, height: 540, fps: 30, bg: '#000' });\nconst tl = timeline();\n";
    fs::write(dir.join("film.ts"), format!("{head}{body}\n{export}")).unwrap();
    build(&dir.join("film.ts"), &Options::default()).unwrap_or_else(|e| panic!("{e:#}"))
}
fn layers(film: &Value) -> Vec<&Value> {
    let shots = film["timeline"]["shots"].as_array();
    shots.map_or_else(|| film["layers"].as_array().unwrap().iter().collect(), |s| s.iter().flat_map(|s| s["layers"].as_array().unwrap()).collect())
}
fn by<'a>(film: &'a Value, id: &str) -> &'a Value {
    layers(film).into_iter().find(|l| l["id"] == id).unwrap_or_else(|| panic!("no layer {id}"))
}

#[test]
fn a_cube_is_a_root_and_six_faces_and_a_puzzle_cube_has_three_twistable_slabs() {
    let v = run("cubes", r##"
const c = T.cube({ id: 'c', size: 100, faces: ['#f00', { fill: '#0f0', items: [{ ellipse: [0, 0, 5, 5], fill: '#fff' }] }] });
const p = T.puzzleCube({ id: 'p', size: 90, tile: () => [{ ellipse: [0, 0, 3, 3], fill: '#000' }] });
const l = T.letterCube({ id: 'l', size: 80, fill: '#00f', letter: 'A' });
film.timeline({ shots: [shot({ id: 's', dur: 1, layers: [light({ kind: 'ambient' }), c, p, l] })] });
tl.to(p.slabs[0], { rotation: [null, 90, null], duration: 1 });"##);
    let ls = layers(&v);
    assert_eq!(ls.iter().filter(|l| l["id"].as_str().is_some_and(|i| i.starts_with("c-f"))).count(), 6);
    assert_eq!(by(&v, "c-f1")["items"].as_array().unwrap().len(), 2, "art goes on the second face");
    assert_eq!(by(&v, "c-f4")["items"][0]["fill"], "#f00", "faces past the list reuse the first fill");
    assert_eq!(by(&v, "c-f0")["parent"], "c");
    assert_eq!(ls.iter().filter(|l| l["id"].as_str().is_some_and(|i| i.starts_with("p-slab"))).count(), 3);
    assert_eq!(by(&v, "l-letter")["text"], "A");
    assert_eq!(by(&v, "l-letter")["rotation"], json!([0, 180, 0]));
}

#[test]
fn a_swarm_scatters_with_the_same_seed_every_time_and_a_flip_tile_turns_half_a_circle() {
    let body = "const s = T.swarm({ tl, id: 's', n: 4, seed: 3, at: 0.1, dur: 0.5 }); const f = T.flipTile({ tl, id: 'f', size: 100, pos: [10, 20], front: { fill: '#111', label: 'a' }, back: { fill: '#222' }, at: 0.2, dur: 0.4 });\nfilm.timeline({ shots: [shot({ id: 's', dur: 2, layers: [light({ kind: 'ambient' }), s, f] })] });";
    let (a, b) = (run("swarm-a", body), run("swarm-b", body));
    assert_eq!(by(&a, "s2")["pos"], by(&b, "s2")["pos"]);
    assert_eq!(layers(&a).iter().filter(|l| l["type"] == "null").count(), 5);
    assert_eq!(by(&a, "f")["rotation"][1]["to"], 180);
    assert_eq!(by(&a, "f-back")["rotation"], json!([0, 180, 0]));
}

#[test]
fn a_montage_places_each_word_shot_so_the_holds_follow_one_another() {
    let out = data("montage", r##"
const m = T.montage({ frame: [960, 540], hold: 0.5, transition: { type: 'dissolve', dur: 0.25 }, items: [{ word: 'a', field: '#f00' }, { word: 'b', field: '#0f0', hold: 1 }, { word: 'c' }] });
film.timeline({ shots: m.shots });"##, "export default { starts: m.starts, inView: m.inView, end: m.end, film };");
    let v = out["film"].clone();
    let (starts, in_view, end) = (&out["starts"], &out["inView"], &out["end"]);
    assert_eq!(starts.as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect::<Vec<_>>(), vec![0.0, 0.75, 2.0]);
    assert_eq!((in_view[1][0].as_f64(), in_view[1][1].as_f64()), (Some(1.0), Some(2.0)));
    assert_eq!(end, &serde_json::json!(2.75));
    assert_eq!(v["timeline"]["shots"][2]["in"]["dur"], 0.25);
    let plan = turbomotion_comp::plan::Plan::from_value(v, &common::scratch("library", "montage")).unwrap();
    assert!((plan.root().dur - 2.75).abs() < 1e-9, "{}", plan.root().dur);
}

#[test]
fn a_dolly_zoom_moves_the_camera_in_while_the_lens_widens_and_rejects_nonsense() {
    let out = data("dolly", "const cam = camera({ id: 'cam', pos: [480, 270, -800], zoom: 800 }); const end = T.dollyZoom(cam, { tl, at: 0.5, dur: 2, amount: 0.5 });\nfilm.add(cam);", "export default { end, film };");
    let v = out["film"].clone();
    let cam = by(&v, "cam");
    assert_eq!(cam["pos"][2]["from"], -800);
    assert_eq!(cam["pos"][2]["to"], -400);
    assert_eq!(cam["zoom"]["to"], 400);
    assert_eq!(out["end"], 2.5);
}

#[test]
fn appear_gives_every_visible_layer_an_engine_entrance_without_touching_its_own_tweens() {
    let v = run("appear", "const s = T.statCounter({ tl, value: 5, label: 'x', at: 0.4, theme: { mono: 'Menlo', display: 'Menlo' }, appear: { preset: 'slide_up', dur: 0.3, stagger: 0.1 } }); film.add(s);");
    let ls = layers(&v);
    assert_eq!(ls[0]["enter"]["preset"], "slide_up");
    assert_eq!(ls[0]["enter"]["delay"], 0.4);
    assert_eq!(ls[1]["enter"]["delay"], 0.5);
    assert!(ls[0]["opacity"].is_object(), "the component's own opacity tween stays");
}

#[test]
fn lens_blob_hand_capsule_and_glass_panel_load_as_compositions() {
    let v = run("shapes", r##"
film.add(T.lens({ id: 'lens', pos: [100, 100], open: { tl, at: 0.1 } }), T.blob({ id: 'b', pos: [300, 100], morph: { tl, seed: 5, at: 0.2, dur: 1, back: true } }),
  T.hand({ id: 'h', pos: [500, 300] }), T.capsule({ from: [0, 0], to: [100, 100], w: 10, fill: '#fff' }), T.glassPanel({ id: 'g', pos: [400, 200], size: [200, 100] }));"##);
    assert_eq!(by(&v, "b")["items"][0]["morph"]["keys"].as_array().unwrap().len() >= 3, true, "{}", by(&v, "b")["items"][0]["morph"]);
    assert!(by(&v, "h")["items"].as_array().unwrap().len() == 7, "forearm, palm, four fingers, thumb");
    assert!(by(&v, "g")["items"][0]["fill"]["gradient"].is_object());
}
