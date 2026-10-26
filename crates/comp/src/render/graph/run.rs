//! Running a graph: each pass in order, each image dropped after its last reader.

use anyhow::{Context, Result};
use skia_safe::{AlphaType, Canvas, Color4f, ColorType, ISize, ImageInfo, Paint, Surface};
use turbomotion_core::layer::Layer;

use super::{comp_graph, effect_inputs_graph, layer_graph, FxOwner, Graph, ImageId, Pass, Stage, Write};
use crate::{
    eval::Frame,
    render::{color4f, fx, local, post, Ctx},
};

struct Run<'a, 'c> {
    ctx: &'a Ctx<'c>,
    like: &'a Canvas,
    graph: &'a Graph<'a>,
    /// A finished picture per image, empty before it is written and after its last reader.
    slots: Vec<Option<Layer>>,
    /// The composite, once the backdrop pass has made it.
    composite: Option<Surface>,
}

impl Run<'_, '_> {
    fn picture(&self, id: ImageId) -> &Layer {
        self.slots[id.0].as_ref().expect("the builder emits every writer before its readers and a reader count keeps an image alive until its last read")
    }

    fn composite(&mut self) -> &mut Surface {
        self.composite.as_mut().expect("the backdrop is the first pass that touches the composite")
    }

    fn exec(&mut self, pass: &Pass) -> Result<Option<Layer>> {
        let (ctx, like, graph) = (self.ctx, self.like, self.graph);
        let (frame, comp) = (graph.frames[pass.frame], graph.comp);
        let (w, h) = (frame.width as i32, frame.height as i32);
        let read = |k: usize| self.picture(pass.reads[k]);
        Ok(match &pass.stage {
            Stage::Backdrop => {
                let info = ImageInfo::new(ISize::new(w, h), ColorType::RGBAF16, AlphaType::Premul, None);
                let mut acc = like.new_surface(&info, None).with_context(|| format!("no GPU memory for a {w}x{h} composite"))?;
                match frame.bg {
                    Some(bg) => acc.canvas().clear(premul(color4f(bg))),
                    None => acc.canvas().clear(skia_safe::Color::TRANSPARENT),
                };
                self.composite = Some(acc);
                None
            }
            Stage::Paint { layer } => Some(post::paint_stage(ctx, like, frame, comp, *layer).with_context(|| comp.layers[*layer].label.clone())?),
            Stage::PaintLocal { layer } => Some(local::paint(ctx, like, frame, comp, *layer).with_context(|| comp.layers[*layer].label.clone())?),
            Stage::MaskCoverageLocal { layer } => Some(local::mask_coverage(like, comp, &frame.layers[*layer])?),
            Stage::Place { layer } => Some(local::place(like, frame, &frame.layers[*layer], read(0))?),
            Stage::Finish { layer } => Some(post::finish(ctx, like, frame, &frame.layers[*layer], read(0).clone())?),
            Stage::Effects { owner, upto, pictures, behind } => {
                let mut at = 1;
                let behind_layer = if *behind {
                    let b = read(at);
                    at += 1;
                    Some(b)
                } else {
                    None
                };
                let maps: Vec<(usize, &Layer)> = pictures.iter().zip(&pass.reads[at..]).map(|(&l, &id)| (l, self.picture(id))).collect();
                Some(self.effects(frame, *owner, *upto, read(0).clone(), behind_layer, &maps)?)
            }
            Stage::MaskCoverage { layer } => Some(post::mask_coverage(like, frame, comp, &frame.layers[*layer])?),
            Stage::MatteCoverage { mode, opacity } => Some(post::matte_coverage(like, frame, read(0), *mode, *opacity)?),
            Stage::Multiply => Some(post::multiply_by(like, read(0), read(1))?),
            Stage::Reflect { bounds, reflect } => Some(post::floor_reflect(like, read(0).clone(), *bounds, *reflect)?),
            Stage::Wipe { bounds, clip } => Some(post::apply_clip(like, frame, read(0), *bounds, *clip)?),
            Stage::ShutterAverage { opacities } => {
                let info = ImageInfo::new(ISize::new(w, h), ColorType::RGBAF16, AlphaType::Premul, None);
                let mut sum = like.new_surface(&info, None).with_context(|| format!("no GPU memory for a {w}x{h} blur accumulator"))?;
                sum.canvas().clear(skia_safe::Color::TRANSPARENT);
                let mut plus = Paint::default();
                plus.set_blend_mode(skia_safe::BlendMode::Plus);
                let n = graph.frames.len() as f32 - 1.0;
                for (k, opacity) in opacities.iter().enumerate() {
                    plus.set_alpha_f(*opacity as f32 / n);
                    sum.canvas().draw_image(read(k).image(), (0, 0), Some(&plus));
                }
                Some(Layer::from_image(sum.image_snapshot()))
            }
            Stage::Composite { blend, opacity, placed } => {
                let mut paint = Paint::default();
                if let Some(b) = blend {
                    paint.set_blend_mode(b.skia());
                }
                paint.set_alpha_f(*opacity as f32);
                let img = read(0).clone();
                match placed {
                    Some(layer) => local::draw_placed(self.composite().canvas(), &frame.layers[*layer], &img, &paint)?,
                    None => {
                        self.composite().canvas().draw_image(img.image(), (0, 0), Some(&paint));
                    }
                }
                None
            }
            Stage::Snapshot => Some(Layer::from_image(self.composite().image_snapshot())),
            Stage::AdjustCoverage { layer } => Some(post::adjust_coverage(like, frame, comp, &frame.layers[*layer])?),
            Stage::AdjustMix => {
                let (below, changed, cov) = (read(0).clone(), read(1).clone(), read(2).clone());
                post::adjust_mix(like, frame, self.composite(), &below, &changed, &cov)?;
                None
            }
        })
    }

    /// Effects `0..upto` of an effect list on `img`.
    fn effects(&self, frame: &Frame, owner: FxOwner, upto: usize, img: Layer, behind: Option<&Layer>, maps: &[(usize, &Layer)]) -> Result<Layer> {
        let comp = self.graph.comp;
        let (layer, fx_list, states, t) = match owner {
            FxOwner::Layer(i) | FxOwner::Adjustment(i) => (Some(i), &comp.layers[i].fx, &frame.layers[i].fx, frame.layers[i].local_t),
            FxOwner::Look => (None, &comp.look, &frame.look, frame.t),
        };
        let space = match owner {
            FxOwner::Layer(i) => frame.layers[i].local.map(|b| b.space()),
            FxOwner::Adjustment(_) | FxOwner::Look => None,
        };
        let scope = fx::Scope { ctx: self.ctx, like: self.like, frame, layer, behind, pictures: maps, space };
        let label = match owner {
            FxOwner::Look => "look".to_string(),
            FxOwner::Layer(i) | FxOwner::Adjustment(i) => comp.layers[i].label.clone(),
        };
        fx::chain(&scope, img, &fx_list[..upto], &states[..upto], t).with_context(|| label)
    }

    fn run(mut self) -> Result<Vec<Layer>> {
        let mut left = self.graph.readers();
        for pass in &self.graph.passes {
            let made = self.exec(pass)?;
            if let (Write::New(id), Some(layer)) = (pass.write, made) {
                self.slots[id.0] = Some(layer);
            }
            for r in &pass.reads {
                left[r.0] -= 1;
                if left[r.0] == 0 {
                    self.slots[r.0] = None;
                }
            }
        }
        Ok(self.graph.outputs.iter().map(|&o| self.picture(o).clone()).collect())
    }
}

fn premul(c: Color4f) -> Color4f {
    Color4f::new(c.r * c.a, c.g * c.a, c.b * c.a, c.a)
}

fn execute(ctx: &Ctx, like: &Canvas, graph: &Graph) -> Result<Vec<Layer>> {
    Run { ctx, like, graph, slots: vec![None; graph.images], composite: None }.run()
}

/// Composite one composition frame into a transparent-or-backgrounded layer of the composition's size.
pub fn comp_picture(ctx: &Ctx, like: &Canvas, frame: &Frame) -> Result<Layer> {
    let comp = &ctx.plan.comps[&frame.comp];
    let samples = if frame.comp == crate::plan::ROOT { ctx.subs } else { &[] };
    Ok(execute(ctx, like, &comp_graph(comp, frame, samples))?.remove(0))
}

/// One layer's finished picture, before its opacity and blend.
pub fn layer_picture(ctx: &Ctx, like: &Canvas, frame: &Frame, i: usize) -> Result<Layer> {
    let comp = &ctx.plan.comps[&frame.comp];
    Ok(execute(ctx, like, &layer_graph(comp, frame, i))?.remove(0))
}

/// What effect `effect` of `layer` is given at `frame`: its input picture, and the pictures of the layers it reads.
pub fn effect_inputs(ctx: &Ctx, like: &Canvas, frame: &Frame, layer: usize, effect: usize) -> Result<(Layer, Vec<(usize, Layer)>)> {
    let comp = &ctx.plan.comps[&frame.comp];
    let (graph, read) = effect_inputs_graph(comp, frame, layer, effect);
    let mut pictures = execute(ctx, like, &graph)?.into_iter();
    let image = pictures.next().expect("the graph's first output is the effect's input picture");
    Ok((image, read.into_iter().zip(pictures).collect()))
}
