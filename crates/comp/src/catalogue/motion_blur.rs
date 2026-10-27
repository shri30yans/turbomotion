//! The catalogue entry for motion blur. It owns the wording and one runnable composition; the behaviour is in
//! `schema::mblur` (the keys and the shutter), `render::graph::shutter` (the averaging) and `eval::same` (what is still).

use serde_json::{json, Value};

use crate::schema::{MAX_BLUR_SAMPLES, Shutter};

pub fn entry() -> Value {
    let d = Shutter::default();
    json!({
        "summary": format!("True motion blur, as After Effects has it: the frame at time `t` is the whole scene (every layer's transform, its motion path and keys with their ease or speed and influence handles, the camera, 3D layers, text animators, particles, effects and masks) drawn at several moments while the shutter is open and averaged. The shutter is open over `[t + phase, t + phase + angle]`, both in degrees of the frame interval (360 degrees is one frame), so the default angle of {} and phase of {} blur over a quarter of a frame either side of `t`. Only layers with `\"mblur\": true` blur. The blur is an average of finished pictures in the film's working colour, so with `color.linear_light` it averages light (an edge half through a pixel is level 188 on screen, not 128), and it averages premultiplied pictures, so alpha coverage is exact. Samples are spread evenly over the shutter (the middle of each of `samples` equal slices) and their number is fixed: the smear of a fast mover is `samples` copies, so it steps when the distance between samples (travel / samples) is more than a pixel or two; lower the angle or raise `samples` (at most {MAX_BLUR_SAMPLES}; each sample draws every blurred layer again). A layer that does not move, and has no effect, matte or clip, is drawn once and exactly as without blur. A comp with the block and no switched layer costs nothing. Only the layers of the root composition have a switch: a precomp layer with `\"mblur\": true` blurs everything it shows, whatever is switched inside it, and a switch inside a nested composition does nothing (`comp check` says so).", d.shutter_angle, d.shutter_phase),
        "keys": {
            "shutter_angle": format!("How long the shutter is open, in degrees of the frame interval, 0 to 720 (default {}). 0 draws no blur. A 100 px per frame mover blurs over 100 * angle / 360 px plus its own size.", d.shutter_angle),
            "shutter_phase": format!("When the shutter opens relative to the frame time, in degrees of the frame interval, -360 to 360 (default {}: a 180 degree shutter centred on the frame time; 0 opens it at the frame time, so the smear trails ahead of the layer).", d.shutter_phase),
            "samples": format!("Moments averaged per frame, 2 to {MAX_BLUR_SAMPLES} (default {}). A frame costs about this many times a frame without blur, for the layers that blur.", d.samples),
            "mblur": "A layer key, not part of the block: `\"mblur\": true` switches motion blur on for the layer (default false). With no `motion_blur` block the older key's shutter is used.",
            "older key": "`\"mblur\": {\"samples\": 8, \"shutter\": 180}` on the composition (and on a timeline) is kept as it always was: the shutter is centred on the frame time whatever its angle, a layer that is not showing at the frame time takes no part, and the numbers are not range-checked. Use `motion_blur` for After Effects' phase and for a layer that enters or leaves during the shutter; the two keys together are an error.",
        },
        "example": {"composition": {
            "width": 1280, "height": 720, "fps": 30, "dur": 2, "bg": "#101820",
            "motion_blur": {"shutter_angle": 180, "shutter_phase": -90, "samples": 16},
            "layers": [
                {"type": "solid", "id": "puck", "color": "#ffd166", "size": [120, 120], "mblur": true,
                    "pos": {"keys": [{"t": 0, "v": [320, 360], "ease_out": {"speed": 0, "influence": 60}}, {"t": 1.2, "v": [960, 360], "ease_in": {"speed": 0, "influence": 60}}]},
                    "rotation": {"from": 0, "to": 180, "at": 0, "dur": 1.2, "ease": "power1.inOut"}},
                {"type": "text", "text": "SHUTTER 180", "size": 56, "color": "#f4f7ff", "pos": [640, 600], "align": "center"}
            ]
        }},
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_names_the_keys_the_parser_reads_and_its_example_loads_as_motion_blur() {
        let e = entry();
        let keys: Vec<&str> = e["keys"].as_object().unwrap().keys().map(String::as_str).collect();
        let parsed: Shutter = serde_json::from_value(json!({"shutter_angle": 180, "shutter_phase": -90, "samples": 16})).unwrap();
        assert_eq!(parsed, Shutter::default());
        for key in ["shutter_angle", "shutter_phase", "samples", "mblur"] {
            assert!(keys.contains(&key), "the entry lacks {key}");
        }
        let plan = crate::plan::Plan::from_value(e["example"]["composition"].clone(), std::path::Path::new(".")).unwrap();
        assert!(matches!(plan.comps[crate::plan::ROOT].blur, Some(crate::schema::Blur::AfterEffects(_))));
    }
}
