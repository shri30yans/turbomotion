//! The plugin library: the shipped pack compiles, headers are checked, parameters and inputs plan correctly.

use super::*;

#[test]
fn the_whole_built_in_pack_compiles() {
    let lib = Library::load(Path::new("/nonexistent")).unwrap();
    assert!(lib.defs().count() >= 61, "the pack has {} entries", lib.defs().count());
    for name in ["bloom", "tilt_shift", "halation", "edge_glow", "liquid", "pixel_dissolve", "split_tone", "chroma_key"] {
        let d = lib.get(name).unwrap_or_else(|e| panic!("{name}: {e:#}"));
        assert!(!d.params.is_empty(), "{name} should expose knobs");
    }
    for d in lib.defs() {
        assert!(!d.summary.is_empty(), "{} needs a @summary", d.name);
    }
}

#[test]
fn a_platform_effect_describes_itself_from_its_registry_entry() {
    let descriptors = Library::builtin().unwrap().descriptors();
    let by_name = |n: &str| descriptors.iter().find(|d| d.name == n).unwrap_or_else(|| panic!("{n} has no descriptor"));
    let blur = by_name("blur");
    assert_eq!((blur.origin.as_str(), blur.params.len()), (PLATFORM_ORIGIN, 1));
    let radius = &blur.params[0];
    assert_eq!((radius.default.clone(), radius.min, radius.max, radius.optional), (vec![8.0], Some(0.0), None, false));
    assert!(!radius.doc.is_empty());
    let glow = by_name("glow");
    let colour = glow.params.iter().find(|p| p.name == "color").unwrap();
    assert!(colour.optional && colour.default.is_empty() && colour.ty == Ty::Color);
    let tint = by_name("tint").params.iter().find(|p| p.name == "color").unwrap().clone();
    assert_eq!(tint.default, vec![255.0, 255.0, 255.0, 1.0]);
    for d in descriptors.iter().filter(|d| d.origin == PLATFORM_ORIGIN) {
        assert!(d.params.iter().all(|p| !p.doc.is_empty()), "{} has an undocumented parameter", d.name);
    }
}

#[test]
fn a_platform_effect_is_checked_against_its_declaration_when_it_is_parsed() {
    use crate::schema::Fx;
    let lib = Library::builtin().unwrap();
    lib.plan_fx(&Fx::parse(&serde_json::json!({"effect": "blur"})).unwrap()).expect("every parameter may be left out");
    Fx::parse(&serde_json::json!({"effect": "blur", "radius": {"from": 8, "to": 0, "dur": 1}})).expect("an eased parameter is baked and accepted");
    let e = format!("{:#}", Fx::parse(&serde_json::json!({"effect": "blur", "radis": 3})).err().unwrap());
    assert!(e.contains("\"radis\"") && e.contains("Did you mean \"radius\"") && e.contains("Its parameters are: radius"), "{e}");
    let e = format!("{:#}", Fx::parse(&serde_json::json!({"effect": "blur", "radius": "big"})).err().unwrap());
    assert!(e.contains("effect \"blur\" parameter \"radius\""), "{e}");
}

#[test]
fn colour_params_come_from_the_declaration_of_the_effect() {
    let lib = Library::builtin().unwrap();
    assert_eq!(lib.colour_params("tint"), Some(vec!["color".to_string()]));
    assert_eq!(lib.colour_params("blur"), Some(vec![]));
    assert_eq!(lib.colour_params("duotone"), Some(vec!["shadow".to_string(), "light".to_string()]));
    assert_eq!(lib.colour_params("no_such_effect"), None);
}

#[test]
fn kit_plugins_load_from_an_include_folder_and_a_broken_one_only_hurts_its_users() {
    let dir = std::env::temp_dir().join(format!("tm-fxkit-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("shared/post")).unwrap();
    fs::write(dir.join("shared/post/swirl.sksl"), "// @effect swirl\n// @summary kit effect\n// @param amount float 0.5 0 1\nhalf4 main(float2 xy) { return image.eval(xy) * amount; }\n").unwrap();
    fs::write(dir.join("shared/oops.sksl"), "// @effect oops\nhalf4 main(float2 xy) { return nothing_here; }\n").unwrap();
    let lib = Library::load_with(&dir, &["shared".to_string()]).unwrap();
    assert!(lib.get("swirl").unwrap().origin.ends_with("shared/post/swirl.sksl"), "{}", lib.get("swirl").unwrap().origin);
    assert_eq!(lib.broken.len(), 1, "{:?}", lib.broken);
    let e = lib.get("oops").err().unwrap().to_string();
    assert!(e.contains("oops.sksl") && e.contains("nothing_here"), "{e}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn every_shipped_plugin_is_declared_documented_and_found_without_a_hand_kept_list() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fxlib/builtin");
    let files = fs::read_dir(&dir).unwrap().filter(|e| e.as_ref().unwrap().path().extension().is_some_and(|x| x == "sksl")).count();
    let lib = Library::builtin().unwrap();
    assert_eq!(lib.defs().count(), files, "every file in the plugin folder is a registered plugin");
    for d in lib.descriptors().iter().filter(|d| d.origin != "rust") {
        assert!(!d.summary.is_empty(), "{} needs a @summary", d.name);
        assert!(d.params.iter().all(|p| !p.doc.is_empty()), "{}: every @param needs a `-- doc`", d.name);
        assert!(d.inputs.iter().all(|i| !i.doc.is_empty()), "{}: every @input needs a `-- doc`", d.name);
        let name = if d.kind == Kind::Effect { d.example["effect"].as_str() } else { d.example["shader"].as_str() };
        assert_eq!(name, Some(d.name.as_str()), "{}: its example must use it", d.name);
    }
}

#[test]
fn a_descriptor_lists_inputs_passes_history_and_a_generated_example() {
    let src = "// @effect trail\n// @summary x\n// @input map layer -- steers it\n// @input prev history=2 depth=4 -- before\n// @param amount float 0.5 0 1 -- how much\n// @param mode choice soft soft,hard -- which\n// @pass a scale=0.5\nhalf4 main(float2 xy) { return image.eval(xy) * amount * mode + map.eval(xy) * prev.eval(xy) * prev2.eval(xy); }\n// @pass b\nhalf4 main(float2 xy) { return a.eval(xy) * float(resolution.x + time); }\n";
    let d = Def::compile(src, "t").unwrap().descriptor();
    assert_eq!(d.history, 2);
    assert_eq!(d.inputs.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["map", "prev"]);
    assert_eq!(d.passes.iter().map(|p| (p.name.as_str(), p.scale)).collect::<Vec<_>>(), [("a", 0.5), ("b", 1.0)]);
    assert_eq!(d.example, serde_json::json!({"effect": "trail", "amount": 0.5, "mode": "soft", "map": "layer_id"}));
}

#[test]
fn input_pass_and_choice_headers_are_checked() {
    let body = "half4 main(float2 xy) { return half4(time + resolution.x); }";
    let err = |header: &str| format!("{:#}", Def::compile(&format!("// @effect x\n{header}\n{body}"), "t").err().expect("should be rejected"));
    assert!(err("// @input m texture").contains("use `layer` or `history=N`"));
    assert!(err("// @input p history=0").contains("history=0 is out of range"));
    assert!(err("// @input p history=1 depth=99").contains("depth=99 is out of range"));
    assert!(err("// @pass first scale=2\n// @pass last").contains("scale=2 is out of range"));
    assert!(err("// @input a history=1\n// @input b history=1").contains("at most one history"));
    assert!(err("// @input time layer").contains("reserved or repeated"));
    assert!(err("// @pass first scale=0.5").contains("must be full size"));
    assert!(err("// @param m choice nope a,b").contains("not one of the options"));
    assert!(err("// @example {nope").contains("not JSON"));
    let g = Def::compile("// @effect g\n// @kind generator\n// @input m layer\nhalf4 main(float2 xy) { return half4(time + resolution.x); }", "t");
    assert!(g.err().unwrap().to_string().contains("no inputs"));
}

#[test]
fn a_use_names_layers_and_choices_in_the_forms_the_header_promises() {
    let lib = Library::builtin().unwrap();
    let plan = |v: serde_json::Value| lib.plan("displacement_map", v.as_object().unwrap());
    let ok = plan(serde_json::json!({"map": {"layer": "m", "fit": "stretch"}, "horizontal": "luma"})).unwrap();
    assert_eq!((ok.inputs[0].id.as_str(), ok.inputs[0].fit), ("m", Fit::Stretch));
    let e = format!("{:#}", plan(serde_json::json!({"map": "m", "horizontal": "lumaa"})).err().unwrap());
    assert!(e.contains("Did you mean \"luma\""), "{e}");
    let e = format!("{:#}", plan(serde_json::json!({"map": "m", "horizontal": 3})).err().unwrap());
    assert!(e.contains("one of its names"), "{e}");
}

#[test]
fn errors_point_at_the_authors_line() {
    let src = "// @effect broken\n// @param amount float 1\nhalf4 main(float2 xy) {\n    return half4(amount + nothing_here);\n}\n";
    let e = Def::compile(src, "t").err().unwrap().to_string();
    assert!(e.contains("line 4") && e.contains("nothing_here"), "{e}");
}

#[test]
fn headers_are_checked() {
    assert!(Def::compile("half4 main(float2 xy) { return half4(1); }", "t").is_err());
    let e = Def::compile("// @effect x\n// @param a floot 1\nhalf4 main(float2 xy) { return half4(1); }", "t").err().unwrap();
    assert!(format!("{e:#}").contains("float, vec2, color or choice"), "{e:#}");
    let e = Def::compile("// @effect blur\nhalf4 main(float2 xy) { return half4(time + resolution.x); }", "t").err().unwrap();
    assert!(e.to_string().contains("already a platform effect"), "{e}");
}

#[test]
fn uses_are_checked_against_the_definition() {
    let lib = Library::load(Path::new("/nonexistent")).unwrap();
    let ok = lib.plan("halftone", serde_json::json!({"size": 10}).as_object().unwrap()).unwrap();
    assert_eq!(ok.params.len(), lib.get("halftone").unwrap().params.len());
    let e = lib.plan("halftone", serde_json::json!({"sise": 10}).as_object().unwrap()).err().unwrap().to_string();
    assert!(e.contains("Did you mean \"size\""), "{e}");
    let e = lib.get("halftoen").err().unwrap().to_string();
    assert!(e.contains("Did you mean \"halftone\""), "{e}");
}

#[test]
fn a_composition_uses_builtin_and_project_plugins_in_one_chain() {
    use crate::{eval::{fx::FxOpState, Content, Evaluator}, plan::{Plan, ROOT}};
    let dir = std::env::temp_dir().join(format!("tm-fx-{}", std::process::id()));
    fs::create_dir_all(dir.join("effects")).unwrap();
    fs::write(dir.join("effects/tint2.sksl"), "// @effect tint2\n// @param amount float 0.5 0 1\nhalf4 main(float2 xy) { return image.eval(xy) * amount; }\n").unwrap();
    let v = serde_json::json!({"dur": 2, "layers": [
        {"type": "solid", "color": "#ffffff", "fx": [{"effect": "blur", "radius": 2}, {"effect": "tint2", "amount": {"from": 0, "to": 2, "dur": 2, "ease": "none"}}]},
        {"type": "texture", "shader": "noise", "params": {"scale": 50}}
    ]});
    let plan = Plan::from_value(v, &dir).unwrap();
    let f = Evaluator::new(&plan).frame(ROOT, 1.0).unwrap();
    assert!(matches!(f.layers[0].fx[0].op, FxOpState::Builtin));
    let FxOpState::Plugin(p) = &f.layers[0].fx[1].op else { panic!("second effect is the plugin") };
    // 0 -> 2 over two seconds is 1.0 at t=1, then clamped to the declared 0..1 range
    assert_eq!(p.values[0], ("amount".to_string(), vec![1.0]));
    assert!(matches!(f.layers[1].content, Content::Texture { .. }));
    let bad = serde_json::json!({"layers": [{"type": "solid", "color": "#fff", "fx": [{"effect": "tint2", "amout": 1}]}]});
    let e = format!("{:#}", Plan::from_value(bad, &dir).err().unwrap());
    assert!(e.contains("Did you mean \"amount\""), "{e}");
    let wrong = serde_json::json!({"layers": [{"type": "texture", "shader": "duotone"}]});
    assert!(format!("{:#}", Plan::from_value(wrong, &dir).err().unwrap()).contains("needs an input layer"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn composition_declared_shader_paths_are_used() {
    let dir = std::env::temp_dir().join(format!("tm-declared-fx-{}", std::process::id()));
    fs::create_dir_all(dir.join("shared")).unwrap();
    fs::write(dir.join("shared/declared.sksl"), "// @effect declared\n// @kind generator\n// @summary declared shader path\nhalf4 main(float2 xy) { return half4(1); }\n").unwrap();
    let value = serde_json::json!({"fx_path": ["shared"], "layers": [{"type": "texture", "shader": "declared"}]});
    let plan = crate::plan::Plan::from_value(value, &dir).unwrap();
    assert!(plan.fx.get("declared").is_ok());
    // relative to the composition's root, as "block_path" is
    let rooted = serde_json::json!({"root": "assets", "fx_path": ["../shared"], "layers": [{"type": "texture", "shader": "declared"}]});
    fs::create_dir_all(dir.join("assets")).unwrap();
    assert!(crate::plan::Plan::from_value(rooted, &dir).unwrap().fx.get("declared").is_ok());
    let invalid = serde_json::json!({"fx_path": ["missing"], "layers": []});
    let error = crate::plan::Plan::from_value(invalid, &dir).err().unwrap();
    assert!(error.to_string().contains("not a folder"), "{error:#}");
}

#[test]
fn space_declares_the_encoding_a_shader_computes_in_and_defaults_to_display() {
    let body = "half4 main(float2 xy) { return half4(time + resolution.x); }";
    let space = |header: &str| Def::compile(&format!("// @effect x\n{header}\n{body}"), "t").map(|d| d.descriptor().space);
    assert_eq!(space("").unwrap(), "display");
    assert_eq!(space("// @space display").unwrap(), "display");
    assert_eq!(space("// @space linear").unwrap(), "linear");
    let e = format!("{:#}", space("// @space lienar").unwrap_err());
    assert!(e.contains("@space is display or linear, not \"lienar\""), "{e}");
    let e = format!("{:#}", space("// @space linear\n// @space display").unwrap_err());
    assert!(e.contains("@space is given twice"), "{e}");
}

#[test]
fn every_platform_effect_and_the_shipped_pack_say_which_encoding_they_compute_in() {
    let lib = Library::builtin().unwrap();
    let space = |name: &str| lib.descriptors().into_iter().find(|d| d.name == name).unwrap_or_else(|| panic!("no effect {name}")).space;
    for light in ["blur", "glow", "shadow", "exposure", "opacity", "tilt_shift"] {
        assert_eq!(space(light), "linear", "{light} moves or spreads light");
    }
    for seen in ["contrast", "brightness", "hue", "tint", "grain", "bloom"] {
        assert_eq!(space(seen), "display", "{seen} is written against what the eye sees");
    }
}
