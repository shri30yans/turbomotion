//! The SDK's sound helpers (`hit`, `melody`, `drumPattern`, `scale`) and what the engine makes of them: the sounds a film
//! asks for by family come out different each time.

use std::{fs, path::{Path, PathBuf}};

use serde_json::Value;
mod common;

use turbomotion_script::{build, Options};

const HEAD: &str = "import { comp, shot, text, hit, melody, drumPattern, scale, transpose } from \"turbomotion\";\nconst film = comp({ width: 640, height: 360, fps: 30 });\n";

fn run(name: &str, body: &str) -> Value {
    let dir = common::scratch("sound-tests", name);
    let file: PathBuf = dir.join("film.ts");
    fs::write(&file, format!("{HEAD}{body}\nexport default film;\n")).unwrap();
    build(&file, &Options::default()).unwrap_or_else(|e| panic!("{e:#}"))
}

fn hits(v: &Value) -> Vec<(String, String, Option<f64>)> {
    v["timeline"]["hits"].as_array().unwrap().iter().map(|h| (h["at"].as_str().map_or_else(|| h["at"].to_string(), String::from), h["cue"].as_str().unwrap().to_string(), h["volume"].as_f64())).collect()
}

fn at(h: &(String, String, Option<f64>)) -> f64 {
    h.0.parse().unwrap()
}

#[test]
fn melody_emits_one_pitched_hit_per_note_and_skips_rests() {
    let v = run("melody", r#"film.timeline({ shots: [shot({ id: "a", dur: 6, layers: [text("x")] })], hits: [
        ...melody(["C4", "Eb4", null, ["G3", "G4"], "c5"], { at: 2, step: 0.25, voice: "bell", volume: 0.3 }),
        ...melody(["C4", "G4"], { at: 5, bpm: 120, transpose: 12, loops: 2 })] });"#);
    let h = hits(&v);
    let cues: Vec<&str> = h.iter().map(|x| x.1.as_str()).collect();
    assert_eq!(cues, ["bell/C4", "bell/Eb4", "bell/G3", "bell/G4", "bell/C5", "pluck/C5", "pluck/G5", "pluck/C5", "pluck/G5"]);
    assert_eq!(h[..5].iter().map(at).collect::<Vec<_>>(), [2.0, 2.25, 2.75, 2.75, 3.0], "a rest takes a step; a chord shares one");
    assert!(h[..5].iter().all(|x| x.2 == Some(0.3)) && h[5].2.is_none());
    assert_eq!(h[5..].iter().map(at).collect::<Vec<_>>(), [5.0, 5.25, 5.5, 5.75], "bpm 120 with two steps a beat is a step every quarter second");
}

#[test]
fn drum_pattern_places_steps_accents_and_voices() {
    let v = run("drums", r#"film.timeline({ shots: [shot({ id: "a", dur: 6, layers: [text("x")] })], hits: [
        ...drumPattern("x...X.o.|x...", { at: 1, bpm: 120, voice: "kick" }),
        ...drumPattern("o.x.", { at: 3, step: 0.1, voice: { x: "snare/tight", o: "hat/closed-01" }, volume: 0.5 })] });"#);
    let h = hits(&v);
    assert_eq!(h.iter().map(|x| x.1.as_str()).collect::<Vec<_>>(), ["kick", "kick", "kick", "kick", "hat/closed-01", "snare/tight"]);
    assert_eq!(h[..4].iter().map(at).collect::<Vec<_>>(), [1.0, 1.5, 1.75, 2.0], "sixteenths at 120 bpm are 0.125 s; the bar line is not a step");
    assert_eq!(h[0].2, None);
    assert_eq!((h[1].2, h[2].2), (Some(0.585), Some(0.225)), "X is an accent, o a ghost note");
    assert_eq!(h[4..].iter().map(at).collect::<Vec<_>>(), [3.0, 3.2]);
}

#[test]
fn helpers_reject_mistakes_with_the_fix() {
    let err = |body: &str| {
        let dir = common::scratch("sound-tests", "bad");
        let file = dir.join("film.ts");
        fs::write(&file, format!("{HEAD}{body}\nexport default film;\n")).unwrap();
        format!("{:#}", build(&file, &Options::default()).unwrap_err())
    };
    assert!(err("melody([\"H4\"], { at: 0 });").contains("not a note name"));
    assert!(err("drumPattern(\"x.q.\", { at: 0 });").contains("has no voice"));
    assert!(err("melody([\"C4\"], { at: 0, step: -1 });").contains("step must be"));
    let v = run("scale", "film.timeline({ shots: [shot({ id: \"a\", dur: 3, layers: [text(\"x\")] })], hits: melody(scale(\"C4\", \"minor\"), { at: 0 }).concat(melody([transpose(\"C4\", 7)], { at: 2 })) });");
    let cues: Vec<String> = hits(&v).into_iter().map(|x| x.1).collect();
    assert_eq!(cues[..8], ["pluck/C4", "pluck/D4", "pluck/D#4", "pluck/F4", "pluck/G4", "pluck/G#4", "pluck/A#4", "pluck/C5"]);
    assert_eq!(cues[8], "pluck/G4");
}

#[test]
fn a_family_cue_picks_a_different_sound_each_time_and_notes_resolve_in_the_engine() {
    let v = run("spread", r#"film.timeline({ shots: [shot({ id: "a", dur: 8, layers: [text("a")] })],
        hits: [...[0.5, 1.0, 1.5, 2.0, 2.5, 3.0].map((t) => hit(t, "whoosh")), ...melody(["C4", "D4", "E4"], { at: 6, step: 0.2, voice: "marimba" }), hit(7, "kit:ui/click-01")] });"#);
    let plan = turbomotion_comp::plan::Plan::from_value(v, Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap_or_else(|e| panic!("{e:#}"));
    let log = turbomotion_comp::audio::sound_log(&plan);
    let cue = |l: &String| l.split_whitespace().nth(1).unwrap().to_string();
    let whooshes: Vec<String> = log.iter().map(cue).filter(|c| c.starts_with("whoosh/")).collect();
    assert_eq!(whooshes.len(), 6, "{log:#?}");
    assert!(whooshes.windows(2).all(|w| w[0] != w[1]), "neighbouring hits repeat a sound: {whooshes:?}");
    assert!(whooshes.iter().collect::<std::collections::BTreeSet<_>>().len() >= 5, "{whooshes:?}");
    let notes: Vec<String> = log.iter().map(cue).filter(|c| c.starts_with("marimba/")).collect();
    assert_eq!(notes.len(), 3, "{log:#?}");
    assert!(log.iter().any(|l| l.contains("ui/click-01") && l.contains("rate 1.000")), "an exact variant is played as rendered");
    assert!(log.iter().filter(|l| l.contains("marimba/")).any(|l| !l.contains("rate 1.000")), "D4 is the root a semitone away, retuned: {log:#?}");
}

#[test]
fn transitions_of_one_kind_do_not_repeat_their_sound() {
    let shots: String = (0..7).map(|i| format!("shot({{ id: \"s{i}\", dur: 2, layers: [text(\"{i}\")] }})")).collect::<Vec<_>>().join(", ");
    let v = run("transitions", &format!("film.timeline({{ transition: {{ type: \"push_left\", dur: 0.5 }}, shots: [{shots}] }});"));
    let plan = turbomotion_comp::plan::Plan::from_value(v, Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap_or_else(|e| panic!("{e:#}"));
    let log = turbomotion_comp::audio::sound_log(&plan);
    let cues: Vec<&str> = log.iter().map(|l| l.split_whitespace().nth(1).unwrap()).collect();
    assert_eq!(cues.len(), 6, "{log:#?}");
    assert!(cues.windows(2).all(|w| w[0] != w[1]) && cues.iter().collect::<std::collections::BTreeSet<_>>().len() == 6, "six pushes, six different sounds: {cues:?}");
}

#[test]
fn an_unknown_cue_is_a_plan_error_that_lists_the_families() {
    let bad = run("unknown", r#"film.timeline({ shots: [shot({ id: "a", dur: 2, layers: [text("a")] })], hits: [hit(0.5, "whoooosh")] });"#);
    let msg = format!("{:#}", turbomotion_comp::plan::Plan::from_value(bad, Path::new(env!("CARGO_MANIFEST_DIR"))).err().unwrap());
    assert!(msg.contains("whoosh") && msg.contains("kick") && msg.contains("kit:pluck/C4"), "{msg}");
    let bad = run("variant", r#"film.timeline({ shots: [shot({ id: "a", dur: 2, layers: [text("a")] })], hits: [hit(0.5, "whoosh/foggy")] });"#);
    let msg = format!("{:#}", turbomotion_comp::plan::Plan::from_value(bad, Path::new(env!("CARGO_MANIFEST_DIR"))).err().unwrap());
    assert!(msg.contains("airy-01"), "a wrong variant lists the variants: {msg}");
}
