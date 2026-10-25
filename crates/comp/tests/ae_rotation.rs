//! Pose matrices against After Effects, when the owner has dumped them.
//!
//! `tools/ae-export/dump_layer_matrices.jsx` writes `golden/ae_rotation.json`. The test is `#[ignore]`d (it shows as
//! ignored, never as a pass) because the file needs After Effects to produce; once it is copied in, run
//! `cargo test -p turbomotion-comp --test ae_rotation -- --ignored`. A missing file then fails loudly. Every case
//! must agree with `Pose::matrix` within `AE_PRECISION`.

use std::path::Path;

use turbomotion_comp::eval::{mat4::Mat4, pose::Pose};

/// After Effects prints matrix entries with about six digits.
const AE_PRECISION: f64 = 1e-4;

#[test]
#[ignore = "needs tests/golden/ae_rotation.json, dumped from After Effects by tools/ae-export/dump_layer_matrices.jsx"]
fn pose_matrices_match_the_after_effects_dump() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/ae_rotation.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}; run tools/ae-export/dump_layer_matrices.jsx in After Effects and copy its output there", path.display()));
    let doc: serde_json::Value = serde_json::from_str(&text).expect("ae_rotation.json is JSON");
    let cases = doc["cases"].as_array().expect("a \"cases\" list");
    assert!(!cases.is_empty());
    let v3 = |v: &serde_json::Value| -> [f64; 3] { std::array::from_fn(|i| v[i].as_f64().expect("three numbers")) };
    let mut bad = vec![];
    for c in cases {
        let m = Pose::from_angles(v3(&c["orientation"]), v3(&c["rotation"])).matrix(&Mat4::IDENTITY).0;
        let ours = [[m[0], m[4], m[8]], [m[1], m[5], m[9]], [m[2], m[6], m[10]]];
        for (axis, want) in ["x_axis", "y_axis", "z_axis"].iter().zip(ours) {
            let got = v3(&c[*axis]);
            if got.iter().zip(want).any(|(a, b)| (a - b).abs() > AE_PRECISION) {
                bad.push(format!("{} {axis}: AE {got:?}, ours {want:?}", c["name"]));
            }
        }
    }
    assert!(bad.is_empty(), "{} mismatches:\n{}", bad.len(), bad.join("\n"));
}
