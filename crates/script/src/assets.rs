//! QuickJS bridge for artwork: SVG import through the engine, and reading a text asset beside the film.
//! Every file read is recorded so `sdk watch` knows what to poll.
use std::{path::{Path, PathBuf}, sync::{Arc, Mutex}};
use rquickjs::{Ctx, Exception, Function, Result};

/// Files the script read through `svgFile`, shared with the runner.
pub type Reads = Arc<Mutex<Vec<PathBuf>>>;

pub fn install(ctx: &Ctx<'_>, film_dir: &Path, reads: Reads) -> Result<()> {
    let svg = Function::new(ctx.clone(), |ctx: Ctx<'_>, markup: String, id: String, fit: f64| -> Result<String> {
        let imported = turbomotion_comp::api::svg_layer(&markup, &id, fit).map_err(|e| Exception::throw_message(&ctx, &format!("svg(): {e:#}")))?;
        Ok(serde_json::json!({"layer": imported.layer, "warnings": imported.warnings}).to_string())
    })?;
    let base = film_dir.to_path_buf();
    let read = Function::new(ctx.clone(), move |ctx: Ctx<'_>, root: String, name: String| -> Result<String> {
        let fail = |m: String| Exception::throw_message(&ctx, &format!("svgFile(): {m}"));
        if Path::new(&name).is_absolute() { return Err(fail(format!("{name:?} is absolute; give a path relative to the film's folder (or its `root`)"))); }
        let path = base.join(&root).join(&name);
        let text = std::fs::read_to_string(&path).map_err(|e| fail(format!("cannot read {}: {e}", path.display())))?;
        reads.lock().unwrap().push(std::fs::canonicalize(&path).unwrap_or(path));
        Ok(text)
    })?;
    ctx.globals().set("__tm_svg", svg)?;
    ctx.globals().set("__tm_read", read)
}
