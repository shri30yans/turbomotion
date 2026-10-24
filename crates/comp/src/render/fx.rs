//! Running effect chains: platform effects through the effects library, plugins through their compiled shader, in the
//! order the author wrote them.

use anyhow::{Context, Result};
use skia_safe::{Canvas, Paint, Rect, Shader, TileMode};
use turbomotion_core::{effects, layer::Layer};

use super::Ctx;
use crate::{
    eval::fx::{FxState, PluginState},
    fxlib::PlanFx,
};

/// `img` after every effect in `fx`, with the evaluated plugin values in `states` (same order). `t` is the clock the
/// effects see: the layer's own time, or the composition's for a `look`.
pub fn chain(ctx: &Ctx, like: &Canvas, mut img: Layer, fx: &[PlanFx], states: &[FxState], t: f64) -> Result<Layer> {
    let mut i = 0;
    while i < fx.len() {
        match (&fx[i], &states[i]) {
            (PlanFx::Plugin(_), FxState::Plugin(p)) => {
                img = plugin(ctx, like, &img, p, t).with_context(|| format!("effect {:?}", p.name))?;
                i += 1;
            }
            _ => {
                let mut run = vec![];
                while let Some(PlanFx::Builtin(e)) = fx.get(i) {
                    run.push(e.clone());
                    i += 1;
                }
                img = effects::apply(ctx.shaders, like, img, &run, t)?;
            }
        }
    }
    Ok(img)
}

fn instance(ctx: &Ctx, p: &PluginState, w: f32, h: f32, t: f64, children: &[Shader]) -> Result<Shader> {
    let def = ctx.plan.fx.get(&p.name)?;
    let mut values: Vec<(&str, Vec<f32>)> = p.values.iter().map(|(n, v)| (n.as_str(), v.clone())).collect();
    values.push(("resolution", vec![w, h]));
    values.push(("time", vec![t as f32]));
    let refs: Vec<(&str, &[f32])> = values.iter().map(|(n, v)| (*n, v.as_slice())).collect();
    def.program.instance_named(&refs, children)
}

/// An effect plugin applied to a layer image.
pub fn plugin(ctx: &Ctx, like: &Canvas, img: &Layer, p: &PluginState, t: f64) -> Result<Layer> {
    let (w, h) = (img.width(), img.height());
    let input = img.image().to_shader((TileMode::Clamp, TileMode::Clamp), turbomotion_core::layer::linear(), None).context("the layer image has no shader form")?;
    let shader = instance(ctx, p, w as f32, h as f32, t, &[input])?;
    Layer::try_render(like, w, h, |c| {
        let mut paint = Paint::default();
        paint.set_shader(shader);
        c.draw_rect(Rect::from_wh(w as f32, h as f32), &paint);
        Ok(())
    })
}

/// A generator plugin painted into `size` of the current canvas (the caller has set the layer's matrix).
pub fn generate(ctx: &Ctx, c: &Canvas, p: &PluginState, size: [f64; 2], t: f64) -> Result<()> {
    let shader = instance(ctx, p, size[0] as f32, size[1] as f32, t, &[])?;
    let mut paint = Paint::default();
    paint.set_shader(shader);
    c.draw_rect(Rect::from_wh(size[0] as f32, size[1] as f32), &paint);
    Ok(())
}
