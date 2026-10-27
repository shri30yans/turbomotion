//! The composition keys `"motion_blur"` and `"mblur"`: what is accepted, and mistakes answered with the nearest valid key or
//! the range, never ignored.
use std::path::Path;

use serde_json::{json, Value};
use turbomotion_comp::{
    plan::{Plan, ROOT},
    schema::{Blur, Shutter},
};

fn plan(extra: Value) -> anyhow::Result<Plan> {
    let mut film = json!({"width": 64, "height": 48, "fps": 30, "dur": 1, "layers": [{"type": "solid", "color": "#fff", "size": [8, 8], "mblur": true}]});
    film.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    Plan::from_value(film, Path::new("."))
}

fn error(extra: Value) -> String {
    format!("{:#}", plan(extra).err().expect("rejected"))
}

#[test]
fn leaving_both_keys_out_gives_no_blur_setting_and_the_block_gives_after_effects_defaults() {
    assert_eq!(plan(json!({})).unwrap().comps[ROOT].blur, None);
    let blur = plan(json!({"motion_blur": {}})).unwrap().comps[ROOT].blur;
    assert_eq!(blur, Some(Blur::AfterEffects(Shutter { shutter_angle: 180.0, shutter_phase: -90.0, samples: 16 })));
}

#[test]
fn the_older_key_is_a_legacy_shutter_centred_on_the_frame() {
    let blur = plan(json!({"mblur": {"samples": 10, "shutter": 200}})).unwrap().comps[ROOT].blur.unwrap();
    assert_eq!(blur, Blur::Legacy(Shutter { shutter_angle: 200.0, shutter_phase: -100.0, samples: 10 }));
}

#[test]
fn a_mistyped_key_names_the_nearest_valid_one() {
    let e = error(json!({"motion_blur": {"shutter_angel": 180}}));
    assert!(e.contains("shutter_angel") && e.contains("shutter_angle"), "{e}");
    let e = error(json!({"motionblur": {}}));
    assert!(e.contains("unknown key \"motionblur\"") && e.contains("Did you mean \"motion_blur\"?"), "{e}");
}

#[test]
fn numbers_outside_their_ranges_are_refused_with_the_range() {
    for (block, needle) in [
        (json!({"shutter_angle": 721}), "between 0 and 720"),
        (json!({"shutter_angle": -1}), "between 0 and 720"),
        (json!({"shutter_phase": 361}), "between -360 and 360"),
        (json!({"samples": 17}), "at most 16"),
        (json!({"samples": 1}), "at least 2"),
    ] {
        let e = error(json!({"motion_blur": block.clone()}));
        assert!(e.contains(needle) && e.contains("motion_blur"), "{block}: {e}");
    }
    assert!(error(json!({"motion_blur": {"samples": 17}})).contains("costs `samples` times as much"));
    assert!(plan(json!({"motion_blur": {"shutter_angle": 720, "shutter_phase": -360, "samples": 2}})).is_ok());
}

#[test]
fn the_two_keys_together_are_refused_and_the_message_says_which_to_keep() {
    let e = error(json!({"mblur": {}, "motion_blur": {}}));
    assert!(e.contains("both \"mblur\" and \"motion_blur\"") && e.contains("use \"motion_blur\""), "{e}");
}

#[test]
fn a_timeline_that_switches_its_shots_takes_the_shutter_of_the_block_and_its_object_form_is_the_older_key() {
    let film = |timeline_mblur: Value, key: &str| {
        let shot = json!({"id": "one", "dur": 1.0, "layers": [{"type": "solid", "color": "#fff", "size": [8, 8]}]});
        let mut film = json!({"width": 64, "height": 48, "fps": 30, "timeline": {"shots": [shot], "mblur": timeline_mblur}});
        film[key] = json!({"samples": 4});
        Plan::from_value(film, Path::new("."))
    };
    let plan = film(json!(true), "motion_blur").unwrap();
    assert_eq!(plan.comps[ROOT].blur, Some(Blur::AfterEffects(Shutter { samples: 4, ..Shutter::default() })));
    assert!(plan.comps[ROOT].layers.iter().any(|l| l.def.common.mblur), "the shot's layer is switched on");
    let message = format!("{:#}", film(json!({"samples": 8, "shutter": 180}), "motion_blur").err().expect("both keys"));
    assert!(message.contains("both"), "{message}");
}
