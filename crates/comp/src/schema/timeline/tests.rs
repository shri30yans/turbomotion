use super::*;
use serde_json::json;

fn run(v: Value) -> Value {
    let mut v = v;
    expand(&mut v).unwrap();
    v
}

#[test]
fn shots_overlap_by_their_transition_and_pair_enter_with_exit() {
    let v = run(json!({"width": 1920, "height": 1080, "fps": 30, "timeline": {
        "transition": {"type": "push_left", "dur": 0.5},
        "shots": [
            {"id": "a", "dur": 3, "layers": [{"type": "solid", "color": "#f00"}]},
            {"id": "b", "dur": 2, "layers": [{"type": "solid", "color": "#0f0"}], "in": "pan_out_up"},
            {"id": "c", "dur": 2, "layers": [{"type": "solid", "color": "#00f"}], "in": "cut"}
        ]}}));
    let ls = v["layers"].as_array().unwrap();
    let shots: Vec<&Value> = ls.iter().filter(|l| l["type"] == "precomp").collect();
    assert_eq!(shots[1]["start"], 2.5);
    assert_eq!(shots[0]["exit"]["preset"], "pan_out_up");
    assert_eq!(shots[1]["enter"]["preset"], "pan_out_up");
    assert_eq!(shots[1]["enter"]["dist"], 1080.0);
    assert_eq!(shots[2]["start"], 4.5);
    assert!(shots[1].get("exit").is_none() && shots[2].get("enter").is_none());
    assert_eq!(v["dur"], 6.5);
    assert!(ls.iter().any(|l| l["src"] == "kit:sweep"));
    assert_eq!(v["markers"][1]["name"], "b");
}

#[test]
fn locked_starts_extend_the_previous_shot() {
    let v = run(json!({"timeline": {"shots": [
        {"id": "a", "dur": 1, "layers": []},
        {"id": "b", "start": 4, "dur": 2, "layers": [], "in": {"type": "dissolve", "dur": 0.4}}]}}));
    assert_eq!(v["layers"][0]["dur"], 4.4);
    assert_eq!(v["layers"][1]["start"], 4.0);
}

#[test]
fn a_speed_ramp_integrates_to_a_remap() {
    let v = run(json!({"fps": 10, "timeline": {"shots": [{"id": "a", "dur": 2, "layers": [], "speed": [[0, 1, "none"], [1, 1, "none"], [1.0001, 0.5, "none"], [2, 0.5]]}]}}));
    let keys = v["layers"][0]["remap"].as_array().unwrap();
    let last = keys.last().unwrap();
    assert_eq!(last[0], 2.0);
    // a step in speed costs half a sub-frame of error (1/8 frame), well under one frame
    assert!((last[1].as_f64().unwrap() - 1.5).abs() < 1e-2, "{last}");
    assert!(v["comps"]["shot:a"]["dur"].as_f64().unwrap() > 1.5);
}

#[test]
fn mistakes_name_the_choices() {
    let mut v = json!({"timeline": {"shots": [{"id": "a", "dur": 1, "layers": []}, {"dur": 1, "layers": [], "in": "spiral"}]}});
    assert!(expand(&mut v).unwrap_err().chain().any(|e| e.to_string().contains("pan_out_left")));
    let mut v = json!({"timeline": {"shots": [{"id": "a", "dur": 1, "layers": [], "move": "zoomzoom"}]}});
    assert!(format!("{:#}", expand(&mut v).unwrap_err()).contains("push_in"));
}

#[test]
fn flood_disc_is_centred_on_the_frame() {
    let v = run(json!({"width": 1920, "height": 1080, "fps": 30, "timeline": {"shots": [
        {"id": "a", "dur": 2, "layers": [{"type": "solid", "color": "#f00"}]},
        {"id": "b", "dur": 2, "layers": [{"type": "solid", "color": "#0f0"}], "in": {"type": "flood", "dur": 0.8}}]}}));
    let disc = v["layers"].as_array().unwrap().iter().find(|l| l["type"] == "shape").unwrap();
    let e = &disc["items"][0]["ellipse"];
    // [cx, cy, rx, ry]: centred on the layer origin and big enough to cover the corners
    assert_eq!((e[0].as_f64(), e[1].as_f64()), (Some(0.0), Some(0.0)));
    assert!(e[2].as_f64().unwrap() > 1100.0 && e[2] == e[3]);
}
