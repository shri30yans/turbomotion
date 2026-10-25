//! QuickJS bridge to the renderer's text shaper. JSON keeps the host boundary typed and validated.
use std::{collections::BTreeMap, path::Path};
use rquickjs::{Ctx, Exception, Function, Result};
use turbomotion_comp::text::{layout::{Engine, Fonts}, measure::{measure, Request}};

pub fn install(ctx: &Ctx<'_>, film_dir: &Path) -> Result<()> {
    let root = film_dir.to_path_buf();
    let engine = Engine::new();
    let host = Function::new(ctx.clone(), move |ctx: Ctx<'_>, request: String, fonts: String, relative_root: String| -> Result<String> {
        let result = (|| -> anyhow::Result<String> {
            let request: Request = serde_json::from_str(&request)?;
            let roles: BTreeMap<String, String> = serde_json::from_str(&fonts)?;
            let book = Fonts { roles, root: root.join(relative_root) };
            Ok(serde_json::to_string(&measure(&engine, &book, &request)?)?)
        })();
        result.map_err(|e| Exception::throw_message(&ctx, &format!("measure(): {e:#}")))
    })?;
    ctx.globals().set("__tm_measure", host)
}
