//! The element library. Each kind is built into a layer and landed with the shared entrance animation; kinds are
//! grouped by what they draw (type, panels, marks, charts, maps, media).

mod charts;
mod maps;
mod marks;
mod panels;
mod photo;
mod surface;
mod text;

use anyhow::Result;
use skia_safe::Canvas;

use super::{anim::{Timing}, model::{Element, Kind, H, W}, stage::{Stage}};
use turbomotion_core::{effects, layer::{Layer}};

/// Draw one element at time `t` onto the scene canvas. An element with effects or a blend mode is drawn on its own
/// transparent layer first, which the effects then process before it lands on the scene.
pub fn draw(st: &Stage, cv: &Canvas, el: &Element, t: f64) -> Result<()> {
    if el.common.fx.is_empty() && el.common.blend.is_none() {
        return draw_kind(st, cv, el, t);
    }
    let layer = Layer::try_render(cv, W, H, |c| draw_kind(st, c, el, t))?;
    let layer = effects::apply(&st.shaders.core, cv, layer, &el.common.fx, t)?;
    effects::composite(cv, &layer, el.common.blend);
    Ok(())
}

fn draw_kind(st: &Stage, cv: &Canvas, el: &Element, t: f64) -> Result<()> {
    let tm = Timing::new(&el.common, t);
    match &el.kind {
        Kind::Text(e) => text::text(st, cv, e, &tm),
        Kind::Chip(e) => text::chip(st, cv, e, &tm),
        Kind::Title(e) => text::title(st, cv, e, &tm),
        Kind::Counter(e) => text::counter(st, cv, e, &tm),
        Kind::Stamp(e) => text::stamp(st, cv, e, &tm),
        Kind::Card(e) => panels::card(st, cv, e, &tm),
        Kind::Doc(e) => panels::doc(st, cv, e, &tm),
        Kind::List(e) => panels::list(st, cv, e, &tm),
        Kind::Ticker(e) => panels::ticker(st, cv, e, &tm),
        Kind::Rect(e) => panels::rect(cv, e, &tm),
        Kind::Linechart(e) => charts::linechart(st, cv, e, &tm),
        Kind::Bars(e) => charts::bars(st, cv, e, &tm),
        Kind::Jar(e) => charts::jar(st, cv, e, &tm),
        Kind::Tiles(e) => charts::tiles(cv, e, &tm),
        Kind::Arrow(e) => marks::arrow(st, cv, e, &tm),
        Kind::Packets(e) => marks::packets(st, cv, e, &tm),
        Kind::Ring(e) => marks::ring(st, cv, e, &tm),
        Kind::VignetteRed(e) => marks::vignette_red(st, cv, e, &tm),
        Kind::Flash(e) => marks::flash(cv, e, &tm),
        Kind::Scrim(e) => marks::scrim(cv, e, &tm),
        Kind::Map(e) => maps::map(st, cv, e, &tm),
        Kind::Globe(e) => maps::globe(st, cv, e, &tm),
        Kind::Plate(e) => photo::plate(st, cv, e, &tm),
        Kind::Char(e) => photo::character(st, cv, e, &tm),
    }
}
