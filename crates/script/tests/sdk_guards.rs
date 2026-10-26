//! A wrong guess at a layer is refused where it is written, naming what was meant: unknown keys (checked against the
//! engine's own key lists), assigning an option as a field, `texture()`, and a matte that cannot work.
mod common;
use std::fs;
use turbomotion_script::{build, Options};

fn film(name: &str, body: &str) -> std::path::PathBuf {
    let dir = common::scratch("sdk-guards", name);
    let path = dir.join("film.ts");
    fs::write(&path, format!("import {{ comp, shot, solid, text, texture, group, raw, shape, rect }} from 'turbomotion';\nconst film = comp({{ width: 320, height: 180, fps: 30, dur: 1 }});\n{body}\nexport default film;")).unwrap();
    path
}

fn error_of(name: &str, body: &str) -> String {
    format!("{:#}", build(&film(name, body), &Options::default()).unwrap_err())
}

fn built(name: &str, body: &str) -> serde_json::Value {
    build(&film(name, body), &Options::default()).unwrap_or_else(|e| panic!("{name}: {e:#}"))
}

#[test]
fn an_unknown_layer_key_throws_at_its_line_with_the_nearest_valid_key() {
    let e = error_of("colour", "film.add(solid('#000'));\nfilm.add(text('hi', { id: 'hi', colour: '#fff' } as any));");
    assert!(e.contains("unknown key \"colour\". Did you mean \"color\"?") && e.contains("text layer \"hi\""), "{e}");
    assert!(e.contains("film.ts:4:"), "the error points at the author's line: {e}");
    // the keys of every kind are the engine's: a solid has `size`, no `radius`
    assert!(error_of("solid-key", "film.add(solid('#000', { radius: 8 } as any));").contains("A solid layer takes color, size"));
    // a misspelt key on a later `set()` is refused the same way
    let e = error_of("set-key", "film.add(solid('#000', { id: 's' }).set({ opacty: 0.5 }));");
    assert!(e.contains("unknown key \"opacty\". Did you mean \"opacity\"?"), "{e}");
}

#[test]
fn text_italic_is_a_key_and_outline_is_outline_only() {
    let ok = built("italic", "film.add(text('hi', { id: 'hi', italic: true, outline_only: true, stroke: { color: '#fff', width: 2 } }));");
    assert_eq!(ok["layers"][0]["italic"], true);
    assert_eq!(ok["layers"][0]["outline_only"], true);
    let e = error_of("outline", "film.add(text('hi', { id: 'hi', outline: true } as any));");
    assert!(e.contains("unknown key \"outline\"") && e.contains("outline_only"), "{e}");
    assert!(error_of("font-style", "film.add(text('hi', { fontStyle: 'italic' } as any));").contains("unknown key \"fontStyle\""));
}

#[test]
fn every_key_the_sdk_writes_itself_is_a_valid_key() {
    // position shorthand, repeat, extrude, parents, raw kinds: all pass through the same check
    let ok = built(
        "valid",
        "const g = group({ id: 'rig', x: 160, y: 90 }, [rect(40, 40, { id: 'box', fill: '#f00', radius: 4 })]);\n\
         film.add(g, solid('#00f', { id: 'slab', size: [40, 40], repeat: { count: 3, offset: [10, 0] } }), \
         text('88', { id: 'deep', size: 80, three_d: true, pos: [160, 90, 0], extrude: { depth: 10 } }), \
         raw({ type: 'adjustment', id: 'grade', fx: [{ effect: 'vignette' }] }), \
         texture('noise', { id: 'n', params: { scale: 0.01 }, size: [320, 180] }));",
    );
    assert_eq!(ok["layers"].as_array().unwrap().len(), 6);
    assert_eq!(ok["layers"][0]["pos"], serde_json::json!([160, 90]));
}

#[test]
fn an_option_assigned_as_a_field_after_construction_is_refused_with_the_place_that_works() {
    let e = error_of("effects", "const t = text('hi', { id: 'hi' });\n(t as any).effects = [{ effect: 'blur', radius: 4 }];\nfilm.add(t);");
    assert!(e.contains("layer.effects = ... has no effect") && e.contains("\"fx\"") && e.contains("layer.set({ fx: [...] })"), "{e}");
    let e = error_of("opacity", "const s = solid('#000', { id: 's' });\n(s as any).opacity = 0.5;\nfilm.add(s);");
    assert!(e.contains("solid layer \"s\": layer.opacity = ... has no effect") && e.contains("layer.set({ opacity: ... })"), "{e}");
    // what does work: `fx` in the options and `set()` later; builder state (`id`, `children`) stays assignable
    let ok = built("fx", "const t = text('hi', { id: 'hi', fx: [{ effect: 'blur', radius: 2 }] });\nt.set({ opacity: 0.5 });\nfilm.add(t);");
    assert_eq!(ok["layers"][0]["fx"][0]["effect"], "blur");
    assert_eq!(ok["layers"][0]["opacity"], 0.5);
}

#[test]
fn texture_takes_the_generator_name_first_and_its_params_as_an_object() {
    let ok = built("texture", "film.add(texture('noise', { id: 'plate', params: { scale: 0.02 }, size: [320, 180], opacity: 0.5 }));");
    assert_eq!(ok["layers"][0]["type"], "texture");
    assert_eq!(ok["layers"][0]["shader"], "noise");
    assert_eq!(ok["layers"][0]["params"]["scale"], 0.02);
    let e = error_of("texture-object", "film.add(texture({ shader: 'noise' } as any));");
    assert!(e.contains("texture(shader, opts)") && e.contains("first argument, not an option"), "{e}");
    assert!(error_of("texture-number", "film.add(texture(5 as any));").contains("got number 5"));
    // a typo in the layer options is caught like on any layer
    assert!(error_of("texture-key", "film.add(texture('noise', { param: {} } as any));").contains("Did you mean \"params\"?"));
}

#[test]
fn a_matte_taken_from_a_hidden_layer_on_an_adjustment_layer_is_an_error() {
    let hidden = "film.add(solid('#f00', { id: 'plate', size: [320, 180] }), solid('#fff', { id: 'hole', size: [100, 100], hidden: true }), raw({ type: 'adjustment', id: 'grade', fx: [{ effect: 'invert' }], matte: { source: 'hole' } }));";
    let e = error_of("matte-hidden", hidden);
    assert!(e.contains("adjustment layer \"grade\" takes its matte from \"hole\", which is hidden") && e.contains("\"masks\""), "{e}");
    // the same inside a shot
    let e = error_of("matte-shot", "film.timeline({ shots: [shot({ id: 's', dur: 1, layers: [solid('#fff', { id: 'hole', hidden: true }), raw({ type: 'adjustment', id: 'g', matte: { source: 'hole' } })] })] });");
    assert!(e.contains("shot \"s\": adjustment layer \"g\""), "{e}");
    // masks are the way, and a matte on an ordinary layer is untouched
    let ok = built(
        "matte-fine",
        "film.add(solid('#f00', { id: 'plate', size: [320, 180] }), solid('#fff', { id: 'hole', size: [100, 100], hidden: true }), \
         solid('#00f', { id: 'blue', matte: { source: 'hole' } }), \
         raw({ type: 'adjustment', id: 'grade', fx: [{ effect: 'invert' }], masks: [{ path: { rect: [110, 40, 100, 100] } }] }));",
    );
    assert_eq!(ok["layers"][3]["masks"][0]["path"]["rect"][2], 100);
}
