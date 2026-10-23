//! A retained vector scene in the vocabulary of the reference SVG diagrams: shapes with fill and
//! stroke, groups that pass their paint down, and the step/cue choreography of `kit.tsx`.
//!
//! A node can carry a reveal (`data-s`: "enter on step 2's cue 1, as a pop") and a hide
//! (`data-hide`). `Node::resolve` turns them into times for one shot and step; drawing then
//! needs only the shot's clock.

use anyhow::{anyhow, ensure, Context, Result};
use skia_safe::{
    gradient_shader::{self, Flags, Interpolation}, Canvas, Color, Color4f, ContourMeasureIter, Matrix, Paint, PaintCap, PaintJoin, PaintStyle, Path, PathEffect, PictureRecorder,
    Point, RRect, Rect, Shader, TileMode,
};

use crate::{
    ease::{Ease, SETTLE},
    film::Shot,
    layer::with_opacity,
    motion::{self, Tween},
    text::TextBlock,
};

/// SVG `text-anchor`.
#[derive(Debug, Clone, Copy)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

/// What a closed shape is filled with.
#[derive(Clone)]
pub enum Fill {
    None,
    Solid(Color),
    /// A radial gradient across the shape's bounding box from `Color` to the same colour at zero alpha.
    Fade(Color),
    /// A square tile of one horizontal and one vertical hairline, repeated from the origin.
    Cross { color: Color, cell: f32, width: f32 },
}

#[derive(Clone)]
pub struct Dash {
    pub pattern: [f32; 2],
    pub offset: f32,
}

#[derive(Clone)]
pub struct Stroke {
    pub color: Color,
    pub width: f32,
    pub cap: PaintCap,
    pub join: PaintJoin,
    pub dash: Option<Dash>,
}

/// Fill and stroke as set on one node; `None` inherits from the enclosing group.
#[derive(Clone, Default)]
struct Look {
    fill: Option<Fill>,
    stroke: Option<Stroke>,
}

impl Look {
    /// This look over `outer`: what is unset here comes from there.
    fn over(&self, outer: &Look) -> Look {
        Look { fill: self.fill.clone().or_else(|| outer.fill.clone()), stroke: self.stroke.clone().or_else(|| outer.stroke.clone()) }
    }
}

/// How an element enters (`data-a`, and the explicit tweens of the palace plan).
#[derive(Debug, Clone, Copy)]
pub enum Anim {
    Fade { duration: f32, ease: Ease },
    /// Rise `y` pixels while fading in.
    Up { y: f32, duration: f32 },
    /// Scale up from `from` while fading in.
    Pop { from: f32, duration: f32, ease: Ease },
    /// Stroke the path on (GSAP `drawSVG: 0% → 100%`).
    Draw { duration: f32, ease: Ease },
}

impl Anim {
    /// `show`: a plain fade.
    pub const fn fade() -> Self {
        Self::Fade { duration: 0.5, ease: Ease::PowerOut(1) }
    }

    /// `fadeUp`.
    pub const fn up() -> Self {
        Self::Up { y: 16.0, duration: 0.6 }
    }

    pub const fn pop() -> Self {
        Self::Pop { from: 0.6, duration: 0.45, ease: Ease::BackOut(2.2) }
    }

    pub const fn draw(duration: f32) -> Self {
        Self::Draw { duration, ease: Ease::PowerInOut(2) }
    }
}

/// `data-s`'s default stroke-on time.
pub const DRAW_SECONDS: f32 = 0.9;

/// The seconds a spec such as `"t1.4"`, `"1"`, `"1-0.3"` or `"payoff+0.6"` names in a shot.
fn when(shot: &Shot, spec: &str) -> Result<f32> {
    if let Some(seconds) = spec.strip_prefix('t').filter(|s| s.chars().all(|c| c.is_ascii_digit() || c == '.')) {
        return Ok(seconds.parse::<f32>().with_context(|| format!("{}: bad step time {spec:?}", shot.id))?.max(0.05));
    }
    let split = spec.char_indices().skip(1).find(|(_, c)| matches!(c, '+' | '-')).map_or(spec.len(), |(i, _)| i);
    let (cue, offset) = spec.split_at(split);
    let offset = if offset.is_empty() { 0.0 } else { offset.parse::<f32>().with_context(|| format!("{}: bad step time {spec:?}", shot.id))? };
    let index = if cue.chars().all(|c| c.is_ascii_digit()) && !cue.is_empty() {
        cue.parse::<usize>()?
    } else {
        *shot.cue_names.get(cue).ok_or_else(|| anyhow!("{}: no cue named {cue:?} (has {})", shot.id, names(shot)))?
    };
    Ok((shot.cue_at(index, None) + offset).max(0.05))
}

fn names(shot: &Shot) -> String {
    let mut names: Vec<&str> = shot.cue_names.keys().map(String::as_str).collect();
    names.sort_unstable();
    if names.is_empty() { "none".into() } else { names.join(", ") }
}

/// `"2:1-0.3"`: the step an element belongs to and the time spec within it.
fn step_spec(spec: &str) -> Result<(u32, &str)> {
    let (step, when) = spec.split_once(':').with_context(|| format!("bad step spec {spec:?}"))?;
    Ok((step.parse().with_context(|| format!("bad step in {spec:?}"))?, when))
}

#[derive(Clone)]
enum Enter {
    Present,
    /// A later step's element, not yet on screen.
    Hidden,
    /// A path whose stroke has not started to draw.
    Undrawn,
    At(f32, Anim),
}

#[derive(Clone, Copy)]
enum Exit {
    Stays,
    /// An earlier step's element that has since been replaced.
    Gone,
    /// Fades out over 0.3 s from this time.
    At(f32),
}

const EXIT_SECONDS: f32 = 0.3;

/// Unresolved `data-s` / `data-hide`.
struct Pending {
    reveal: Option<(String, Anim)>,
    hide: Option<String>,
}

enum Shape {
    Path(Path),
    Rect(Rect, f32),
    Ellipse(Rect),
    /// A text run: its first baseline starts at the point (adjusted for the anchor).
    Text(TextBlock, Point, Anchor),
    Group(Vec<Node>),
    /// A node rebuilt for each moment (animated geometry).
    Live(Box<dyn Fn(f32) -> Result<Node>>),
}

pub struct Node {
    shape: Shape,
    look: Look,
    opacity: f32,
    transform: Option<Matrix>,
    pending: Pending,
    enter: Enter,
    exit: Exit,
}

/// What a node's choreography makes of it at one moment.
struct Presence {
    opacity: f32,
    shift: f32,
    scale: f32,
    /// Fraction of a path's stroke drawn so far.
    drawn: Option<f32>,
    /// The scale a pop started from, which also fixes where the finished element sits.
    pop_from: Option<f32>,
}

impl Node {
    fn new(shape: Shape) -> Self {
        Self {
            shape,
            look: Look::default(),
            opacity: 1.0,
            transform: None,
            pending: Pending { reveal: None, hide: None },
            enter: Enter::Present,
            exit: Exit::Stays,
        }
    }

    pub fn path(d: &str) -> Result<Self> {
        Ok(Self::new(Shape::Path(Path::from_svg(d).with_context(|| format!("bad path data {d:?}"))?)))
    }

    pub fn from_path(path: Path) -> Self {
        Self::new(Shape::Path(path))
    }

    pub fn line(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        let mut path = Path::new();
        path.move_to((x1, y1)).line_to((x2, y2));
        Self::from_path(path)
    }

    pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self::new(Shape::Rect(Rect::from_xywh(x, y, w, h), 0.0))
    }

    pub fn circle(cx: f32, cy: f32, r: f32) -> Self {
        Self::ellipse(cx, cy, r, r)
    }

    pub fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Self {
        Self::new(Shape::Ellipse(Rect::new(cx - rx, cy - ry, cx + rx, cy + ry)))
    }

    pub fn text(block: TextBlock, x: f32, baseline: f32, anchor: Anchor) -> Self {
        Self::new(Shape::Text(block, Point::new(x, baseline), anchor))
    }

    pub fn group(children: impl IntoIterator<Item = Node>) -> Self {
        Self::new(Shape::Group(children.into_iter().collect()))
    }

    pub fn live(build: impl Fn(f32) -> Result<Node> + 'static) -> Self {
        Self::new(Shape::Live(Box::new(build)))
    }

    pub fn fill(mut self, color: Color) -> Self {
        self.look.fill = Some(Fill::Solid(color));
        self
    }

    pub fn no_fill(mut self) -> Self {
        self.look.fill = Some(Fill::None);
        self
    }

    pub fn filled(mut self, fill: Fill) -> Self {
        self.look.fill = Some(fill);
        self
    }

    pub fn stroke(mut self, color: Color, width: f32) -> Self {
        let kept = self.look.stroke.take();
        let (cap, join, dash) = kept.map_or((PaintCap::Butt, PaintJoin::Miter, None), |s| (s.cap, s.join, s.dash));
        self.look.stroke = Some(Stroke { color, width, cap, join, dash });
        self
    }

    fn stroke_mut(&mut self) -> &mut Stroke {
        self.look.stroke.get_or_insert(Stroke { color: Color::BLACK, width: 1.0, cap: PaintCap::Butt, join: PaintJoin::Miter, dash: None })
    }

    pub fn cap(mut self, cap: PaintCap) -> Self {
        self.stroke_mut().cap = cap;
        self
    }

    pub fn join(mut self, join: PaintJoin) -> Self {
        self.stroke_mut().join = join;
        self
    }

    pub fn dashed(mut self, on: f32, off: f32) -> Self {
        self.stroke_mut().dash = Some(Dash { pattern: [on, off], offset: 0.0 });
        self
    }

    /// Slide an already dashed stroke along its path (SVG `stroke-dashoffset`).
    pub fn dash_offset(mut self, offset: f32) -> Self {
        if let Some(dash) = &mut self.stroke_mut().dash {
            dash.offset = offset;
        }
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// Round the corners of a rectangle.
    pub fn rounded(mut self, radius: f32) -> Self {
        if let Shape::Rect(_, r) = &mut self.shape {
            *r = radius;
        }
        self
    }

    /// SVG `rotate(deg cx cy)`.
    pub fn rotated(mut self, degrees: f32, cx: f32, cy: f32) -> Self {
        self.transform = Some(Matrix::rotate_deg_pivot(degrees, (cx, cy)));
        self
    }

    /// SVG `translate(x y) scale(sx 1)`.
    pub fn placed(mut self, x: f32, y: f32, flip: bool) -> Self {
        let mut m = Matrix::translate((x, y));
        m.pre_scale((if flip { -1.0 } else { 1.0 }, 1.0), None);
        self.transform = Some(m);
        self
    }

    /// `data-s` + `data-a`: enter as `anim` at the time `spec` names (`"2:1-0.3"`).
    pub fn reveal(mut self, spec: &str, anim: Anim) -> Self {
        self.pending.reveal = Some((spec.to_owned(), anim));
        self
    }

    /// `data-hide`: fade out at the time `spec` names once its step is reached.
    pub fn hide(mut self, spec: &str) -> Self {
        self.pending.hide = Some(spec.to_owned());
        self
    }

    /// Enter as `anim` at `at` seconds, outside the step vocabulary.
    pub fn enter(mut self, at: f32, anim: Anim) -> Self {
        self.enter = Enter::At(at, anim);
        self
    }

    /// Not drawn on yet (GSAP `drawSVG: '0%'`).
    pub fn undrawn(mut self) -> Self {
        self.enter = Enter::Undrawn;
        self
    }

    /// Settle every node's reveal and hide for `shot` at `step`: earlier steps' elements are
    /// present, later steps' absent, and the current step's enter at their cue.
    pub fn resolve(&mut self, shot: &Shot, step: u32) -> Result<()> {
        if let Some(spec) = self.pending.hide.take() {
            let (at, cue) = step_spec(&spec)?;
            self.exit = match at.cmp(&step) {
                std::cmp::Ordering::Less => Exit::Gone,
                std::cmp::Ordering::Equal => Exit::At(when(shot, cue)?),
                std::cmp::Ordering::Greater => Exit::Stays,
            };
        }
        if let Some((spec, anim)) = self.pending.reveal.take() {
            let (at, cue) = step_spec(&spec)?;
            self.enter = match at.cmp(&step) {
                std::cmp::Ordering::Greater => Enter::Hidden,
                std::cmp::Ordering::Less => Enter::Present,
                std::cmp::Ordering::Equal => {
                    if matches!(anim, Anim::Draw { .. }) {
                        ensure!(matches!(self.shape, Shape::Path(_)), "{}: only a path can be drawn on", shot.id);
                    }
                    Enter::At(when(shot, cue)?, anim)
                }
            };
        }
        if let Shape::Group(children) = &mut self.shape {
            children.iter_mut().try_for_each(|c| c.resolve(shot, step))?;
        }
        Ok(())
    }

    /// The words on screen under each reveal (`"2:1-0.3"` and the text it brings in), before
    /// `resolve`: what a step shows, for tools that plan narration against the drawing.
    pub fn reveals(&self) -> Vec<(String, String)> {
        let mut found: Vec<(String, Vec<&str>)> = Vec::new();
        self.collect_text(None, &mut found);
        found.into_iter().filter(|(_, words)| !words.is_empty()).map(|(spec, words)| (spec, words.join(" "))).collect()
    }

    fn collect_text<'a>(&'a self, current: Option<usize>, found: &mut Vec<(String, Vec<&'a str>)>) {
        let current = match &self.pending.reveal {
            Some((spec, _)) => {
                found.push((spec.clone(), Vec::new()));
                Some(found.len() - 1)
            }
            None => current,
        };
        match &self.shape {
            Shape::Text(block, ..) => {
                if let Some(i) = current {
                    found[i].1.push(block.text());
                }
            }
            Shape::Group(children) => children.iter().for_each(|c| c.collect_text(current, found)),
            _ => {}
        }
    }

    /// Where the node's geometry lies (an SVG `getBBox`, which excludes the stroke).
    fn bounds(&self, t: f32) -> Result<Rect> {
        Ok(match &self.shape {
            Shape::Path(p) => p.compute_tight_bounds(),
            Shape::Rect(r, _) | Shape::Ellipse(r) => *r,
            Shape::Text(block, at, anchor) => {
                let width = block.content_width();
                let left = at.x - width * anchor_share(*anchor);
                Rect::from_xywh(left, at.y - block.baseline(), width, block.height)
            }
            Shape::Group(children) => {
                let mut union: Option<Rect> = None;
                for child in children {
                    let mut r = child.bounds(t)?;
                    if let Some(m) = &child.transform {
                        r = m.map_rect(r).0;
                    }
                    union = Some(union.map_or(r, |u| Rect::new(u.left.min(r.left), u.top.min(r.top), u.right.max(r.right), u.bottom.max(r.bottom))));
                }
                union.unwrap_or_else(Rect::new_empty)
            }
            Shape::Live(build) => build(t)?.bounds(t)?,
        })
    }

    fn presence(&self, t: f32) -> Presence {
        let mut p = Presence { opacity: self.opacity, shift: 0.0, scale: 1.0, drawn: None, pop_from: None };
        match self.enter {
            Enter::Present => {}
            Enter::Hidden => p.opacity = 0.0,
            Enter::Undrawn => p.drawn = Some(0.0),
            Enter::At(at, anim) => match anim {
                Anim::Fade { duration, ease } => p.opacity *= Tween::new(at, duration, ease).progress(t),
                Anim::Up { y, duration } => {
                    let rise = motion::fade_up(t, at, y, duration);
                    p.opacity *= rise.opacity;
                    p.shift = rise.dy;
                }
                Anim::Pop { from, duration, ease } => {
                    let k = Tween::new(at, duration, ease).progress(t);
                    p.opacity *= k.clamp(0.0, 1.0);
                    p.scale = from + (1.0 - from) * k;
                    p.pop_from = Some(from);
                }
                Anim::Draw { duration, ease } => p.drawn = Some(motion::draw(t, at, duration, ease)),
            },
        }
        match self.exit {
            Exit::Stays => {}
            Exit::Gone => p.opacity = 0.0,
            Exit::At(at) => p.opacity *= 1.0 - Tween::new(at, EXIT_SECONDS, SETTLE).progress(t),
        }
        p
    }

    /// Draw at `seconds` into the shot; paint nothing sets is filled black, as in SVG.
    pub fn draw(&self, canvas: &Canvas, seconds: f32) -> Result<()> {
        self.paint(canvas, seconds, &Look { fill: Some(Fill::Solid(Color::BLACK)), stroke: None })
    }

    fn paint(&self, canvas: &Canvas, t: f32, outer: &Look) -> Result<()> {
        let p = self.presence(t);
        if p.opacity <= 0.0 {
            return Ok(());
        }
        canvas.save();
        if p.shift != 0.0 {
            canvas.translate((0.0, p.shift));
        }
        if let Some(from) = p.pop_from {
            let c = self.bounds(t)?;
            canvas.translate(motion::pop_residual(from, (c.width(), c.height())));
            canvas.translate((c.center_x(), c.center_y()));
            canvas.scale((p.scale, p.scale));
            canvas.translate((-c.center_x(), -c.center_y()));
        }
        if let Some(m) = &self.transform {
            canvas.concat(m);
        }
        let look = self.look.over(outer);
        let mut result = Ok(());
        let mut shape = || result = self.paint_shape(canvas, t, &look, p.drawn);
        if p.opacity < 1.0 { with_opacity(canvas, p.opacity, shape) } else { shape() }
        canvas.restore();
        result
    }

    fn paint_shape(&self, canvas: &Canvas, t: f32, look: &Look, drawn: Option<f32>) -> Result<()> {
        match &self.shape {
            Shape::Path(path) => paint_geometry(canvas, |c, paint| { c.draw_path(path, paint); }, path.compute_tight_bounds(), look, drawn.map(|d| (d, path))),
            Shape::Rect(rect, radius) if *radius > 0.0 => {
                let rrect = RRect::new_rect_xy(rect, *radius, *radius);
                paint_geometry(canvas, |c, paint| { c.draw_rrect(rrect, paint); }, *rect, look, None);
            }
            Shape::Rect(rect, _) => paint_geometry(canvas, |c, paint| { c.draw_rect(rect, paint); }, *rect, look, None),
            Shape::Ellipse(oval) => paint_geometry(canvas, |c, paint| { c.draw_oval(oval, paint); }, *oval, look, None),
            Shape::Text(block, at, anchor) => block.draw_on_baseline(canvas, at.x - block.content_width() * anchor_share(*anchor), at.y),
            Shape::Group(children) => {
                for child in children {
                    child.paint(canvas, t, look)?;
                }
            }
            Shape::Live(build) => build(t)?.paint(canvas, t, look)?,
        }
        Ok(())
    }
}

fn anchor_share(anchor: Anchor) -> f32 {
    match anchor {
        Anchor::Start => 0.0,
        Anchor::Middle => 0.5,
        Anchor::End => 1.0,
    }
}

/// Fill then stroke one shape. `trim` is a path and the fraction of its stroke to draw.
fn paint_geometry(canvas: &Canvas, draw: impl Fn(&Canvas, &Paint), bounds: Rect, look: &Look, trim: Option<(f32, &Path)>) {
    if let Some(fill) = &look.fill {
        if let Some(paint) = fill_paint(fill, bounds) {
            draw(canvas, &paint);
        }
    }
    let Some(stroke) = &look.stroke else { return };
    let mut paint = Paint::default();
    paint.set_anti_alias(true).set_style(PaintStyle::Stroke).set_color(stroke.color).set_stroke_width(stroke.width).set_stroke_cap(stroke.cap).set_stroke_join(stroke.join);
    match trim {
        // GSAP's drawSVG replaces any dash pattern with "drawn, rest of the path"; a stroke
        // that has not started draws nothing, even with round caps.
        Some((fraction, _)) if fraction <= 0.0 => return,
        Some((fraction, path)) => {
            let total: f32 = ContourMeasureIter::new(path, false, None).map(|c| c.length()).sum();
            let drawn = total * fraction;
            if drawn < total {
                paint.set_path_effect(PathEffect::dash(&[drawn, DRAW_GAP], 0.0));
            }
        }
        None => {
            if let Some(d) = &stroke.dash {
                paint.set_path_effect(PathEffect::dash(&d.pattern, d.offset));
            }
        }
    }
    draw(canvas, &paint);
}

/// The gap GSAP leaves after the drawn part of a stroke: longer than any path.
const DRAW_GAP: f32 = 999_999.0;

fn fill_paint(fill: &Fill, bounds: Rect) -> Option<Paint> {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    match fill {
        Fill::None => return None,
        Fill::Solid(color) => {
            paint.set_color(*color);
        }
        Fill::Fade(color) => {
            let mut to_box = Matrix::translate((bounds.center_x(), bounds.center_y()));
            to_box.pre_scale((bounds.width() / 2.0, bounds.height() / 2.0), None);
            let stops = [Color4f::from(*color), Color4f::from(color.with_a(0))];
            paint.set_shader(gradient_shader::radial_with_interpolation(
                (Point::new(0.0, 0.0), 1.0),
                (&stops[..], None),
                None,
                TileMode::Clamp,
                Interpolation::from(Flags::default()),
                &to_box,
            )?);
        }
        Fill::Cross { color, cell, width } => {
            paint.set_shader(cross_tile(*color, *cell, *width)?);
        }
    }
    Some(paint)
}

/// An SVG `<pattern>` of one horizontal and one vertical hairline through the middle of each cell.
fn cross_tile(color: Color, cell: f32, width: f32) -> Option<Shader> {
    let tile = Rect::from_wh(cell, cell);
    let mut recorder = PictureRecorder::new();
    let canvas = recorder.begin_recording(tile, None);
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Stroke).set_color(color).set_stroke_width(width);
    canvas.draw_line((0.0, cell / 2.0), (cell, cell / 2.0), &paint);
    canvas.draw_line((cell / 2.0, 0.0), (cell / 2.0, cell), &paint);
    let picture = recorder.finish_recording_as_picture(None)?;
    Some(picture.to_shader((TileMode::Repeat, TileMode::Repeat), skia_safe::FilterMode::Linear, None, &tile))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn shot() -> Shot {
        Shot {
            id: "s001".into(),
            kind: "diagram".into(),
            args: vec![],
            params: HashMap::new(),
            duration: 10.0,
            start_frame: 0,
            duration_in_frames: 300,
            cues: 2,
            cue_times: vec![1.0, 3.0],
            cue_names: HashMap::from([("payoff".to_string(), 1)]),
        }
    }

    #[test]
    fn resolves_absolute_cue_relative_and_named_times() {
        let s = shot();
        assert_eq!(when(&s, "t1.4").unwrap(), 1.4);
        assert_eq!(when(&s, "0").unwrap(), 1.0);
        assert!((when(&s, "1-0.3").unwrap() - 2.7).abs() < 1e-6);
        assert!((when(&s, "payoff+0.6").unwrap() - 3.6).abs() < 1e-6);
        assert_eq!(when(&s, "0-5").unwrap(), 0.05);
        assert!(when(&s, "finale").is_err());
    }

    #[test]
    fn earlier_steps_are_present_and_later_ones_absent() {
        let s = shot();
        let mut nodes: Vec<Node> = ["1:0", "2:0", "3:0"].iter().map(|spec| Node::circle(0.0, 0.0, 5.0).reveal(spec, Anim::fade())).collect();
        nodes.iter_mut().try_for_each(|n| n.resolve(&s, 2)).unwrap();
        assert_eq!(nodes.iter().map(|n| n.presence(0.0).opacity).collect::<Vec<_>>(), vec![1.0, 0.0, 0.0]);
        assert_eq!(nodes[1].presence(5.0).opacity, 1.0);
    }

    #[test]
    fn a_replaced_element_fades_out_at_its_cue() {
        let s = shot();
        let mut node = Node::circle(0.0, 0.0, 5.0).hide("1:0");
        node.resolve(&s, 1).unwrap();
        assert_eq!(node.presence(0.9).opacity, 1.0);
        assert_eq!(node.presence(2.0).opacity, 0.0);
        let mut later = Node::circle(0.0, 0.0, 5.0).hide("1:0");
        later.resolve(&s, 2).unwrap();
        assert_eq!(later.presence(0.0).opacity, 0.0);
    }

    #[test]
    fn only_paths_can_be_drawn_on() {
        let mut node = Node::circle(0.0, 0.0, 5.0).reveal("1:0", Anim::draw(1.0));
        assert!(node.resolve(&shot(), 1).is_err());
    }

    #[test]
    fn a_path_waiting_to_draw_has_drawn_nothing() {
        let mut node = Node::path("M0 0 H10").unwrap().reveal("1:t2", Anim::draw(1.0));
        node.resolve(&shot(), 1).unwrap();
        assert_eq!(node.presence(0.0).drawn, Some(0.0));
        assert_eq!(node.presence(9.0).drawn, Some(1.0));
    }
}
