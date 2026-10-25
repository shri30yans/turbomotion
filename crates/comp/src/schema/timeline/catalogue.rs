//! The names a timeline accepts, with what each looks like: transitions (and their default sound), 2D frame moves
//! and 3D camera moves. `comp catalogue` prints these.

/// Shot-to-shot transitions: the name an author writes, what it looks like, the sound it makes by default.
pub const TRANSITIONS: &[(&str, &str, Option<&str>)] = &[
    ("cut", "no transition", None),
    ("dissolve", "the next shot fades in over the last", None),
    ("blur_dissolve", "a dissolve through a soft blur", Some("whoosh")),
    ("dip_black", "fade through black", None),
    ("dip_white", "fade through white (a flash cut)", Some("riser")),
    ("dip", "fade through `color`", None),
    ("push_left", "both shots travel left as one strip", Some("whoosh")),
    ("push_right", "both shots travel right", Some("whoosh")),
    ("push_up", "both shots travel up", Some("whoosh")),
    ("push_down", "both shots travel down", Some("whoosh")),
    ("slide_over_left", "the next shot slides in from the right over the last, which steps back and dims", Some("whoosh")),
    ("slide_over_right", "slides in from the left over the last", Some("whoosh")),
    ("slide_over_up", "slides up over the last", Some("whoosh")),
    ("slide_over_down", "slides down over the last", Some("whoosh")),
    ("whip_pan_left", "a camera whip pan with motion smear", Some("swish")),
    ("whip_pan_right", "a whip pan to the right", Some("swish")),
    ("pan_out_left", "pull back until both shots are tiles on one canvas, pan, push into the next", Some("sweep")),
    ("pan_out_right", "pull back, pan right, push in", Some("sweep")),
    ("pan_out_up", "pull back, pan up, push in", Some("sweep")),
    ("pan_out_down", "pull back, pan down, push in", Some("sweep")),
    ("zoom_through", "fly forward through the last shot into the next", Some("riser")),
    ("zoom_back", "fly backward out of the last shot", Some("sweep")),
    ("cross_zoom", "a constant-speed dolly: both shots zoom at the same rate, the next arrives from far behind the last (no jump in speed at the hand-over)", Some("riser")),
    ("window", "the next shot is a rounded card (`portal`, `radius`) that opens like a window while the last zooms toward it, frame-in-frame", Some("riser")),
    ("iris", "a circle opens on the next shot", Some("reveal")),
    ("flood", "a disc of `color` floods the frame, then opens on the next shot", Some("pop")),
    ("wipe_left", "a hard edge reveals the next shot from the left", Some("whoosh")),
    ("wipe_right", "a hard edge reveals the next shot from the right", Some("whoosh")),
    ("wipe_up", "a hard edge reveals the next shot from the bottom", Some("whoosh")),
    ("wipe_down", "a hard edge reveals the next shot from the top", Some("whoosh")),
    ("glitch", "a jittering, smeared, flickering cut", Some("glitch")),
];

/// Moves of the frame itself over a shot's life (a 2D camera on the shot as a whole).
pub const MOVES: &[(&str, &str)] = &[
    ("push_in", "a slow push in by `amount` (0.06)"),
    ("pull_out", "a slow pull out by `amount`"),
    ("drift_left", "the frame drifts left (scaled up by `amount` so no edge shows)"),
    ("drift_right", "drifts right"),
    ("drift_up", "drifts up"),
    ("drift_down", "drifts down"),
    ("ken_burns", "push in while drifting left"),
    ("handheld", "a gentle, deterministic handheld float"),
    ("punch", "a stepped punch-in on each time in `at` (a list), on a fast ease: the beat of a cut without a cut"),
];

/// 3D camera moves added inside a shot (its layers need `three_d`).
pub const CAMERA_MOVES: &[(&str, &str)] = &[
    ("orbit", "the camera circles the centre by `deg` degrees (30)"),
    ("dolly_in", "the camera travels toward the scene by `amount` of its distance (0.3)"),
    ("dolly_out", "the camera travels away"),
    ("dolly_zoom", "vertigo: travels in while the lens widens, the centre plane stays the same size, depth stretches"),
    ("crane_up", "the camera rises while looking at the centre"),
    ("crane_down", "the camera descends"),
    ("truck_left", "the camera slides left, looking straight ahead (parallax across depth)"),
    ("truck_right", "the camera slides right"),
];

/// Where the loud part of each kit cue is, so a transition's sound lands on its midpoint.
pub fn cue_peak(cue: &str) -> f64 {
    match cue {
        "whoosh" => 0.28,
        "swish" => 0.1,
        "sweep" => 0.55,
        "riser" => 0.85,
        "pop" | "click" | "tick" | "boom" | "stamp" | "counter" => 0.0,
        "glitch" => 0.06,
        "reveal" => 0.2,
        _ => 0.1,
    }
}
