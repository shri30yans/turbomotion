use std::{fs, path::{Path, PathBuf}, time::Duration};

use turbomotion_script::{build, Options};

/// A scratch folder per test, under cargo's own target tmp dir.
fn dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sdk-tests").join(name);
    fs::create_dir_all(&d).unwrap();
    d
}

fn film(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let d = dir(name);
    for (f, src) in files {
        fs::write(d.join(f), src).unwrap();
    }
    d.join(files[0].0)
}

fn run(name: &str, src: &str) -> serde_json::Value {
    build(&film(name, &[("film.ts", src)]), &Options::default()).unwrap_or_else(|e| panic!("{e:#}"))
}

fn err(name: &str, src: &str) -> String {
    format!("{:#}", build(&film(name, &[("film.ts", src)]), &Options::default()).unwrap_err())
}

const HEAD: &str = r#"import { comp, text, rect, group, timeline, shot, text3d, letters3d, rand, grid, range } from "turbomotion";
const film = comp({ width: 1280, height: 720, fps: 30, dur: 3 });
"#;

#[test]
fn types_are_stripped_and_the_default_export_is_the_film() {
    let v = run("types", &format!("{HEAD}interface Card {{ label: string }}\nconst c: Card = {{ label: \"hi\" as string }};\nfilm.add(text(c.label, {{ pos: [10, 20] }}));\nexport default film;\n"));
    assert_eq!(v["width"], 1280);
    assert_eq!(v["layers"][0]["text"], "hi");
    assert_eq!(v["layers"][0]["type"], "text");
}

#[test]
fn relative_imports_resolve_beside_the_script() {
    let main = film("imports", &[
        ("film.ts", "import { comp } from \"turbomotion\";\nimport { label } from \"./helpers\";\nimport { size } from \"./lib/sizes.ts\";\nconst film = comp({ width: size, height: 720, fps: 30, dur: 1 });\nfilm.add(label(\"x\"));\nexport default film;\n"),
        ("helpers.ts", "import { text } from \"turbomotion\";\nexport const label = (s: string) => text(s, { pos: [1, 2] });\n"),
    ]);
    fs::create_dir_all(main.parent().unwrap().join("lib")).unwrap();
    fs::write(main.parent().unwrap().join("lib/sizes.ts"), "export const size: number = 1920;\n").unwrap();
    let v = build(&main, &Options::default()).unwrap_or_else(|e| panic!("{e:#}"));
    assert_eq!(v["width"], 1920);
    assert_eq!(v["layers"][0]["text"], "x");
}

#[test]
fn two_runs_give_identical_json_and_random_is_seeded() {
    let src = format!("{HEAD}const r = rand(5);\nfilm.add(text(String(Math.random()) + \"|\" + Date.now() + \"|\" + performance.now(), {{ pos: [r.range(0, 1000), r.int(100)] }}));\nexport default film;\n");
    let a = run("det", &src);
    let b = run("det", &src);
    assert_eq!(a.to_string(), b.to_string());
    assert!(a["layers"][0]["text"].as_str().unwrap().ends_with("|0|0"), "{a}");
}

#[test]
fn tweens_on_one_property_merge_into_one_key_list() {
    let v = run("merge", &format!("{HEAD}const a = text(\"a\", {{ pos: [100, 100], opacity: 0.5 }});\nconst tl = timeline();\ntl.to(a, {{ opacity: 1, duration: 1 }}, 0.5);\ntl.to(a, {{ opacity: 0, duration: 0.5, ease: \"none\" }}, 3);\ntl.to(a, {{ x: 400, duration: 2 }}, 0);\nfilm.add(a);\nexport default film;\n"));
    let l = &v["layers"][0];
    let keys = l["opacity"]["keys"].as_array().unwrap();
    // starts at the layer's own value, holds until the first tween, eases to 1, holds, eases to 0
    let flat: Vec<(f64, f64)> = keys.iter().map(|k| (k[0].as_f64().unwrap(), k[1].as_f64().unwrap())).collect();
    assert_eq!(flat, vec![(0.5, 0.5), (1.5, 1.0), (3.0, 1.0), (3.5, 0.0)]);
    assert_eq!(keys[3][2], "none");
    // a single tween stays a plain tween, and x moves only the first pos component
    assert_eq!(l["pos"][0]["to"], 400);
    assert_eq!(l["pos"][0]["from"], 100);
    assert_eq!(l["pos"][1], 100);
}

#[test]
fn from_tweens_and_stagger_offset_each_layer() {
    let v = run("stagger", &format!("{HEAD}const ls = letters3d(\"ABC\", {{ pos: [640, 360, 0], size: 100 }});\nconst tl = timeline();\ntl.from(ls, {{ rot_x: -90, opacity: 0, z: -300, duration: 0.5, stagger: 0.1 }}, 0.2);\nfilm.add(ls);\nexport default film;\n"));
    let ats: Vec<f64> = (0..3).map(|i| v["layers"][i]["rot_x"]["at"].as_f64().unwrap()).collect();
    assert_eq!(ats, vec![0.2, 0.3, 0.4]);
    let l = &v["layers"][1];
    assert_eq!(l["rot_x"]["from"], -90);
    assert_eq!(l["rot_x"]["to"], 0);
    assert_eq!(l["opacity"]["to"], 1);
    assert_eq!(l["pos"][2]["from"], -300);
    assert_eq!(l["pos"][2]["to"], 0);
    assert_eq!(l["ease"], serde_json::Value::Null);
    assert_eq!(l["rot_x"]["ease"], "power1.out");
}

#[test]
fn text3d_is_a_mesh_with_a_text_geometry() {
    let v = run("t3d", &format!("{HEAD}film.add(text3d(\"Hi\", {{ font: \"Avenir Next\", weight: 800, size: 200, depth: 60, bevel: 4, color: \"#fff\", side: \"#a00\", pos: [1, 2, 3], rot_y: 20 }}));\nexport default film;\n"));
    let t = v["layers"][0].as_object().unwrap();
    let mut keys: Vec<_> = t.keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["color", "geometry", "id", "pos", "rot_y", "side", "three_d", "type"]);
    assert_eq!(t["type"], "mesh");
    assert_eq!(t["three_d"], true);
    let g = t["geometry"].as_object().unwrap();
    assert_eq!((g["kind"].as_str(), g["text"].as_str(), g["size"].as_f64(), g["depth"].as_f64(), g["bevel"].as_f64(), g["weight"].as_u64()), (Some("text"), Some("Hi"), Some(200.0), Some(60.0), Some(4.0), Some(800)));
}

#[test]
fn solids_and_a_turntable_build_valid_mesh_layers() {
    let src = format!("{HEAD}import {{ box, sphere, torus, cylinder, extrude, spin }} from \"turbomotion\";\nfilm.add([box({{ size: 100, depth: 40 }}, {{ rot_y: spin(90, 20) }}), sphere({{ size: 80 }}), torus({{ size: 90 }}), cylinder({{ size: 60, height: 120, top: 0 }}), extrude({{ d: \"M0 0H10V10Z\", size: 90 }})]);\nexport default film;\n");
    let v = run("solids", &src);
    let kinds: Vec<_> = v["layers"].as_array().unwrap().iter().map(|l| l["geometry"]["kind"].as_str().unwrap().to_string()).collect();
    assert_eq!(kinds, ["box", "sphere", "torus", "cylinder", "extrude"]);
    assert_eq!(v["layers"][0]["rot_y"], "20+time*90");
    turbomotion_comp::plan::Plan::from_value(v, Path::new(".")).unwrap_or_else(|e| panic!("{e:#}"));
}

#[test]
fn grid_range_and_shots_compile() {
    let v = run("shots", &format!("{HEAD}const cells = grid(3, 2, 100);\nconst s = shot({{ id: \"a\", dur: 2, layers: cells.map((c) => rect(50, 50, {{ pos: [c.x, c.y] }})), in: \"pan_out_left\" }});\nfilm.timeline({{ shots: [s, shot({{ id: \"b\", dur: 1, layers: [group({{ id: \"g\" }}, range(2).map((i) => text(\"n\" + i)))] }})] }});\nexport default film;\n"));
    assert_eq!(v["timeline"]["shots"][0]["layers"].as_array().unwrap().len(), 6);
    assert_eq!(v["timeline"]["shots"][0]["in"], "pan_out_left");
    let b = v["timeline"]["shots"][1]["layers"].as_array().unwrap();
    assert_eq!(b.len(), 3);
    assert_eq!(b[1]["parent"], "g");
}

#[test]
fn runtime_errors_name_the_typescript_file_and_line() {
    let e = err("err-line", "import { comp } from \"turbomotion\";\n\nfunction boom(n: number): number {\n  throw new Error(\"bad number \" + n);\n}\nconst film = comp({ width: 10, height: 10, fps: 30, dur: 1 });\nboom(3);\nexport default film;\n");
    assert!(e.contains("bad number 3"), "{e}");
    assert!(e.contains("film.ts:4"), "expected the throw on line 4 in {e}");
    assert!(e.contains("film.ts:7"), "expected the call on line 7 in {e}");
}

#[test]
fn sdk_validation_errors_are_readable() {
    let e = err("err-sdk", &format!("{HEAD}film.add(text(\"x\"));\nfilm.add({{ nope: 1 }});\nexport default film;\n"));
    assert!(e.contains("expected a layer"), "{e}");
    let e = err("err-size", "import { comp } from \"turbomotion\";\nexport default comp({ width: 0 as number, height: 1, fps: 30 });\n");
    assert!(e.contains("\"width\" must be a positive number"), "{e}");
    let e = err("err-prop", &format!("{HEAD}const a = text(\"x\");\ntimeline().to(a, {{ wobble: 3 }} as any);\nexport default film;\n"));
    assert!(e.contains("\"wobble\" cannot be animated"), "{e}");
}

#[test]
fn syntax_and_import_errors_say_what_and_where() {
    let e = err("err-syntax", "const a = 1;\nconst b = ;\nexport default a;\n");
    assert!(e.contains("film.ts:2:"), "{e}");
    let e = err("err-import", "import { x } from \"nowhere\";\nexport default x;\n");
    assert!(e.contains("nowhere"), "{e}");
    let e = err("err-missing", "import { x } from \"./missing\";\nexport default x;\n");
    assert!(e.contains("missing"), "{e}");
}

#[test]
fn a_script_with_no_default_export_is_told_so_and_endless_loops_stop() {
    let e = err("err-noexport", "const a = 1;\n");
    assert!(e.contains("no default export"), "{e}");
    let path = film("loop", &[("film.ts", "while (true) {}\nexport default 1;\n")]);
    let e = build(&path, &Options { time_limit: Duration::from_millis(300), ..Options::default() }).unwrap_err();
    assert!(format!("{e:#}").contains("ran longer"), "{e:#}");
}

#[test]
fn the_launch_example_builds_and_loads_as_a_composition() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sdk/launch/film.ts");
    let v = build(&root, &Options::default()).unwrap_or_else(|e| panic!("{e:#}"));
    assert_eq!(v["timeline"]["shots"].as_array().unwrap().len(), 2);
    turbomotion_comp::plan::Plan::from_value(v, root.parent().unwrap()).unwrap_or_else(|e| panic!("{e:#}"));
}
