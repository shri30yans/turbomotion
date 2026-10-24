//! Hand-drawn explainer diagrams and the palace plan (`bench/remotion/src/diagrams`): a board of vector
//! shapes that enter step by step on the narration's cues.

mod engineering;
mod explainers;
mod graph;
mod kit;
mod node;

use anyhow::{bail, Context, Result};
use skia_safe::Canvas;

use self::{kit::Kit, node::Node};
use super::{caption, Loader, Moment, Scene, Stage};
use crate::{
    film::Shot,
    paper::{Backdrop, Paper},
    palette::{hex, INK, WHITE},
    text::{Fonts, Style},
};

pub const DIAGRAM_PARAMS: &[&str] = &["step"];
pub const PLAN_PARAMS: &[&str] = &["step", "highlight", "mark"];
pub const GRAPH_PARAMS: &[&str] = graph::PARAMS;

/// The diagrams `shot diagram <name>` can name.
const DIAGRAMS: &[(&str, Build)] = &[
    ("jaivana_scale", explainers::jaivana_scale),
    ("family", explainers::family),
    ("jali_view", explainers::jali_view),
    ("ganesh_route", explainers::ganesh_route),
    ("mirror_light", explainers::mirror_light),
    ("cooling", explainers::cooling),
    ("water_lift", engineering::water_lift),
];

type Build = fn(&Kit, &Shot) -> Result<Sheet>;

/// One finished drawing: its nodes, and the board captions around them.
pub struct Sheet {
    pub dark: bool,
    pub tag: &'static str,
    pub legend: Option<&'static str>,
    /// The step whose elements are entering; earlier steps are already on the board.
    pub step: u32,
    pub nodes: Vec<Node>,
}

struct Diagram {
    backdrop: Backdrop,
    tag: crate::text::TextBlock,
    legend: Option<crate::text::TextBlock>,
    nodes: Vec<Node>,
}

/// A shot's `step` parameter.
pub fn step_param(shot: &Shot, default: u32) -> Result<u32> {
    shot.param("step").map_or(Ok(default), |s| s.parse().with_context(|| format!("{}: step {s:?} is not a number", shot.id)))
}

/// One reveal of a step: when it enters (`"1-0.3"`, `"payoff+0.6"`, `"t1.4"`) and the words it brings.
#[derive(Debug, serde::Serialize)]
pub struct Read {
    pub when: String,
    pub text: String,
}

/// The words `step` of the diagram `name` puts on screen, with the time each arrives.
pub fn reads(fonts: &Fonts, name: &str, step: u32) -> Result<Vec<Read>> {
    let (_, build) = DIAGRAMS.iter().find(|(n, _)| *n == name).with_context(|| format!("no diagram named {name:?}"))?;
    let shot = Shot {
        id: name.into(),
        kind: "diagram".into(),
        args: vec![name.into()],
        params: [("step".to_string(), step.to_string())].into(),
        duration: 0.0,
        start_frame: 0,
        duration_in_frames: 0,
        cues: 0,
        cue_times: vec![],
        cue_names: Default::default(),
    };
    let sheet = build(&Kit { fonts }, &shot)?;
    let mut reads = Vec::new();
    for node in &sheet.nodes {
        for (spec, text) in node.reveals() {
            if let Some((at, when)) = spec.split_once(':') {
                if at.parse() == Ok(step) {
                    reads.push(Read { when: when.to_owned(), text });
                }
            }
        }
    }
    Ok(reads)
}

/// Reject a diagram name the renderer does not know, before any work starts.
pub fn verify(shot: &Shot) -> Result<()> {
    let name = shot.args.first().map_or("", String::as_str);
    if !DIAGRAMS.iter().any(|(n, _)| *n == name) {
        bail!("{}: no diagram named {name:?}", shot.id);
    }
    Ok(())
}

pub fn prepare_diagram(loader: &mut Loader, shot: &Shot) -> Result<Box<dyn Scene>> {
    let name = shot.args.first().map_or("", String::as_str);
    let (_, build) = DIAGRAMS.iter().find(|(n, _)| *n == name).with_context(|| format!("{}: no diagram named {name:?}", shot.id))?;
    assemble(loader, shot, *build)
}

/// A `graph` shot: tree, flow or timeline laid out from its JSON data file.
pub fn prepare_graph(loader: &mut Loader, shot: &Shot) -> Result<Box<dyn Scene>> {
    let json = graph::load(loader.public, shot)?;
    let sheet = graph::build(&Kit { fonts: loader.fonts }, shot, &json)?;
    finish(loader, shot, sheet)
}

pub fn prepare_plan(loader: &mut Loader, shot: &Shot) -> Result<Box<dyn Scene>> {
    assemble(loader, shot, engineering::palace_plan)
}

fn assemble(loader: &mut Loader, shot: &Shot, build: Build) -> Result<Box<dyn Scene>> {
    let sheet = build(&Kit { fonts: loader.fonts }, shot)?;
    finish(loader, shot, sheet)
}

fn finish(loader: &mut Loader, shot: &Shot, mut sheet: Sheet) -> Result<Box<dyn Scene>> {
    sheet.nodes.iter_mut().try_for_each(|n| n.resolve(shot, sheet.step))?;

    let size = (loader.film.width as f32, loader.film.height as f32);
    let backdrop = Backdrop::prepare(loader.context, loader.assets, Paper::Board { dark: sheet.dark }, size)?;
    let tag_ink = if sheet.dark { WHITE } else { INK };
    let legend_ink = if sheet.dark { hex_alpha(0xFFF8EC, 140) } else { caption::LEGEND_INK };
    let line = |text: &str, style: Style| loader.fonts.typeset(text, &style, crate::text::UNBOUNDED, false);
    Ok(Box::new(Diagram {
        backdrop,
        tag: line(sheet.tag, Style::mono(21.0, 2.2, tag_ink))?,
        legend: sheet.legend.map(|l| line(l, Style::mono(17.0, 2.0, legend_ink))).transpose()?,
        nodes: sheet.nodes,
    }))
}

fn hex_alpha(rgb: u32, alpha: u8) -> skia_safe::Color {
    hex(rgb).with_a(alpha)
}

impl Scene for Diagram {
    fn draw(&self, canvas: &Canvas, stage: &Stage, at: Moment) -> Result<()> {
        self.backdrop.draw(canvas, at.frame)?;
        for node in &self.nodes {
            node.draw(canvas, at.seconds)?;
        }
        caption::draw_tag(canvas, &self.tag);
        if let Some(legend) = &self.legend {
            caption::draw_legend(canvas, legend, (stage.film.width as f32, stage.film.height as f32), 56.0);
        }
        Ok(())
    }
}
