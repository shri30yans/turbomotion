//! Module resolution for QuickJS: `"turbomotion"` is the SDK embedded in this binary, `./x` is a file beside the
//! importing script. Every module is type-stripped on its way in.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use rquickjs::{loader::{Loader, Resolver}, module::Declared, Ctx, Error, Module, Result};

use crate::strip::{strip, Stripped};

/// The SDK, compiled into the binary. `index.ts` is what `import ... from "turbomotion"` reaches.
pub const SDK: &[(&str, &str)] = &[
    ("index", include_str!("../sdk/index.ts")),
    ("tween-types", include_str!("../sdk/tween-types.ts")),
    ("overwrite", include_str!("../sdk/overwrite.ts")),
    ("film-types", include_str!("../sdk/film-types.ts")),
    ("types", include_str!("../sdk/types.ts")),
    ("measure", include_str!("../sdk/measure.ts")),
    ("site", include_str!("../sdk/site.ts")),
    ("relative", include_str!("../sdk/relative.ts")),
    ("item-tweens", include_str!("../sdk/item-tweens.ts")),
    ("vector-tweens", include_str!("../sdk/vector-tweens.ts")),
    ("image-check", include_str!("../sdk/image-check.ts")),
    ("assets", include_str!("../sdk/assets.ts")),
    ("motion", include_str!("../sdk/motion.ts")),
    ("layers", include_str!("../sdk/layers.ts")),
    ("text-fx", include_str!("../sdk/text-fx.ts")),
    ("three", include_str!("../sdk/three.ts")),
    ("timeline", include_str!("../sdk/timeline.ts")),
    ("comp", include_str!("../sdk/comp.ts")),
    ("sound", include_str!("../sdk/sound.ts")),
    ("stage", include_str!("../sdk/stage.ts")),
    ("components/index", include_str!("../sdk/components/index.ts")),
    ("components/util", include_str!("../sdk/components/util.ts")),
    ("components/promptBox", include_str!("../sdk/components/promptBox.ts")),
    ("components/codePanel", include_str!("../sdk/components/codePanel.ts")),
    ("components/browserWindow", include_str!("../sdk/components/browserWindow.ts")),
    ("components/statCounter", include_str!("../sdk/components/statCounter.ts")),
    ("components/barRace", include_str!("../sdk/components/barRace.ts")),
    ("components/bboxOverlay", include_str!("../sdk/components/bboxOverlay.ts")),
    ("components/cardWall", include_str!("../sdk/components/cardWall.ts")),
    ("components/title3d", include_str!("../sdk/components/title3d.ts")),
    ("components/scribble", include_str!("../sdk/components/scribble.ts")),
    ("components/waveform", include_str!("../sdk/components/waveform.ts")),
    ("components/textStack", include_str!("../sdk/components/textStack.ts")),
    ("components/cutout", include_str!("../sdk/components/cutout.ts")),
    ("components/appear", include_str!("../sdk/components/appear.ts")),
    ("components/cube", include_str!("../sdk/components/cube.ts")),
    ("components/swarm", include_str!("../sdk/components/swarm.ts")),
    ("components/flipTile", include_str!("../sdk/components/flipTile.ts")),
    ("components/capsule", include_str!("../sdk/components/capsule.ts")),
    ("components/hand", include_str!("../sdk/components/hand.ts")),
    ("components/lens", include_str!("../sdk/components/lens.ts")),
    ("components/blob", include_str!("../sdk/components/blob.ts")),
    ("components/montage", include_str!("../sdk/components/montage.ts")),
    ("components/dollyZoom", include_str!("../sdk/components/dollyZoom.ts")),
    ("components/glassPanel", include_str!("../sdk/components/glassPanel.ts")),
];

const SDK_PREFIX: &str = "sdk:";

/// What the loader learned, shared with the runner so errors can be mapped back to the author's lines.
#[derive(Default)]
pub struct Registry {
    pub modules: HashMap<String, Stripped>,
    /// The first failure to strip a module (the engine's own message for it is terse).
    pub failure: Option<String>,
}

pub type Shared = Arc<Mutex<Registry>>;

/// Name a module the way errors should show it: SDK modules as `turbomotion/comp.ts`, files as given.
pub fn display(name: &str) -> String {
    match name.strip_prefix(SDK_PREFIX) {
        Some(m) => format!("turbomotion/{m}.ts"),
        None => name.to_string(),
    }
}

pub struct Resolve;

impl Resolver for Resolve {
    fn resolve<'js>(&mut self, _ctx: &Ctx<'js>, base: &str, name: &str, _attrs: Option<rquickjs::loader::ImportAttributes<'js>>) -> Result<String> {
        if name == "turbomotion" || name == "turbomotion/index" {
            return Ok(format!("{SDK_PREFIX}index"));
        }
        if let Some(rest) = name.strip_prefix("turbomotion/") {
            return Ok(format!("{SDK_PREFIX}{}", rest.trim_end_matches(".ts")));
        }
        if let (Some(module), true) = (base.strip_prefix(SDK_PREFIX), name.starts_with("./") || name.starts_with("../")) {
            // relative to the importing SDK module's folder (`components/`), so components can reach `../layers`
            let dir = Path::new(module).parent().unwrap_or(Path::new(""));
            let joined = normalise(&dir.join(name));
            return Ok(format!("{SDK_PREFIX}{}", joined.to_string_lossy().trim_end_matches(".ts")));
        }
        if name.starts_with("./") || name.starts_with("../") || name.starts_with('/') {
            let dir = Path::new(base).parent().unwrap_or(Path::new("."));
            return Ok(with_extension(&normalise(&dir.join(name))).to_string_lossy().into_owned());
        }
        Err(Error::new_resolving_message(base, name, format!("cannot find module \"{name}\": import from \"turbomotion\" or a relative path like \"./scenes\"")))
    }
}

fn with_extension(p: &Path) -> PathBuf {
    if p.extension().is_some() {
        p.to_path_buf()
    } else {
        p.with_extension("ts")
    }
}

/// Collapse `.` and `..` without touching the disk (module names must be stable keys).
fn normalise(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

pub struct Load {
    pub registry: Shared,
    pub sdk_dir: Option<PathBuf>,
}

impl Loader for Load {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str, _attrs: Option<rquickjs::loader::ImportAttributes<'js>>) -> Result<Module<'js, Declared>> {
        let (source, label) = match name.strip_prefix(SDK_PREFIX) {
            Some(m) => {
                if Path::new(m).components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
                    return Err(Error::new_loading_message(name, "SDK imports must stay inside the SDK directory"));
                }
                if let Some(dir) = &self.sdk_dir {
                    let path = dir.join(m).with_extension("ts");
                    let source = std::fs::read_to_string(&path).map_err(|e| Error::new_loading_message(name,
                        format!("check --sdk-dir and its imports: cannot read SDK module {}: {e}", path.display())))?;
                    (source, path.to_string_lossy().into_owned())
                } else {
                let found = SDK.iter().find(|(n, _)| *n == m).ok_or_else(|| Error::new_loading_message(name, format!("the SDK has no module \"{m}\"")))?;
                (found.1.to_string(), format!("{m}.ts"))
                }
            }
            None => (std::fs::read_to_string(name).map_err(|e| Error::new_loading_message(name, format!("cannot read {name}: {e}")))?, name.to_string()),
        };
        match strip(&label, &source) {
            Ok(done) => {
                let js = done.js.clone();
                self.registry.lock().unwrap().modules.insert(name.to_string(), done);
                Module::declare(ctx.clone(), name, js)
            }
            Err(e) => {
                let msg = format!("{e}");
                self.registry.lock().unwrap().failure.get_or_insert(msg.clone());
                Err(Error::new_loading_message(name, msg))
            }
        }
    }
}

/// The SDK folder named with `--sdk-dir`, if any. Without one the embedded SDK is used; there is no probing of
/// folders beside the binary. An explicit override never silently falls back to the embedded modules.
pub fn sdk_directory(explicit: Option<&Path>) -> anyhow::Result<Option<PathBuf>> {
    if let Some(dir) = explicit {
        anyhow::ensure!(dir.join("index.ts").is_file(), "SDK directory {} needs index.ts; pass --sdk-dir <sdk-folder>", dir.display());
        return Ok(Some(std::fs::canonicalize(dir)?));
    }
    Ok(None)
}
