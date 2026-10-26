//! Running effect chains: platform effects through the effects library, plugins through their compiled passes, in the
//! order the author wrote them. A plugin may read other layers' pictures (`@input name layer`) and its own earlier
//! output (`@input name history=N`); both are resolved here, at the moment being drawn.

use anyhow::{anyhow, bail, ensure, Context, Result};
use skia_safe::{Canvas, Image, Matrix, Paint, Rect, Shader, TileMode};
use turbomotion_core::{effects::{self, backdrop_over, Effect}, layer::Layer};

use super::{composite, history, Ctx};
use crate::{
    eval::{
        fx::{FxOpState, FxState, PluginState},
        Frame,
    },
    fxlib::{is_full_size, Def, Fit, FxOp, PlanFx, PluginPlan},
};

/// Where an effect chain runs: the frame it is drawn for, the layer it belongs to (none for a composition's `look`) and
/// the pictures of the layers its effects read, which the frame graph rendered first.
pub struct Scope<'a> {
    pub ctx: &'a Ctx<'a>,
    pub like: &'a Canvas,
    pub frame: &'a Frame,
    pub layer: Option<usize>,
    /// What has been composited below this layer, when an effect needs it (`backdrop`).
    pub behind: Option<&'a Layer>,
    pub pictures: &'a [(usize, &'a Layer)],
}

impl Scope<'_> {
    /// The picture of layer `index`, which the frame graph drew for the effects that read it.
    pub fn picture(&self, index: usize) -> Result<&Layer> {
        self.pictures.iter().find(|(layer, _)| *layer == index).map(|(_, picture)| *picture).ok_or_else(|| anyhow!("the frame graph did not draw layer {index} for an effect that reads it"))
    }
}

/// `img` after every effect in `fx`, with the evaluated plugin values in `states` (same order). `t` is the clock the
/// effects see: the layer's own time, or the composition's for a `look`.
pub fn chain(sc: &Scope, mut img: Layer, fx: &[PlanFx], states: &[FxState], t: f64) -> Result<Layer> {
    for (at, (f, state)) in fx.iter().zip(states).enumerate() {
        let input = img.clone();
        let result = match (&f.op, &state.op) {
            (FxOp::Plugin(plan), FxOpState::Plugin(p)) => plugin(sc, &img, plan, p, t, at).with_context(|| format!("effect {:?}", p.name))?,
            (FxOp::Builtin(Effect::Backdrop(e)), FxOpState::Builtin) => {
                let behind = sc.behind.ok_or_else(|| anyhow!("effect {:?} needs a picture of what is behind the layer", f.name()))?;
                backdrop_over(sc.like, &img, behind, e.blur.at(t))?
            }
            (FxOp::Builtin(e), FxOpState::Builtin) => effects::apply(sc.ctx.shaders, sc.like, img, std::slice::from_ref(e), t)?,
            _ => bail!("effect {:?} was evaluated as a different kind of effect than the plan has", f.name()),
        };
        img = match (&f.composite, &state.composite) {
            (Some(plan), Some(state)) => composite::apply(sc, &input, result, plan, state).with_context(|| format!("effect {:?} composite", f.name()))?,
            _ => result,
        };
    }
    Ok(img)
}

/// The values every plugin pass is given: its parameters, the pass size and the clock.
fn values(p: &PluginState, w: f32, h: f32, t: f64) -> Vec<(String, Vec<f32>)> {
    let mut values: Vec<(String, Vec<f32>)> = p.values.clone();
    values.push(("resolution".to_string(), vec![w, h]));
    values.push(("time".to_string(), vec![t as f32]));
    values
}

/// An image that a pass samples, and where it sits relative to the layer the effect is on.
struct Source {
    name: String,
    image: Image,
    place: Matrix,
}

/// An image as a shader, sampled bilinearly with no mipmaps. A pass smaller than the picture it reads therefore skips
/// pixels instead of averaging them: a pass at `scale=0.5` over a full-size picture is fine, one far below that
/// aliases fine detail, so a blur that wants a small pass should halve in steps (one `@pass` per halving).
fn shader_of(image: &Image, m: &Matrix) -> Result<Shader> {
    image.to_shader((TileMode::Clamp, TileMode::Clamp), turbomotion_core::layer::linear(), Some(m)).context("a picture has no shader form")
}

/// The box a layer occupies on screen (the whole frame for layers that have none and for a `look`).
fn screen_box(frame: &Frame, layer: Option<usize>) -> [f64; 4] {
    layer.and_then(|l| frame.layers[l].bounds).unwrap_or([0.0, 0.0, f64::from(frame.width), f64::from(frame.height)])
}

/// A box narrower or shorter than this (pixels) has nothing a map could be fitted to.
const MIN_EXTENT: f64 = 0.5;

/// A box's width and height, which must be real for a map to be fitted to it.
fn extent(b: [f64; 4], what: &str) -> Result<(f64, f64)> {
    let (w, h) = (b[2] - b[0], b[3] - b[1]);
    ensure!(w > MIN_EXTENT && h > MIN_EXTENT, "the {what} has no size ({w:.0}x{h:.0}), so a map cannot be fitted to it");
    Ok((w, h))
}

/// The matrix that carries a map's picture onto the effect layer: centred on it, stretched to its box, or left in place.
fn place(map: [f64; 4], on: [f64; 4], fit: Fit) -> Result<Matrix> {
    Ok(match fit {
        Fit::Screen => Matrix::new_identity(),
        Fit::Center => {
            extent(map, "map layer")?;
            extent(on, "effect layer")?;
            Matrix::translate(((((on[0] + on[2]) - (map[0] + map[2])) / 2.0) as f32, (((on[1] + on[3]) - (map[1] + map[3])) / 2.0) as f32))
        }
        Fit::Stretch => {
            let ((mw, mh), (lw, lh)) = (extent(map, "map layer")?, extent(on, "effect layer")?);
            let (sx, sy) = (lw / mw, lh / mh);
            Matrix::new_all(sx as f32, 0.0, (on[0] - map[0] * sx) as f32, 0.0, sy as f32, (on[1] - map[1] * sy) as f32, 0.0, 0.0, 1.0)
        }
    })
}

/// The pictures of the layers an effect use names, ready to be sampled.
fn layer_inputs(sc: &Scope, plan: &PluginPlan) -> Result<Vec<Source>> {
    plan.inputs
        .iter()
        .map(|ip| {
            let index = ip.index.ok_or_else(|| anyhow!("input {:?}: layer {:?} was not resolved when the plan was made", ip.name, ip.id))?;
            let map = sc.picture(index)?;
            let from = screen_box(sc.frame, Some(index));
            let m = place(from, screen_box(sc.frame, sc.layer), ip.fit).with_context(|| format!("input {:?}: layer {:?}", ip.name, ip.id))?;
            Ok(Source { name: ip.name.clone(), image: map.image().clone(), place: m })
        })
        .collect()
}

/// Run every pass of `def` over `sources` (`image` first); the last pass's picture is the effect's output.
fn run_passes(sc: &Scope, def: &Def, p: &PluginState, w: i32, h: i32, t: f64, sources: &[Source]) -> Result<Layer> {
    let mut done: Vec<(String, Layer)> = Vec::with_capacity(def.passes.len());
    for pass in &def.passes {
        let (pw, ph) = if is_full_size(pass.scale) { (w, h) } else { (((f64::from(w) * pass.scale).ceil() as i32).max(1), ((f64::from(h) * pass.scale).ceil() as i32).max(1)) };
        let to_pass = Matrix::scale((pw as f32 / w as f32, ph as f32 / h as f32));
        let mut kids: Vec<(String, Shader)> = Vec::with_capacity(sources.len() + done.len());
        for s in sources {
            kids.push((s.name.clone(), shader_of(&s.image, &Matrix::concat(&to_pass, &s.place))?));
        }
        for (n, l) in &done {
            let m = Matrix::scale((pw as f32 / l.width() as f32, ph as f32 / l.height() as f32));
            kids.push((n.clone(), shader_of(l.image(), &m)?));
        }
        let vals = values(p, pw as f32, ph as f32, t);
        let refs: Vec<(&str, &[f32])> = vals.iter().map(|(n, v)| (n.as_str(), v.as_slice())).collect();
        let kid_refs: Vec<(&str, Shader)> = kids.iter().map(|(n, s)| (n.as_str(), s.clone())).collect();
        let shader = pass.program.instance_with(&refs, &kid_refs).with_context(|| format!("pass {:?}", pass.name))?;
        let out = Layer::try_render(sc.like, pw, ph, |c| {
            let mut paint = Paint::default();
            paint.set_shader(shader);
            c.draw_rect(Rect::from_wh(pw as f32, ph as f32), &paint);
            Ok(())
        })?;
        done.push((pass.name.clone(), out));
    }
    Ok(done.pop().expect("an effect has at least one pass").1)
}

/// Effect number `at` of the layer's list, a plugin, applied to `img`; its history input, if it has one, is drawn first.
pub fn plugin(sc: &Scope, img: &Layer, plan: &PluginPlan, p: &PluginState, t: f64, at: usize) -> Result<Layer> {
    let def = sc.ctx.plan.fx.get(&p.name)?;
    let past = match def.history() {
        Some((_, frames, depth)) => {
            let layer = sc.layer.ok_or_else(|| anyhow!("effect {:?} reads its own past, which only a layer's effects can (the plan rejects the rest)", p.name))?;
            history::previous(sc.ctx, sc.like, sc.frame, img, history::Spec { layer, effect: at, frames, depth })?
        }
        None => vec![],
    };
    plugin_with(sc, img, plan, p, t, past)
}

/// A plugin applied to `img` given the pictures of its history input (empty when it has none).
pub fn plugin_with(sc: &Scope, img: &Layer, plan: &PluginPlan, p: &PluginState, t: f64, past: Vec<Layer>) -> Result<Layer> {
    let def = sc.ctx.plan.fx.get(&p.name)?;
    let (w, h) = (img.width(), img.height());
    let mut sources = vec![Source { name: "image".to_string(), image: img.image().clone(), place: Matrix::new_identity() }];
    sources.extend(layer_inputs(sc, plan)?);
    if let Some((decl, ..)) = def.history() {
        sources.extend(decl.children().into_iter().zip(past).map(|(name, l)| Source { name, image: l.image().clone(), place: Matrix::new_identity() }));
    }
    run_passes(sc, def, p, w, h, t, &sources)
}

/// A generator plugin painted into `size` of the current canvas (the caller has set the layer's matrix).
pub fn generate(ctx: &Ctx, c: &Canvas, p: &PluginState, size: [f64; 2], t: f64) -> Result<()> {
    let def = ctx.plan.fx.get(&p.name)?;
    let vals = values(p, size[0] as f32, size[1] as f32, t);
    let refs: Vec<(&str, &[f32])> = vals.iter().map(|(n, v)| (n.as_str(), v.as_slice())).collect();
    let shader = def.passes[0].program.instance_with(&refs, &[])?;
    let mut paint = Paint::default();
    paint.set_shader(shader);
    c.draw_rect(Rect::from_wh(size[0] as f32, size[1] as f32), &paint);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: [f64; 4] = [0.0, 0.0, 40.0, 20.0];
    const ON: [f64; 4] = [100.0, 50.0, 300.0, 250.0];

    /// Where the map's point `(x, y)` lands on the effect layer.
    fn landing(fit: Fit, x: f32, y: f32) -> (f32, f32) {
        let p = place(MAP, ON, fit).expect("a fit").map_point((x, y));
        (p.x, p.y)
    }

    #[test]
    fn centre_puts_the_middles_together_and_keeps_the_size() {
        assert_eq!(landing(Fit::Center, 20.0, 10.0), (200.0, 150.0));
        assert_eq!(landing(Fit::Center, 21.0, 10.0), (201.0, 150.0));
    }

    #[test]
    fn stretch_maps_corner_to_corner() {
        assert_eq!(landing(Fit::Stretch, 0.0, 0.0), (100.0, 50.0));
        assert_eq!(landing(Fit::Stretch, 40.0, 20.0), (300.0, 250.0));
    }

    #[test]
    fn screen_leaves_the_map_where_it_is() {
        assert_eq!(landing(Fit::Screen, 7.0, 9.0), (7.0, 9.0));
    }

    #[test]
    fn a_layer_without_a_size_cannot_be_fitted() {
        let e = place([5.0, 5.0, 5.0, 5.0], ON, Fit::Stretch).err().expect("rejected").to_string();
        assert!(e.contains("map layer has no size"), "{e}");
        assert!(place(MAP, [0.0, 0.0, 0.0, 10.0], Fit::Center).is_err());
    }
}
