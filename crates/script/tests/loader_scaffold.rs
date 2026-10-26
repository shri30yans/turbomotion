//! `sdk new scene` writes a typed scene that builds and checks clean, and never overwrites.
mod common;
use std::fs;
use turbomotion_script::{build, check, scaffold};

#[test]
fn the_scaffold_writes_its_files_and_the_probe_builds_and_checks_with_no_errors() {
    let dir = common::scratch("scaffold", "ok");
    let created = scaffold::scene("chevron-tunnel", &dir, true).unwrap();
    for file in ["chevron-tunnel/index.ts", "chevron-tunnel/types.ts", "probe_chevron-tunnel.ts"] {
        assert!(dir.join(file).is_file(), "{file}");
    }
    assert!(created.render.contains("probe_chevron-tunnel.ts") && created.render.contains("sdk render"), "{}", created.render);
    let types = fs::read_to_string(dir.join("chevron-tunnel/types.ts")).unwrap();
    assert!(types.contains("ChevronTunnelOptions"), "{types}");
    let film = build(&dir.join("probe_chevron-tunnel.ts"), &Default::default()).unwrap();
    assert_eq!(film["timeline"]["shots"][0]["id"], "chevron-tunnel");
    let report = check::auto(&dir.join("probe_chevron-tunnel.ts"), None, None, true).unwrap();
    assert!(report.success, "{}", report.diagnostics);
}

#[test]
fn without_probe_only_the_scene_folder_is_written() {
    let dir = common::scratch("scaffold", "noprobe");
    scaffold::scene("vs", &dir, false).unwrap();
    assert!(dir.join("vs/index.ts").is_file() && !dir.join("probe_vs.ts").exists());
}

#[test]
fn an_existing_file_is_never_overwritten_and_nothing_else_is_written() {
    let dir = common::scratch("scaffold", "conflict");
    fs::create_dir_all(dir.join("vs")).unwrap();
    fs::write(dir.join("vs/types.ts"), "mine").unwrap();
    let error = scaffold::scene("vs", &dir, true).unwrap_err().to_string();
    assert!(error.contains("types.ts"), "{error}");
    assert_eq!(fs::read_to_string(dir.join("vs/types.ts")).unwrap(), "mine");
    assert!(!dir.join("vs/index.ts").exists() && !dir.join("probe_vs.ts").exists());
}

#[test]
fn an_unsafe_name_is_refused() {
    let dir = common::scratch("scaffold", "badname");
    assert!(scaffold::scene("../escape", &dir, false).is_err());
}
