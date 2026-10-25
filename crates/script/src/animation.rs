//! Native animation vocabulary shared by authoring and rendering.
use rquickjs::{Ctx, Exception, Function, Result};
use turbomotion_comp::anim::{ease::Ease, prop::parse_color_static};
pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    let ease = Function::new(ctx.clone(), |ctx: Ctx<'_>, name: String, progress: f64| -> Result<f64> {
        Ease::parse(&name).map(|ease| ease.at(progress.clamp(0.0, 1.0)))
            .map_err(|error| Exception::throw_message(&ctx, &format!("{error:#}")))
    })?;
    let color = Function::new(ctx.clone(), |ctx: Ctx<'_>, source: String| -> Result<String> {
        let result = (|| -> anyhow::Result<String> {
            Ok(serde_json::to_string(&parse_color_static(&serde_json::from_str(&source)?)?)?)
        })();
        result.map_err(|error| Exception::throw_message(&ctx, &format!("{error:#}")))
    })?;
    ctx.globals().set("__tm_ease", ease)?;
    ctx.globals().set("__tm_color", color)
}
