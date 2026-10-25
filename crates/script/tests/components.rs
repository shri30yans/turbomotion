//! Sounds on the timeline (hits, `sfx`, audio layers) and the reusable components in `sdk/components/`.

use std::{fs, path::{Path, PathBuf}};

use serde_json::Value;
mod common;

use turbomotion_script::{build, Options};

fn write(name: &str, src: &str) -> PathBuf {
    let d = common::scratch("component-tests", name);
    fs::write(d.join("film.ts"), src).unwrap();
    d.join("film.ts")
}

fn run(name: &str, src: &str) -> Value {
    build(&write(name, src), &Options::default()).unwrap_or_else(|e| panic!("{e:#}"))
}

const HEAD: &str = "import { comp, shot, timeline, text, audio, hit, promptBox, codePanel, browserWindow, statCounter, barRace, bboxOverlay, cardWall, title3d } from \"turbomotion\";\nconst film = comp({ width: 1280, height: 720, fps: 30, fonts: { \"JetBrains Mono\": \"../../kit/fonts/JetBrainsMono-Regular.woff\" } });\n";

fn hits(v: &Value) -> Vec<(String, String)> {
    v["timeline"]["hits"].as_array().unwrap().iter().map(|h| (h["at"].as_str().map_or_else(|| h["at"].to_string(), String::from), h["cue"].as_str().unwrap().to_string())).collect()
}

#[test]
fn sfx_on_a_tween_emits_one_hit_per_staggered_element_in_shot_time() {
    let src = format!("{HEAD}const ls = [text(\"a\"), text(\"b\"), text(\"c\")];\nconst tl = timeline();\ntl.from(ls, {{ opacity: 0, duration: 0.4, stagger: 0.1, sfx: \"pop\" }}, 0.5);\ntl.hit(2, \"boom\", 0.8);\nfilm.timeline({{ shots: [shot({{ id: \"one\", dur: 3, layers: [ls] }}), shot({{ id: \"two\", dur: 3, start: 2.7, layers: [ls.map((l) => text(\"x\" + l.id))], hits: tl.hits() }})], hits: [hit(0.25, \"tick\")] }});\nexport default film;\n");
    let v = run("sfx", &src);
    let h = hits(&v);
    assert!(h.contains(&("two.start+0.5".into(), "pop".into())), "{h:?}");
    assert!(h.contains(&("two.start+0.6".into(), "pop".into())));
    assert!(h.contains(&("two.start+0.7".into(), "pop".into())));
    assert!(h.contains(&("two.start+2".into(), "boom".into())));
    assert!(h.contains(&("0.25".into(), "tick".into())), "film-time hits pass through: {h:?}");
    let boom = v["timeline"]["hits"].as_array().unwrap().iter().find(|x| x["cue"] == "boom").unwrap();
    assert_eq!(boom["volume"], 0.8);
    // hits live on the timeline, not inside the shot the engine receives
    assert!(v["timeline"]["shots"][1].get("hits").is_none());
}

#[test]
fn audio_is_a_layer_and_shot_hits_become_start_literals() {
    let src = format!("{HEAD}film.add(audio(\"kit:pop\", {{ id: \"music\", volume: 0.8, fade_out: 1 }}));\nconst tl = timeline();\ntl.hit(0.5, \"click\");\nfilm.timeline({{ shots: [shot({{ id: \"a\", dur: 2, layers: [text(\"x\")], hits: tl.hits() }})] }});\nexport default film;\n");
    let v = run("audio", &src);
    let music = v["layers"].as_array().unwrap().iter().find(|l| l["id"] == "music").unwrap();
    assert_eq!((music["type"].as_str(), music["src"].as_str(), music["fade_out"].as_f64()), (Some("audio"), Some("kit:pop"), Some(1.0)));
    // the engine resolves "a.start+0.5" itself (it alone knows where a shot really starts once transitions overlap)
    assert_eq!(hits(&v), [("a.start+0.5".to_string(), "click".to_string())]);
    turbomotion_comp::plan::Plan::from_value(v, &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../launch")).unwrap_or_else(|e| panic!("{e:#}"));
}

#[test]
fn prompt_box_types_a_hit_per_visible_character_and_steps_the_caret() {
    let src = format!("{HEAD}const tl = timeline();\nconst p = promptBox({{ tl, text: \"go now\", at: 1, cps: 10, enterAt: 2 }});\nfilm.timeline({{ shots: [shot({{ id: \"p\", dur: 3, layers: [p], hits: tl.hits() }})] }});\nexport default film;\n");
    let v = run("prompt", &src);
    let types = hits(&v).into_iter().filter(|(_, c)| c == "type").count();
    assert_eq!(types, 5, "five visible characters; the space takes no keystroke");
    assert!(hits(&v).iter().any(|(at, c)| c == "click" && at == "p.start+2"));
    let layers = v["timeline"]["shots"][0]["layers"].as_array().unwrap();
    let ids: Vec<&str> = layers.iter().map(|l| l["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["prompt", "prompt-card", "prompt-rim", "prompt-glyph", "prompt-text", "prompt-caret", "prompt-dot"]);
    let text = layers.iter().find(|l| l["id"] == "prompt-text").unwrap();
    assert_eq!(text["preset"]["name"], "typewriter");
    assert_eq!(text["parent"], "prompt");
    let caret_x = layers.iter().find(|l| l["id"] == "prompt-caret").unwrap()["pos"][0]["keys"].as_array().unwrap().len();
    assert!(caret_x >= 5 * 2, "the caret holds and jumps once per character, got {caret_x} keys");
}

#[test]
fn code_panel_colours_tokens_with_runs_and_lights_one_line_at_a_time() {
    let src = format!("{HEAD}const tl = timeline();\nconst c = codePanel({{ tl, at: 0, perLine: 1, lines: [\"const x = 3; // go\", \"const s = \\\"hi\\\";\"] }});\nfilm.timeline({{ shots: [shot({{ id: \"c\", dur: 3, layers: [c], hits: tl.hits() }})] }});\nexport default film;\n");
    let v = run("code", &src);
    let layers = v["timeline"]["shots"][0]["layers"].as_array().unwrap();
    let l0 = layers.iter().find(|l| l["id"] == "code-l0").unwrap();
    let runs: Vec<(&str, usize)> = l0["runs"].as_array().unwrap().iter().map(|r| (r["match"].as_str().unwrap(), r["nth"].as_u64().unwrap() as usize)).collect();
    assert_eq!(runs, [("const", 0), ("3", 0), ("// go", 0)]);
    let l1 = layers.iter().find(|l| l["id"] == "code-l1").unwrap();
    assert_eq!(l1["runs"][0]["match"], "const");
    assert_eq!(l1["runs"][1]["match"], "\"hi\"");
    assert_eq!(hits(&v).iter().filter(|(_, c)| c == "tick").count(), 2);
    assert!(layers.iter().find(|l| l["id"] == "code-bar").unwrap()["pos"][1].is_object(), "the highlight bar moves to line 2");
}

#[test]
fn every_component_builds_and_the_engine_accepts_the_result() {
    let src = format!("{HEAD}const tl = timeline();\nconst parts = [\n  browserWindow({{ tl }}), statCounter({{ tl, value: 373, suffix: \" fps\", label: \"FPS\", at: 0.2 }}),\n  barRace({{ tl, at: 0, bars: [{{ label: \"a\", value: 10, color: \"#fff\" }}, {{ label: \"b\", value: 3, color: \"#888\", dur: 0.5 }}] }}),\n  bboxOverlay({{ tl, at: 0, boxes: [{{ x: 10, y: 10, w: 100, h: 50, label: \"t\", change: {{ at: 1, state: \"fixed\", dx: 10 }} }}] }}),\n  cardWall({{ tl, cols: 4, rows: 3, hero: 5, heroLabel: \"3D\" }}), title3d(\"HI\", {{ tl }}),\n];\nfilm.timeline({{ shots: [shot({{ id: \"all\", dur: 3, layers: parts, hits: tl.hits() }})] }});\nexport default film;\n");
    let v = run("all", &src);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../launch");
    turbomotion_comp::plan::Plan::from_value(v.clone(), &dir).unwrap_or_else(|e| panic!("the engine rejected the film: {e:#}"));
    let h = hits(&v);
    assert!(h.iter().any(|(_, c)| c == "stamp") && h.iter().any(|(_, c)| c == "counter") && h.iter().any(|(_, c)| c == "card-in") && h.iter().any(|(_, c)| c == "tick"));
    let ids: Vec<&str> = v["timeline"]["shots"][0]["layers"].as_array().unwrap().iter().map(|l| l["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"wall") && ids.contains(&"browser") && ids.contains(&"race-bar1") && ids.contains(&"bbox0") && ids.contains(&"letter0"));
}

#[test]
fn components_are_deterministic() {
    let src = format!("{HEAD}const tl = timeline();\nfilm.timeline({{ shots: [shot({{ id: \"w\", dur: 2, layers: [cardWall({{ tl, seed: 9 }}), promptBox({{ tl, text: \"abc\" }})], hits: tl.hits() }})] }});\nexport default film;\n");
    assert_eq!(run("det-a", &src).to_string(), run("det-a", &src).to_string());
}

#[test]
fn the_font_map_passes_through_to_the_composition() {
    let v = run("fonts", &format!("{HEAD}film.add(text(\"x\", {{ font: \"JetBrains Mono\" }}));\nexport default film;\n"));
    assert_eq!(v["fonts"]["JetBrains Mono"], "../../kit/fonts/JetBrainsMono-Regular.woff");
}
