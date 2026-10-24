//! The studio's graphics, offered to compositions. `comp` knows nothing of globes or charts; this module adapts the
//! studio's element library to its `Pack` interface so one timeline can hold a film element:
//!
//! ```json
//! {"type": "pack", "name": "studio.element",
//!  "params": {"film": "../../films/01", "element": {"k": "globe", "t0": 0, "rot": [[0, [10, 30]], [4, [60, 20]]], "D": 900}}}
//! ```
//!
//! `film` is the folder holding the film's `film.json` (its fonts), geography and plates, relative to the composition;
//! `element` is any element exactly as written in a film, with times counted from the layer's start.

use std::{cell::RefCell, collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use turbomotion_comp::pack::{Pack, PackCtx};
use turbomotion_core::layer::Layer;

use crate::{
    elements,
    model::{Element, Film, H, W},
    stage::Stage,
};

#[derive(Default)]
struct ElementPack {
    stages: RefCell<HashMap<PathBuf, Stage>>,
}

impl Pack for ElementPack {
    fn name(&self) -> &'static str {
        "studio.element"
    }

    fn summary(&self) -> &'static str {
        "One studio film element (title, card, chart, map, globe, plate...) painted at the layer's time. params: {film: <folder with film.json>, element: {k: ..., ...}}"
    }

    fn validate(&self, params: &Value) -> Result<()> {
        params.get("film").and_then(Value::as_str).ok_or_else(|| anyhow!("studio.element needs params.film (the folder with film.json)"))?;
        let el = params.get("element").ok_or_else(|| anyhow!("studio.element needs params.element, an element as written in a film"))?;
        serde_json::from_value::<Element>(el.clone()).map(|_| ()).map_err(|e| anyhow!("params.element: {e}"))
    }

    fn paint(&self, ctx: &PackCtx, params: &Value, t: f64) -> Result<Layer> {
        let film = ctx.dir.join(params["film"].as_str().unwrap_or("."));
        let el: Element = serde_json::from_value(params["element"].clone()).context("params.element")?;
        let mut stages = self.stages.borrow_mut();
        if !stages.contains_key(&film) {
            let loaded = Film::load(&film).with_context(|| format!("loading the film at {}", film.display()))?;
            stages.insert(film.clone(), Stage::new(&loaded)?);
        }
        let stage = &stages[&film];
        let drawn = Layer::try_render(ctx.like, W, H, |c| elements::draw(stage, c, &el, t))?;
        if (ctx.width, ctx.height) == (W, H) {
            return Ok(drawn);
        }
        Layer::try_render(ctx.like, ctx.width, ctx.height, |c| {
            let mut p = skia_safe::Paint::default();
            p.set_anti_alias(true);
            c.draw_image_rect_with_sampling_options(drawn.image(), None, skia_safe::Rect::from_wh(ctx.width as f32, ctx.height as f32), turbomotion_core::layer::linear(), &p);
            Ok(())
        })
    }
}

/// The packs this crate offers; register with `turbomotion_comp::pack::register(turbomotion_studio::pack::all)`.
pub fn all() -> Vec<Box<dyn Pack>> {
    vec![Box::new(ElementPack::default())]
}
