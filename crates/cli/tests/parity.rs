//! The parity ledger in `parity/` must agree with the catalogue the engine ships, and its gap list must be readable.

use std::{path::Path, process::Command};

fn run(args: &[&str]) -> std::process::Output {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    Command::new(env!("CARGO_BIN_EXE_turbomotion")).arg("parity").args(args).current_dir(&root).output().unwrap()
}

#[test]
fn the_real_ledger_is_sound() {
    let out = run(&["check"]);
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
}

#[test]
fn the_gap_list_names_what_is_not_built_and_what_is_decided_against() {
    let out = run(&["gaps", "--json"]);
    assert!(out.status.success());
    let gaps: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let find = |name: &str| gaps.as_array().unwrap().iter().find(|g| g["name"] == name).cloned().unwrap_or_else(|| panic!("no gap named {name}"));
    assert_eq!(find("Roto Brush and Refine Edge")["status"], "not_supported");
    assert_eq!(find("Box Blur")["wp"], "WP-02");
    assert_eq!(find("Expressions: full object model")["status"], "none");
}

#[test]
fn the_report_prints_the_tier_yardstick() {
    let text = String::from_utf8(run(&["report"]).stdout).unwrap();
    assert!(text.contains("T0+T1 effects done:") && text.contains("target is 95%"), "{text}");
}
