//! The runnable example of every shader plugin in a folder (`motions/lib/effects`): its `@example` header, or, where the
//! header has none, the use generated from its declared defaults (what the reference pages show for a plugin).
//! The film names the folder in `fx_path`, so the plugin resolves the way it does in a real project.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use turbomotion_comp::fxlib::Library;

use super::{fixtures, Declared, Example};

/// The shared plugin folder of this repository: `<repo>/motions/lib/effects`. `TURBOMOTION_EXAMPLE_FX_DIR` points the
/// test at another folder (a checkout of the engine alone has no such folder, and the test then fails by name).
pub fn shared_folder() -> PathBuf {
    std::env::var_os("TURBOMOTION_EXAMPLE_FX_DIR").map_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../motions/lib/effects"), PathBuf::from)
}

/// Every plugin in `dir` as an example film. A plugin that does not compile is an error, not a missing example.
pub fn collect(dir: &Path) -> Vec<Example> {
    let dir = std::fs::canonicalize(dir).unwrap_or_else(|e| panic!("the plugin folder {} is not readable: {e}", dir.display()));
    let lib = Library::load_with(&fixtures::folder(), &[dir.display().to_string()]).unwrap_or_else(|e| panic!("loading {}: {e:#}", dir.display()));
    assert!(lib.broken.is_empty(), "plugins that do not compile: {:?}", lib.broken);
    let origin = dir.display().to_string();
    let mine: Vec<Value> = lib.descriptors().into_iter().filter(|d| d.origin.starts_with(&origin)).map(|d| serde_json::to_value(d).unwrap()).collect();
    // the harness learns which inputs read another layer from the declarations, plugins' included
    let mut catalogue = turbomotion_comp::catalogue::all();
    catalogue["effects"].as_array_mut().unwrap().extend(mine.iter().cloned());
    let declared = Declared::from(&catalogue);
    mine.iter()
        .map(|d| {
            let name = format!("plugin {}", d["name"].as_str().unwrap());
            let mut film = declared.classify(&name, &d["example"]).unwrap_or_else(|e| panic!("{e}"));
            film["fx_path"] = json!([origin]);
            fixtures::ensure(&film);
            Example { name, film, allowed: vec![] }
        })
        .collect()
}
