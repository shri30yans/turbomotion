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

#[test]
fn a_probe_is_a_film_with_its_own_effect_that_checks_clean_and_whose_sheet_shows_the_effect() {
    let dir = common::scratch("scaffold", "probe");
    let created = turbomotion_script::scaffold::probe("inkwash", &dir).unwrap();
    for file in ["inkwash/film.ts", "inkwash/effects/inkwash.sksl", "inkwash/blocks/.gitkeep"] {
        assert!(dir.join(file).is_file(), "{file}");
    }
    assert!(created.render.contains("sdk render") && created.render.contains("--sheet") && created.render.contains("film.ts"), "{}", created.render);
    let film_file = dir.join("inkwash/film.ts");
    let film = build(&film_file, &Default::default()).unwrap();
    assert_eq!(film["fx_path"], serde_json::json!(["effects"]));
    assert_eq!(film["block_path"], serde_json::json!(["blocks"]));
    assert_eq!(film["layers"][0]["fx"][0]["effect"], "inkwash");
    // the engine accepts the film, plugin included, with nothing to say
    let report = check::auto(&film_file, None, None, true).unwrap();
    assert!(report.success && report.diagnostics.is_empty(), "{}", report.diagnostics);
    // the sheet is drawn, twice the same, and the effect changes it (amount 0 is the plain text)
    let sheet = |script: &std::path::Path, out: &str| {
        let comp = turbomotion_script::cli::build_to(&turbomotion_script::cli::Script { file: script.to_path_buf(), sdk_dir: None, shot: None }, None).unwrap();
        let path = dir.join(out);
        turbomotion_script::cli::write_sheet(&comp, &path, &[0.5, 1.5]).unwrap();
        fs::read(&path).unwrap()
    };
    let (a, b) = (sheet(&film_file, "a.png"), sheet(&film_file, "b.png"));
    assert_eq!(a, b, "two renders of the same sheet are identical");
    let plain = dir.join("inkwash/plain.ts");
    fs::write(&plain, fs::read_to_string(&film_file).unwrap().replace("amount: 0.8", "amount: 0")).unwrap();
    let off = sheet(&plain, "c.png");
    assert_ne!(a, off, "the effect at 0.8 differs from the effect at 0");
}

#[test]
fn a_probe_name_must_be_a_new_effect_name_and_nothing_is_overwritten() {
    let dir = common::scratch("scaffold", "probe-names");
    for bad in ["blur", "Has-Dash", "2fast", "../x", ""] {
        assert!(turbomotion_script::scaffold::probe(bad, &dir).is_err(), "{bad:?}");
    }
    assert!(turbomotion_script::scaffold::probe("blur", &dir).unwrap_err().to_string().contains("already an effect"));
    turbomotion_script::scaffold::probe("wash", &dir).unwrap();
    fs::write(dir.join("wash/film.ts"), "mine").unwrap();
    let e = turbomotion_script::scaffold::probe("wash", &dir).unwrap_err().to_string();
    assert!(e.contains("refusing to overwrite") && e.contains("film.ts"), "{e}");
    assert_eq!(fs::read_to_string(dir.join("wash/film.ts")).unwrap(), "mine");
}
