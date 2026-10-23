//! Map shots: Natural Earth outlines under a keyed camera, with areas, rivers, schematic
//! terrain, rings, routes, pins and labels that appear on the shot's cues.
//! Mirrors `MapShot` in `bench/remotion/src/maps/MapShot.tsx`.

mod geo;
mod spec;

use anyhow::{bail, Context, Result};
use skia_safe::{
    gpu::Mipmapped, BlurStyle, Canvas, Color, Image, MaskFilter, Paint, PaintCap, PaintJoin, PaintStyle, Path,
    PathEffect, PathFillType, Point, Rect,
};

use self::{
    geo::{Geo, Ring},
    spec::{LonLat, Maps, PinStyle, RouteColor, SchematicKind, When},
};
use super::{caption, Loader, Moment, Scene, Stage};
use crate::{
    film::Shot,
    image::{draw_fit, load_texture},
    layer::{with_blur, with_opacity},
    palette::{INK, INK_SOFT, OCHRE, TEAL, TERRACOTTA},
    text::{Family, Halo, Style, TextBlock, UNBOUNDED},
};

/// `focus` and `pins` appear on some map shots but the reference map ignores them.
pub const PARAMS: &[&str] = &["step", "rings", "note", "focus", "pins"];

const WATER: Color = Color::from_rgb(0xCF, 0xDA, 0xD2);
const HOME_LAND: Color = Color::from_rgb(0xEF, 0xE3, 0xC8);
const LAND: Color = Color::from_rgb(0xE2, 0xD5, 0xB9);
const BORDER: Color = Color::from_rgb(0xA9, 0x94, 0x75);
const STATE_LINE: Color = Color::from_rgb(0xC2, 0xB0, 0x8F);
const RIVER: Color = Color::from_rgb(0x86, 0xA9, 0xB4);
const RIVERS_SHOWN: [&str; 6] = ["Ganges", "Indus", "Sutlej", "Chenab", "Narmada", "Brahmaputra"];
const LABEL_HALO: Color = Color::from_rgb(0xF4, 0xEB, 0xD9);
const ROUTE_HALO: Color = Color::from_rgb(0xF1, 0xE7, 0xD2);
const PIN_RIM: Color = Color::from_rgb(0xF7, 0xEE, 0xDC);
const LABEL_INK: Color = Color::from_rgb(0x7A, 0x6A, 0x52);
const DEFAULT_LEGEND: &str = "MODERN BORDERS SHOWN FOR ORIENTATION";
/// Kilometres per degree of latitude.
const KM_PER_DEGREE: f32 = 111.3;
/// Seconds before a step's element with no time of its own appears.
const DEFAULT_AT: f32 = 0.3;
const DEFAULT_VIEW_AT: f32 = 1.0;
const DEFAULT_VIEW_DUR: f32 = 2.2;

/// An element belonging to an earlier step is already there; a later step's is not yet.
const PRESENT: f32 = -10.0;
const ABSENT: f32 = f32::INFINITY;

fn ease(x: f32) -> f32 {
    1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3)
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

struct Keyframe {
    center: LonLat,
    span: f64,
    at: f32,
    dur: f32,
}

struct AreaEl {
    state: String,
    at: f32,
}

enum Terrain {
    /// Soft blobs: position and radius in km.
    Hills(Vec<(LonLat, f32)>),
    Lake(Ring),
    /// A 3×3 grid of plots between two opposite corners.
    Grid([LonLat; 2]),
}

struct SchematicEl {
    terrain: Terrain,
    at: f32,
}

struct RingEl {
    center: LonLat,
    km: f32,
    at: f32,
    label: Option<TextBlock>,
}

struct RouteEl {
    via: Vec<LonLat>,
    at: f32,
    dur: f32,
    bend: f32,
    dashed: bool,
    color: Color,
    label: Option<TextBlock>,
    label_offset: (f32, f32),
}

struct PinEl {
    at_place: LonLat,
    at: f32,
    home: bool,
    offset: (f32, f32),
    label: TextBlock,
    sub: Option<TextBlock>,
}

struct LabelEl {
    at_place: LonLat,
    at: f32,
    faint: bool,
    text: TextBlock,
}

struct MapShot {
    geo: Geo,
    home: Vec<String>,
    keyframes: Vec<Keyframe>,
    areas: Vec<AreaEl>,
    schematic: Vec<SchematicEl>,
    rings: Vec<RingEl>,
    routes: Vec<RouteEl>,
    pins: Vec<PinEl>,
    labels: Vec<LabelEl>,
    duration: f32,
    grain: Image,
    title: Option<TextBlock>,
    note: Option<TextBlock>,
    legend: TextBlock,
}

/// Geographic coordinates to frame pixels for one camera position.
struct Projection {
    size: (f32, f32),
    center: LonLat,
    /// Longitude compression at the camera's latitude.
    cos: f64,
    /// Pixels per degree of latitude.
    scale: f64,
}

impl Projection {
    fn point(&self, [lon, lat]: LonLat) -> Point {
        Point::new(
            (f64::from(self.size.0) / 2.0 + (lon - self.center[0]) * self.cos * self.scale) as f32,
            (f64::from(self.size.1) / 2.0 - (lat - self.center[1]) * self.scale) as f32,
        )
    }

    fn km_px(&self) -> f32 {
        self.scale as f32 / KM_PER_DEGREE
    }

    fn polygon(&self, rings: &[Ring], closed: bool) -> Path {
        let mut path = Path::new();
        for ring in rings {
            let points: Vec<Point> = ring.iter().map(|p| self.point(*p)).collect();
            path.add_poly(&points, closed);
        }
        path
    }
}

fn step_of(shot: &Shot) -> Result<u32> {
    match shot.param("step") {
        Some(step) => step.parse().with_context(|| format!("{}: step is not a number", shot.id)),
        None => Ok(if shot.param("rings").is_some() { 2 } else { 1 }),
    }
}

pub fn prepare(loader: &mut Loader, shot: &Shot) -> Result<Box<dyn Scene>> {
    let name = shot.args.first().with_context(|| format!("{}: map shot without a map", shot.id))?;
    let maps = Maps::load(loader.assets)?;
    let spec = maps.maps.get(name).with_context(|| format!("{}: unknown map {name:?}", shot.id))?;
    let step = step_of(shot)?;
    let fonts = loader.fonts;

    // A step's elements animate on their cues; those of earlier steps are already present.
    let at = |element_step: Option<u32>, when: &Option<When>| -> Result<f32> {
        let element_step = element_step.unwrap_or(1);
        match element_step.cmp(&step) {
            std::cmp::Ordering::Less => Ok(PRESENT),
            std::cmp::Ordering::Greater => Ok(ABSENT),
            std::cmp::Ordering::Equal => when.as_ref().map_or(Ok(DEFAULT_AT), |w| w.resolve(shot)),
        }
    };
    let text = |text: &str, style: &Style| fonts.typeset(text, style, UNBOUNDED, false);

    let mut keyframes: Vec<&spec::View> = spec.views.iter().filter(|v| v.step.unwrap_or(step) == step).collect();
    if keyframes.is_empty() {
        keyframes = spec.views.iter().collect();
    }
    let keyframes = keyframes
        .into_iter()
        .map(|v| {
            Ok(Keyframe {
                center: v.center,
                span: v.span,
                at: v.at.as_ref().map_or(Ok(DEFAULT_VIEW_AT), |w| w.resolve(shot))?,
                dur: v.dur.unwrap_or(DEFAULT_VIEW_DUR),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if keyframes.is_empty() {
        bail!("{}: map {name:?} has no views", shot.id);
    }

    let areas = spec.areas.iter().map(|a| Ok(AreaEl { state: a.state.clone(), at: at(a.step, &a.at)? })).collect::<Result<_>>()?;
    let schematic = spec
        .schematic
        .iter()
        .map(|s| {
            let ring = || s.pts.iter().map(|p| [p[0], p[1]]).collect::<Ring>();
            let terrain = match s.kind {
                SchematicKind::Hills => Terrain::Hills(s.pts.iter().map(|p| ([p[0], p[1]], p.get(2).map_or(1.2, |r| *r as f32))).collect()),
                SchematicKind::Lake => Terrain::Lake(ring()),
                SchematicKind::Grid => match ring()[..] {
                    [a, b] => Terrain::Grid([a, b]),
                    _ => bail!("{}: a grid needs two corners", shot.id),
                },
            };
            Ok(SchematicEl { terrain, at: at(s.step, &s.at)? })
        })
        .collect::<Result<_>>()?;
    let rings = spec
        .rings
        .iter()
        .map(|r| {
            let style = Style { letter_spacing: 3.0, ..Style::mono(24.0, 3.0, TERRACOTTA) };
            Ok(RingEl { center: r.center, km: r.km, at: at(r.step, &r.at)?, label: r.label.as_deref().map(|l| text(l, &style)).transpose()? })
        })
        .collect::<Result<_>>()?;
    let routes = spec
        .routes
        .iter()
        .map(|r| {
            let color = if r.color == Some(RouteColor::Teal) { TEAL } else { TERRACOTTA };
            let style = Style { halo: Some(Halo { color: ROUTE_HALO, width: 7.0 }), ..Style::new(Family::Manrope, 28.0, 600.0, color) };
            Ok(RouteEl {
                via: r.via.iter().map(|id| maps.place(id)).collect::<Result<_>>()?,
                at: at(r.step, &r.at)?,
                dur: r.dur.unwrap_or(1.6),
                bend: r.bend.unwrap_or(0.18),
                dashed: r.dashed,
                color,
                label: r.label.as_deref().map(|l| text(l, &style)).transpose()?,
                label_offset: (r.label_dx.unwrap_or(0.0), r.label_dy.unwrap_or(0.0)),
            })
        })
        .collect::<Result<_>>()?;
    let pins = spec
        .pins
        .iter()
        .map(|p| {
            let home = p.style == Some(PinStyle::Home);
            let name = Style {
                letter_spacing: -0.5,
                halo: Some(Halo { color: LABEL_HALO, width: 9.0 }),
                ..Style::new(Family::Manrope, if home { 44.0 } else { 34.0 }, if home { 700.0 } else { 600.0 }, INK)
            };
            let sub = Style { halo: Some(Halo { color: LABEL_HALO, width: 7.0 }), ..Style::mono(22.0, 1.5, TERRACOTTA) };
            Ok(PinEl {
                at_place: maps.place(&p.place)?,
                at: at(p.step, &p.at)?,
                home,
                offset: (p.dx.unwrap_or(22.0), p.dy.unwrap_or(-18.0)),
                label: text(p.label.as_deref().unwrap_or(&p.place), &name)?,
                sub: p.sub.as_deref().map(|s| text(s, &sub)).transpose()?,
            })
        })
        .collect::<Result<_>>()?;
    let labels = spec
        .labels
        .iter()
        .map(|l| {
            let style = Style::mono(l.size.unwrap_or(24.0), 6.0, LABEL_INK);
            Ok(LabelEl { at_place: l.position, at: at(l.step, &l.at)?, faint: l.faint, text: text(&l.text, &style)? })
        })
        .collect::<Result<_>>()?;

    let note = shot
        .param("note")
        .or(spec.note.as_deref())
        .map(|n| {
            let style = Style::new(Family::Manrope, 26.0, 500.0, INK_SOFT);
            let width = fonts.shrink_to_fit(n, &style, 1100.0);
            fonts.typeset(n, &style, width, false)
        })
        .transpose()?;
    let title = spec.title.as_deref().map(|t| text(t, &Style::mono(21.0, 2.2, INK))).transpose()?;
    let legend = text(spec.legend.as_deref().unwrap_or(DEFAULT_LEGEND), &Style::mono(17.0, 2.0, caption::LEGEND_INK))?;

    Ok(Box::new(MapShot {
        geo: Geo::load(loader.assets)?,
        home: spec.home.clone(),
        keyframes,
        areas,
        schematic,
        rings,
        routes,
        pins,
        labels,
        duration: shot.duration,
        grain: load_texture(loader.context, &loader.assets.join("paper.jpg"), Mipmapped::No)?,
        title,
        note,
        legend,
    }))
}

fn fill(color: Color) -> Paint {
    let mut paint = Paint::default();
    paint.set_anti_alias(true).set_color(color);
    paint
}

fn stroke(color: Color, width: f32) -> Paint {
    let mut paint = fill(color);
    paint.set_style(PaintStyle::Stroke).set_stroke_width(width);
    paint
}

fn dashed(paint: &mut Paint, on: f32, off: f32) {
    paint.set_path_effect(PathEffect::dash(&[on, off], 0.0));
}

impl MapShot {
    /// The camera at `t`: keyframes blend in turn, each over its own duration.
    fn camera(&self, t: f32) -> (LonLat, f64) {
        let first = &self.keyframes[0];
        let (mut center, mut span) = (first.center, first.span);
        for pair in self.keyframes.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            let k = f64::from(smooth((t - b.at) / b.dur));
            if k > 0.0 {
                center = [a.center[0] + (b.center[0] - a.center[0]) * k, a.center[1] + (b.center[1] - a.center[1]) * k];
                span = (a.span.ln() + (b.span.ln() - a.span.ln()) * k).exp();
            }
        }
        (center, span)
    }

    fn projection(&self, stage: &Stage, t: f32) -> Projection {
        let (center, span) = self.camera(t);
        // A slow push-in over the shot.
        let drift = 1.0 + 0.02 * f64::from((t / self.duration).min(1.0));
        let width = f64::from(stage.film.width);
        Projection {
            size: (stage.film.width as f32, stage.film.height as f32),
            center,
            cos: (center[1].to_radians()).cos(),
            scale: width / (span / drift),
        }
    }

    fn draw_land(&self, canvas: &Canvas, proj: &Projection, t: f32) {
        for (name, rings) in &self.geo.countries.0 {
            let path = proj.polygon(rings, true);
            canvas.draw_path(&path, &fill(if self.home.contains(name) { HOME_LAND } else { LAND }));
            let mut outline = stroke(BORDER, 1.6);
            outline.set_stroke_join(PaintJoin::Round);
            canvas.draw_path(&path, &outline);
        }
        for (name, rings) in &self.geo.states.0 {
            let path = proj.polygon(rings, true);
            let Some(area) = self.areas.iter().find(|a| &a.state == name) else {
                let mut line = stroke(STATE_LINE, 1.0);
                dashed(&mut line, 5.0, 6.0);
                canvas.draw_path(&path, &line);
                continue;
            };
            let k = ease((t - area.at) / 0.8);
            canvas.draw_path(&path, &fill(Color::from_argb((0.32 * k * 255.0).round() as u8, 224, 169, 59)));
            let line = if k > 0.0 { stroke(OCHRE, 2.5) } else { stroke(STATE_LINE, 1.0) };
            canvas.draw_path(&path, &line);
        }
        let mut river = stroke(RIVER, 2.4);
        river.set_stroke_cap(PaintCap::Round).set_stroke_join(PaintJoin::Miter);
        with_opacity(canvas, 0.85, || {
            for (name, lines) in &self.geo.rivers.0 {
                if RIVERS_SHOWN.contains(&name.as_str()) {
                    canvas.draw_path(&proj.polygon(lines, false), &river);
                }
            }
        });
    }

    fn draw_schematic(&self, canvas: &Canvas, proj: &Projection, t: f32) {
        for element in &self.schematic {
            let k = ease((t - element.at) / 0.9);
            if k <= 0.0 {
                continue;
            }
            match &element.terrain {
                Terrain::Hills(blobs) => with_blur(canvas, 14.0, k, || {
                    let km = proj.km_px();
                    for (place, radius) in blobs {
                        let c = proj.point(*place);
                        let r = radius * km;
                        let mut ridge = fill(Color::from_argb(107, 0xC9, 0xB4, 0x88));
                        canvas.draw_oval(Rect::from_xywh(c.x - r * 1.2, c.y - r * 0.75, r * 2.4, r * 1.5), &ridge);
                        ridge.set_color(Color::from_argb(77, 0xB3, 0x9A, 0x6C));
                        let (cx, cy) = (c.x - r * 0.15, c.y - r * 0.12);
                        canvas.draw_oval(Rect::from_xywh(cx - r * 0.6, cy - r * 0.36, r * 1.2, r * 0.72), &ridge);
                    }
                }),
                Terrain::Lake(ring) => with_opacity(canvas, k, || {
                    let path = proj.polygon(std::slice::from_ref(ring), true);
                    canvas.draw_path(&path, &fill(Color::from_rgb(0x8D, 0xB3, 0xC0)));
                    canvas.draw_path(&path, &stroke(Color::from_rgb(0x5F, 0x8D, 0x9D), 2.0));
                }),
                Terrain::Grid([a, b]) => self.draw_grid(canvas, proj, element.at, t, k, [*a, *b]),
            }
        }
    }

    fn draw_grid(&self, canvas: &Canvas, proj: &Projection, at: f32, t: f32, k: f32, corners: [LonLat; 2]) {
        const CELLS: u32 = 3;
        let (a, b) = (proj.point(corners[0]), proj.point(corners[1]));
        let (x0, x1, y0, y1) = (a.x.min(b.x), a.x.max(b.x), a.y.min(b.y), a.y.max(b.y));
        let (cell_w, cell_h) = ((x1 - x0) / CELLS as f32, (y1 - y0) / CELLS as f32);
        for gx in 0..CELLS {
            for gy in 0..CELLS {
                let kk = ease((t - at - (gx + gy) as f32 * 0.12) / 0.5);
                let centre = gx == 1 && gy == 1;
                let tint = if centre { Color::from_argb(140, 180, 72, 47) } else { Color::from_argb(115, 224, 169, 59) };
                let cell = Rect::from_xywh(x0 + cell_w * gx as f32 + 4.0, y0 + cell_h * gy as f32 + 4.0, cell_w - 8.0, cell_h - 8.0);
                with_opacity(canvas, kk, || {
                    canvas.draw_rect(cell, &fill(tint));
                    canvas.draw_rect(cell, &stroke(TERRACOTTA, 2.0));
                });
            }
        }
        with_opacity(canvas, k, || {
            canvas.draw_rect(Rect::new(x0, y0, x1, y1), &stroke(INK, 4.0));
        });
    }

    fn draw_rings(&self, canvas: &Canvas, proj: &Projection, t: f32) {
        for ring in &self.rings {
            let k = ease((t - ring.at) / 1.0);
            if k <= 0.0 {
                continue;
            }
            let c = proj.point(ring.center);
            let radius = ring.km * proj.km_px();
            with_opacity(canvas, k, || {
                let r = radius * (0.85 + 0.15 * k);
                canvas.draw_circle(c, r, &fill(Color::from_argb(26, 180, 72, 47)));
                let mut edge = stroke(TERRACOTTA, 3.0);
                dashed(&mut edge, 14.0, 10.0);
                canvas.draw_circle(c, r, &edge);
                if let Some(label) = &ring.label {
                    label.draw_on_baseline(canvas, c.x - label.content_width() / 2.0, c.y - radius - 18.0);
                }
            });
        }
    }

    fn draw_routes(&self, canvas: &Canvas, proj: &Projection, t: f32) {
        for route in &self.routes {
            let k = ease((t - route.at) / route.dur);
            if k <= 0.0 {
                continue;
            }
            // A route bends between successive places: a sampled quadratic curve.
            let mut samples: Vec<Point> = Vec::new();
            for pair in route.via.windows(2) {
                let (a, b) = (proj.point(pair[0]), proj.point(pair[1]));
                let (mid, d) = (Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0), Point::new(b.x - a.x, b.y - a.y));
                let control = Point::new(mid.x - d.y * route.bend, mid.y + d.x * route.bend);
                for q in 0..=40 {
                    let u = q as f32 / 40.0;
                    let w = [(1.0 - u) * (1.0 - u), 2.0 * (1.0 - u) * u, u * u];
                    samples.push(Point::new(w[0] * a.x + w[1] * control.x + w[2] * b.x, w[0] * a.y + w[1] * control.y + w[2] * b.y));
                }
            }
            let shown = ((samples.len() as f32 * k + 0.5).floor() as usize).max(2).min(samples.len());
            let visible = &samples[..shown];
            let (head, behind) = (visible[shown - 1], visible[shown.saturating_sub(4)]);
            let angle = (head.y - behind.y).atan2(head.x - behind.x).to_degrees();

            let mut line = stroke(route.color, 6.0);
            line.set_stroke_cap(PaintCap::Round);
            if route.dashed {
                dashed(&mut line, 16.0, 12.0);
            }
            canvas.draw_path(Path::new().add_poly(visible, false), &line);

            canvas.save();
            canvas.translate((head.x, head.y));
            canvas.rotate(angle, None);
            let mut arrow = Path::new();
            arrow.move_to((-16.0, -11.0)).line_to((4.0, 0.0)).line_to((-16.0, 11.0)).close();
            canvas.draw_path(&arrow, &fill(route.color));
            canvas.restore();

            if let (Some(label), true) = (&route.label, k > 0.6) {
                let mid = visible[visible.len() / 2];
                let x = mid.x + route.label_offset.0 - label.content_width() / 2.0;
                with_opacity(canvas, ease((k - 0.6) / 0.4), || {
                    label.draw_on_baseline(canvas, x, mid.y - 18.0 + route.label_offset.1);
                });
            }
        }
    }

    fn draw_pins(&self, canvas: &Canvas, proj: &Projection, t: f32) {
        for pin in &self.pins {
            let k = ease((t - pin.at) / 0.5);
            if k <= 0.0 {
                continue;
            }
            let c = proj.point(pin.at_place);
            with_opacity(canvas, k, || {
                if pin.home {
                    let pulse = (t - pin.at).rem_euclid(2.0) / 2.0;
                    let mut ring = stroke(TERRACOTTA, 3.0);
                    ring.set_alpha_f(1.0 - pulse);
                    canvas.draw_circle(c, 14.0 + 40.0 * pulse, &ring);
                }
                let r = if pin.home { 13.0 } else { 9.0 } * (0.6 + 0.4 * k);
                canvas.draw_circle(c, r, &fill(if pin.home { TERRACOTTA } else { INK }));
                canvas.draw_circle(c, r, &stroke(PIN_RIM, 4.0));
                let (dx, dy) = pin.offset;
                let anchor = |block: &TextBlock| if dx < 0.0 { c.x + dx - block.content_width() } else { c.x + dx };
                pin.label.draw_on_baseline(canvas, anchor(&pin.label), c.y + dy);
                if let Some(sub) = &pin.sub {
                    sub.draw_on_baseline(canvas, anchor(sub), c.y + dy + 34.0);
                }
            });
        }
    }

    fn draw_labels(&self, canvas: &Canvas, proj: &Projection, t: f32) {
        for label in &self.labels {
            let k = ease((t - label.at) / 0.8) * if label.faint { 0.55 } else { 0.9 };
            let c = proj.point(label.at_place);
            with_opacity(canvas, k, || {
                label.text.draw_on_baseline(canvas, c.x - label.text.content_width() / 2.0, c.y);
            });
        }
    }

    /// The paper grain multiplied over the map, and a warm inset shadow around the frame.
    fn draw_paper(&self, canvas: &Canvas, size: (f32, f32)) {
        let frame = Rect::from_wh(size.0, size.1);
        let mut grain = Paint::default();
        grain.set_alpha_f(0.45).set_blend_mode(skia_safe::BlendMode::Multiply);
        draw_fit(canvas, &self.grain, frame, &grain);

        let mut vignette = Path::new();
        vignette.add_rect(frame.with_outset((1000.0, 1000.0)), None).add_rect(frame, None).set_fill_type(PathFillType::EvenOdd);
        let mut shadow = fill(Color::from_argb(89, 70, 45, 20));
        shadow.set_mask_filter(MaskFilter::blur(BlurStyle::Normal, 110.0, false));
        canvas.save();
        canvas.clip_rect(frame, None, true);
        canvas.draw_path(&vignette, &shadow);
        canvas.restore();
    }

    fn draw_captions(&self, canvas: &Canvas, size: (f32, f32), t: f32) {
        if let Some(title) = &self.title {
            caption::draw_tag(canvas, title);
        }
        if let Some(note) = &self.note {
            with_opacity(canvas, ease((t - 0.8) / 0.6), || {
                note.draw(canvas, Point::new(caption::MARGIN, size.1 - 58.0 - note.height));
            });
        }
        caption::draw_legend(canvas, &self.legend, size, 60.0);
    }
}

impl Scene for MapShot {
    fn draw(&self, canvas: &Canvas, stage: &Stage, at: Moment) -> Result<()> {
        let t = at.seconds;
        let proj = self.projection(stage, t);
        canvas.clear(WATER);
        self.draw_land(canvas, &proj, t);
        self.draw_schematic(canvas, &proj, t);
        self.draw_rings(canvas, &proj, t);
        self.draw_routes(canvas, &proj, t);
        self.draw_pins(canvas, &proj, t);
        self.draw_labels(canvas, &proj, t);
        self.draw_paper(canvas, proj.size);
        self.draw_captions(canvas, proj.size, t);
        Ok(())
    }
}
