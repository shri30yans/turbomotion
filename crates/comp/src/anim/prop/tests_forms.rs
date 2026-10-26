//! The JSON forms: what is accepted, what is refused and how, and that the old forms still mean what they meant.

use serde_json::json;

use super::{
    testkit::{at, position, position_err, scalar, scalar_err, NoEnv},
    ColorProp, Loop, VecProp,
};
use crate::anim::ease::Ease;

/// What a key list meant before keys could carry handles: `a + (b - a) * ease(u) + extra`, written out by hand.
fn legacy(keys: &[(f64, f64, Ease)], looping: Loop, t: f64) -> f64 {
    let (first, last) = (keys[0], keys[keys.len() - 1]);
    let span = last.0 - first.0;
    let (mut t, mut extra) = (t, 0.0);
    if t > last.0 && span > 0.0 {
        let n = ((t - first.0) / span).floor();
        match looping {
            Loop::None => {}
            Loop::Cycle => t = first.0 + (t - first.0).rem_euclid(span),
            Loop::Offset => {
                t = first.0 + (t - first.0).rem_euclid(span);
                extra = n * (last.1 - first.1);
            }
            Loop::PingPong => {
                let m = (t - first.0).rem_euclid(2.0 * span);
                t = first.0 + if m > span { 2.0 * span - m } else { m };
            }
        }
    }
    if t <= first.0 {
        return first.1 + extra;
    }
    match keys.windows(2).find(|w| t < w[1].0) {
        None => last.1 + extra,
        Some(w) => w[0].1 + (w[1].1 - w[0].1) * w[1].2.at((t - w[0].0) / (w[1].0 - w[0].0).max(1e-9)) + extra,
    }
}

#[test]
fn every_old_key_list_gives_the_same_bits_as_before() {
    let eases = ["power2.out", "expo.in", "back.out(1.7)", "sine.inOut", "elastic.out", "bounce.out", "hold", "none"];
    for name in eases {
        for (word, looping) in [("none", Loop::None), ("cycle", Loop::Cycle), ("pingpong", Loop::PingPong), ("offset", Loop::Offset)] {
            let e = Ease::parse(name).unwrap();
            let keys = [(0.5, -20.0, Ease::default()), (1.5, 140.0, e), (2.0, 15.0, Ease::default()), (3.25, 80.0, e)];
            let mut json = json!({"keys": [[0.5, -20], [1.5, 140, name], [2, 15], [3.25, 80, name]]});
            if word != "none" {
                json["loop"] = json!(word);
            }
            let p = scalar(json);
            for i in -10..400 {
                let t = f64::from(i) * 0.037;
                assert_eq!(p.base(t).to_bits(), legacy(&keys, looping, t).to_bits(), "{name} {word} t={t}");
            }
        }
    }
}

#[test]
fn old_vector_forms_still_parse_to_one_property_per_axis() {
    for v in [json!([100, 50]), json!({"from": [0, 0], "to": [10, 20], "dur": 1}), json!({"keys": [[0, [0, 0]], [1, [100, 50], "expo.out"]]}), json!({"separate_dimensions": [{"keys": [[0, 0], [1, 9]]}, 3]})] {
        assert!(matches!(position(v.clone()), VecProp::Axes(a) if a.len() == 2), "{v}");
    }
    let p = position(json!({"separate_dimensions": [{"keys": [{"t": 0, "v": 0, "ease_out": "hold"}, {"t": 1, "v": 9}]}, 3]}));
    assert_eq!(at(&p, 0.5), vec![0.0, 3.0]);
    assert_eq!(at(&p, 1.0), vec![9.0, 3.0]);
}

#[test]
fn a_vector_that_is_not_a_position_interpolates_each_component() {
    let p = VecProp::parse(&json!({"keys": [{"t": 0, "v": [1, 2, 3], "ease": "linear"}, {"t": 2, "v": [3, 6, 11], "ease": "linear"}]}), 2..=3).unwrap();
    assert!(matches!(p, VecProp::Joined { .. }));
    assert_eq!(p.at(1.0, &NoEnv).unwrap(), vec![2.0, 4.0, 7.0]);
    assert_eq!(p.len(), 3);
}

#[test]
fn a_colour_with_handles_interpolates_each_channel_as_written() {
    // channels 0..255 and alpha 0..1, each on its own; see parity/specs/property_model.md for why not in linear light
    let c = ColorProp::parse(&json!({"keys": [{"t": 0, "v": "#000000"}, {"t": 2, "v": [200, 100, 50, 0.5], "ease": "linear"}]})).unwrap();
    assert_eq!(c.at(1.0, &NoEnv).unwrap(), [100.0, 50.0, 25.0, 0.75]);
    assert_eq!(c.at(-1.0, &NoEnv).unwrap(), [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(c.at(9.0, &NoEnv).unwrap(), [200.0, 100.0, 50.0, 0.5]);
}

#[test]
fn a_path_survives_the_composition_schema() {
    let p = position(json!({"keys": [{"t": 0, "v": [0, 0, 0], "spatial": "bezier", "out": [10, 0, 5]}, {"t": 1, "v": [100, 50, 20], "in": [-10, 0, -5]}], "auto_orient": "path"}));
    let VecProp::Joined { track, auto_orient } = &p else { panic!("joined") };
    assert!(auto_orient);
    assert_eq!(track.keys()[0].tangents, super::Tangents::Bezier { into: [0.0; 3], out: [10.0, 0.0, 5.0] });
    assert_eq!(track.keys()[1].tangents, super::Tangents::Bezier { into: [-10.0, 0.0, -5.0], out: [0.0; 3] });
}

#[test]
fn bad_handles_are_refused_with_what_to_change() {
    let two = |extra: serde_json::Value| {
        let mut k = json!({"t": 1, "v": [10, 10]});
        k.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        json!({"keys": [{"t": 0, "v": [0, 0]}, k]})
    };
    let e = position_err(two(json!({"in": [1, 2, 3]})));
    assert!(e.contains("\"in\"") && e.contains("2 numbers"), "{e}");
    let e = position_err(two(json!({"ease_in": {"speed": 5, "influence": 150}})));
    assert!(e.contains("influence 150") && e.contains("0..100"), "{e}");
    let e = position_err(json!({"keys": [{"t": 2, "v": [0, 0]}, {"t": 1, "v": [1, 1]}]}));
    assert!(e.contains("time order"), "{e}");
    let e = position_err(two(json!({"ease_ot": "linear"})));
    assert!(e.contains("ease_ot") && e.contains("Did you mean \"ease_out\""), "{e}");
    let e = position_err(two(json!({"ease": "linear", "ease_in": "easy"})));
    assert!(e.contains("not both"), "{e}");
    let e = position_err(two(json!({"ease_in": {"speed": [1, 2], "influence": 30}})));
    assert!(e.contains("2 speeds") && e.contains("along the path"), "{e}");
    let e = position_err(two(json!({"ease_in": {"speed": 3}})));
    assert!(e.contains("influence"), "{e}");
    let e = position_err(two(json!({"spatial": "continuous", "in": [1, 0], "out": [1, 0]})));
    assert!(e.contains("opposite"), "{e}");
    let e = position_err(two(json!({"spatial": "auto", "out": [1, 0]})));
    assert!(e.contains("bezier"), "{e}");
    let e = position_err(json!({"keys": [{"roving": true, "v": [1, 1]}, {"t": 1, "v": [0, 0]}]}));
    assert!(e.contains("roving"), "{e}");
    let e = position_err(json!({"keys": [{"t": 0, "v": [0, 0]}, {"roving": true, "t": 0.5, "v": [1, 1]}, {"t": 1, "v": [0, 0]}]}));
    assert!(e.contains("no time"), "{e}");
    let e = position_err(json!({"keys": [{"t": 0, "v": [0, 0]}, {"roving": true, "v": [0, 0]}]}));
    assert!(e.contains("last key"), "{e}");
    let e = position_err(json!({"keys": [{"t": 0, "v": [0, 0]}, {"t": 1, "v": [1, 1, 1]}]}));
    assert!(e.contains("key 1"), "{e}");
    let e = position_err(json!({"keys": [{"t": 0, "v": [0, 0]}], "auto_orient": "yes"}));
    assert!(e.contains("auto_orient"), "{e}");
}

#[test]
fn handles_that_belong_to_a_path_are_refused_where_there_is_none() {
    let e = format!("{:#}", VecProp::parse(&json!({"keys": [{"t": 0, "v": [1, 1], "out": [1, 0]}, {"t": 1, "v": [2, 2]}]}), 2..=3).expect_err("a scale has no path"));
    assert!(e.contains("motion path"), "{e}");
    let e = scalar_err(json!({"keys": [{"t": 0, "v": 0, "ease_out": {"speed": [1, 2], "influence": 10}}, {"t": 1, "v": 1}]}));
    assert!(e.contains("2 speeds"), "{e}");
    let e = scalar_err(json!({"keys": [{"t": 1, "v": 0}, {"t": 2, "v": 1, "ease_in": "hold"}]}));
    assert!(e.contains("only the side that leaves a key can hold"), "{e}");
}

#[test]
fn a_switch_and_text_change_only_at_a_key() {
    use super::{Key, Kind, Options, Track, Value};
    let switch = Track::new(Kind::Bool, vec![Key::new(0.0, Value::B(false)), Key::new(1.0, Value::B(true)), Key::new(2.0, Value::B(false))], Options::default()).unwrap();
    let on: Vec<bool> = [0.0, 0.99, 1.0, 1.5, 2.0, 5.0].iter().map(|t| switch.sample(*t) == Value::B(true)).collect();
    assert_eq!(on, [false, false, true, true, false, false]);
    let text = Track::new(Kind::Text, vec![Key::new(0.0, Value::Text("one".into())), Key::new(1.0, Value::Text("two".into()))], Options::default()).unwrap();
    assert_eq!((text.sample(0.5), text.sample(1.0)), (Value::Text("one".into()), Value::Text("two".into())));
    let e = Track::new(Kind::Bool, vec![Key::new(0.0, Value::B(true)), Key::new(1.0, Value::F(1.0))], Options::default()).expect_err("mixed types");
    assert!(format!("{e:#}").contains("key 1 holds a number"), "{e:#}");
    let handled = Key { ease_out: Some(super::Side::Auto), ..Key::new(0.0, Value::B(true)) };
    assert!(Track::new(Kind::Bool, vec![handled, Key::new(1.0, Value::B(false))], Options::default()).is_err(), "a switch has no ease");
}
