//! The names a timeline accepts, with what each looks like: transitions (and their default sound), 2D frame moves
//! and 3D camera moves. `comp catalogue` prints these.

/// Shot-to-shot transitions: the name an author writes, what it looks like, the sound it makes by default.
pub const TRANSITIONS: &[(&str, &str, Option<&str>)] = &[
    ("cut", "no transition", None),
    ("dissolve", "the next shot fades in over the last", None),
    ("blur_dissolve", "a dissolve through a soft blur", Some("soft-pass")),
    ("dip_black", "fade through black", None),
    ("dip_white", "fade through white (a flash cut)", Some("riser/quick")),
    ("dip", "fade through `color`", None),
    ("push_left", "both shots travel left as one strip", Some("fast-pass")),
    ("push_right", "both shots travel right", Some("fast-pass")),
    ("push_up", "both shots travel up", Some("fast-pass")),
    ("push_down", "both shots travel down", Some("fast-pass")),
    ("slide_over_left", "the next shot slides in from the right over the last, which steps back and dims", Some("heavy-pass")),
    ("slide_over_right", "slides in from the left over the last", Some("heavy-pass")),
    ("slide_over_up", "slides up over the last", Some("heavy-pass")),
    ("slide_over_down", "slides down over the last", Some("heavy-pass")),
    ("whip_pan_left", "a camera whip pan with motion smear", Some("swish/whip")),
    ("whip_pan_right", "a whip pan to the right", Some("swish/whip")),
    ("pan_out_left", "pull back until both shots are tiles on one canvas, pan, push into the next", Some("sweep")),
    ("pan_out_right", "pull back, pan right, push in", Some("sweep")),
    ("pan_out_up", "pull back, pan up, push in", Some("sweep")),
    ("pan_out_down", "pull back, pan down, push in", Some("sweep")),
    ("zoom_through", "fly forward through the last shot into the next", Some("riser/quick")),
    ("zoom_back", "fly backward out of the last shot", Some("pull-back")),
    ("cross_zoom", "a constant-speed dolly: both shots zoom at the same rate, the next arrives from far behind the last (no jump in speed at the hand-over)", Some("riser/tonal")),
    ("window", "the next shot is a rounded card (`portal`, `radius`) that opens like a window while the last zooms toward it, frame-in-frame", Some("riser/noise")),
    ("iris", "a circle opens on the next shot", Some("shimmer/sparkle")),
    ("flood", "a disc of `color` floods the frame, then opens on the next shot", Some("pop")),
    ("wipe_left", "a hard edge reveals the next shot from the left", Some("wipe-pass")),
    ("wipe_right", "a hard edge reveals the next shot from the right", Some("wipe-pass")),
    ("wipe_up", "a hard edge reveals the next shot from the bottom", Some("wipe-pass")),
    ("wipe_down", "a hard edge reveals the next shot from the top", Some("wipe-pass")),
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

/// A transition's sound for its `order`-th use of `cue` in the film: the exact kit variant (so the film file shows what
/// plays) and where that variant is loudest, so it can land on the transition's midpoint. Cues that are not in the kit
/// (a typo, a sound file) come back unchanged with a default peak; the planner reports them.
pub fn resolve_cue(cue: &str, order: usize) -> (String, f64) {
    match crate::media::kit::kit().pick(cue, order, TIMELINE_SEED) {
        Ok(p) => {
            let peak = crate::media::kit::kit().pick(&p.cue, 0, 0).map_or(p.peak, |exact| exact.peak);
            (p.cue, peak)
        }
        Err(_) => (cue.to_string(), 0.1),
    }
}

/// Transitions pick their sounds with this seed, so a film's transition sounds are the same on every build.
pub const TIMELINE_SEED: u64 = 0x7469_6d65_6c69_6e65;
