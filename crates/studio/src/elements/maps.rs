//! The flat Mercator map and the orthographic globe: land, highlighted regions, route arcs, pins and labels.

use anyhow::{anyhow, bail, Result};
use skia_safe::{Canvas, Image, Paint, PaintStyle, Path, Point, Rect, TileMode};

use crate::{{anim::{Timing}, atlas::{tint, TEXTURE}, color::{fill}, fx::{Bokeh}, geo::{great_circle, Globe, RoutePoint, View}, model::{GlobeArc, GlobeEl, GlobeHighlight, GlobePin, MapArc, MapEl, MapPin, Rgb, ViewKey, H, W, WHITE}, stage::{Stage}, typeset::{Align, TextSpec}}};
use turbomotion_core::{ease::{back_out, clamp, expo_io, expo_out, pow_io}, layer::{linear, over, place, Layer, Place}, shader::{Uniforms}};

/// Seconds a highlighted region takes to fade in on the map.
const FILL_SECONDS: f64 = 0.7;
/// Seconds a pin takes to land.
const MAP_PIN_SECONDS: f64 = 0.4;
/// Highlighted countries a globe can show at once (one texture slot each in `globe.sksl`).
const MAX_HIGHLIGHTS: usize = 4;
/// Vertices of a route sampled along its great circle.
const MAP_ROUTE_STEPS: usize = 60;
const GLOBE_ROUTE_STEPS: usize = 80;

/// The window `views` describes at `t`: held at the first/last key, eased between keys (centre linear, span geometric).
fn view_at(views: &[(f64, ViewKey)], t: f64) -> Result<View> {
    let Some(&(first_t, first)) = views.first() else { bail!("a map needs at least one view") };
    if t <= first_t {
        return Ok(View::new(first));
    }
    for pair in views.windows(2) {
        let ((ta, a), (tb, b)) = (pair[0], pair[1]);
        if t < tb {
            return Ok(View::new(View::between(a, b, expo_io((t - ta) / (tb - ta).max(1e-3)))));
        }
    }
    Ok(View::new(views[views.len() - 1].1))
}

/// A label with a soft black shadow, centred on (`cx`, `cy`): the shadow sits 2 px lower and blurs by 3.
fn label(st: &Stage, c: &Canvas, text: &str, size: f64, tracking: f64, (cx, cy): (f64, f64), shadow_opacity: f64, text_opacity: f64) -> Result<()> {
    let spec = |fill: Rgb| TextSpec::new(text, "avn_b", size, fill).tracking(tracking).align(Align::Center).pad(6);
    let face = st.type_.layer(c, &spec(WHITE))?;
    let shadow = st.type_.layer(c, &spec([0, 0, 0]))?.blurred(c, 3.0)?;
    let (left, top) = ((cx - f64::from(face.width()) / 2.0) as i32, (cy - f64::from(face.height()) / 2.0) as i32);
    over(c, &shadow, left, top + 2, shadow_opacity);
    over(c, &face, left, top, text_opacity);
    Ok(())
}

fn stroke(color: Rgb, width: f64) -> Paint {
    let mut paint = fill(color);
    paint.set_style(PaintStyle::Stroke).set_stroke_width(width as f32).set_stroke_join(skia_safe::PaintJoin::Round);
    paint
}

fn polyline(points: &[(f64, f64)]) -> Path {
    let points: Vec<Point> = points.iter().map(|&(x, y)| Point::new(x as f32, y as f32)).collect();
    Path::polygon(&points, false, None, None)
}

pub fn map(st: &Stage, cv: &Canvas, e: &MapEl, tm: &Timing) -> Result<()> {
    let u0 = tm.prog(Some(0.8));
    let uo = tm.out_prog();
    if u0 <= 0.0 || uo >= 1.0 {
        return Ok(());
    }
    let t = tm.t;
    let view = view_at(&e.views, t)?;
    let base = st.atlas.base(&st.shaders, cv, &view, e.land, e.border)?;
    let geo = st.atlas.geo()?;
    let layer = Layer::try_render(cv, W, H, |c| {
        over(c, &base, 0, 0, 1.0);
        for f in &e.fills {
            let ua = clamp((t - f.at) / FILL_SECONDS);
            if ua <= 0.0 {
                continue;
            }
            let rings = match (&f.countries, &f.states) {
                (Some(names), None) => geo.country_rings(names)?,
                (None, Some(names)) => geo.state_rings(names)?,
                _ => bail!("a map fill names either countries or states"),
            };
            tint(c, &st.atlas.region(&st.shaders, cv, &view, &rings)?, f.fill, (f.a * ua) as f32);
            tint(c, &st.atlas.outline(&st.shaders, cv, &view, &rings, 2.0)?, f.edge, 1.0);
        }
        for arc in &e.arcs {
            draw_map_arc(c, &view, arc, t);
        }
        for pin in &e.pins {
            draw_map_pin(st, c, &view, pin, t)?;
        }
        Ok(())
    })?;
    over(cv, &layer, 0, 0, tm.opacity() * (1.0 - uo) * clamp(u0));
    Ok(())
}

fn draw_map_arc(c: &Canvas, view: &View, arc: &MapArc, t: f64) {
    let ua = pow_io(clamp((t - arc.at) / arc.dur), 3.0);
    if ua <= 0.0 {
        return;
    }
    let route = great_circle(arc.a, arc.b, MAP_ROUTE_STEPS, 0.0);
    let last = (route.len() - 1) as f64;
    let shown = ((route.len() as f64 * ua) as usize).max(2);
    let points: Vec<(f64, f64)> = route
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (x, y) = view.xy(p.lon, p.lat);
            (x, y - arc.lift * (std::f64::consts::PI * i as f64 / last).sin())
        })
        .take(shown)
        .collect();
    c.draw_path(&polyline(&points), &stroke(arc.fill, arc.w));
}

fn draw_map_pin(st: &Stage, c: &Canvas, view: &View, pin: &MapPin, t: f64) -> Result<()> {
    let ua = clamp((t - pin.at) / MAP_PIN_SECONDS);
    if ua <= 0.0 {
        return Ok(());
    }
    let radius = pin.r * (0.4 + 0.6 * back_out(ua, 1.70158));
    let pulse = ((t - pin.at) % 1.6) / 1.6 * pin.pulse;
    let (x, y) = view.xy(pin.lon, pin.lat);
    Bokeh::map_pin(pin.fill, radius, pulse).draw(st, c, x, y, 1.0)?;
    if let Some(text) = &pin.label {
        label(st, c, text, pin.lsize, 1.0, (x + pin.dx, y + pin.dy), 1.0, clamp(ua * 1.5))?;
    }
    Ok(())
}

/// The globe's facing at `t`: held at the end keys, eased between them, plus the constant spin since the element began.
fn facing_at(keys: &[(f64, [f64; 2])], t: f64, spin: f64, t0: f64) -> Result<[f64; 2]> {
    let Some(&(first_t, first)) = keys.first() else { bail!("a globe needs at least one rotation key") };
    let mut at = keys[keys.len() - 1].1;
    if t <= first_t {
        at = first;
    } else if let Some(pair) = keys.windows(2).find(|p| t < p[1].0) {
        let ((ta, a), (tb, b)) = (pair[0], pair[1]);
        let u = expo_io((t - ta) / (tb - ta).max(1e-3));
        at = [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u];
    }
    Ok([at[0] + spin * (t - t0), at[1]])
}

pub fn globe(st: &Stage, cv: &Canvas, e: &GlobeEl, tm: &Timing) -> Result<()> {
    let u0 = tm.prog(Some(0.9));
    let uo = tm.out_prog();
    if u0 <= 0.0 || uo >= 1.0 {
        return Ok(());
    }
    let t = tm.t;
    let [lon, lat] = facing_at(&e.rot, t, e.spin, tm.t0())?;
    let side = e.diameter as i32;
    let view = Globe { d: f64::from(side), lon, lat };
    let layer = Layer::try_render(cv, side, side, |c| {
        draw_sphere(st, c, &view, &e.highlights, t)?;
        for arc in &e.arcs {
            draw_globe_arc(c, &view, arc, t);
        }
        for arc in &e.arcs {
            draw_globe_packets(st, c, &view, arc, t)?;
        }
        for pin in &e.pins {
            draw_globe_pin(st, c, &view, pin, t)?;
        }
        Ok(())
    })?;
    let scale = 1.0 + 0.03 * (1.0 - expo_out(u0));
    place(cv, &layer, &Place::at(e.x, e.y).scale(scale * e.scale).opacity((1.0 - uo) * clamp(u0)));
    Ok(())
}

fn draw_sphere(st: &Stage, c: &Canvas, view: &Globe, highlights: &[GlobeHighlight], t: f64) -> Result<()> {
    let lit: Vec<(&GlobeHighlight, f64)> = highlights.iter().map(|h| (h, clamp((t - h.at) / 0.6))).filter(|(_, ua)| *ua > 0.0).collect();
    if lit.len() > MAX_HIGHLIGHTS {
        bail!("a globe highlights at most {MAX_HIGHLIGHTS} regions at a time, this one shows {}", lit.len());
    }
    let tiled = |image: &Image| image.to_shader((TileMode::Repeat, TileMode::Repeat), linear(), None).ok_or_else(|| anyhow!("globe texture shader"));
    let textures = st.atlas.equirect()?;
    let masks = lit.iter().map(|(h, _)| st.atlas.countries_texture(&h.countries)).collect::<Result<Vec<_>>>()?;
    let mut children = vec![tiled(&textures.land)?, tiled(&textures.border)?];
    let mut rows = [[0.0; 4]; MAX_HIGHLIGHTS];
    for slot in 0..MAX_HIGHLIGHTS {
        match lit.get(slot) {
            Some((h, ua)) => {
                children.push(tiled(&masks[slot])?);
                rows[slot] = [f64::from(h.fill[0]) / 255.0, f64::from(h.fill[1]) / 255.0, f64::from(h.fill[2]) / 255.0, h.a * ua];
            }
            None => children.push(tiled(&textures.land)?),
        }
    }
    let (land, sea, border, rim) = ([34, 46, 62], [10, 16, 26], [86, 108, 134], [120, 150, 190]);
    let shader = st.shaders.globe.instance(
        Uniforms::new()
            .f(view.d)
            .v2(view.lon.to_radians(), view.lat.to_radians())
            .v2(f64::from(TEXTURE.0), f64::from(TEXTURE.1))
            .rgb(land)
            .rgb(sea)
            .rgb(border)
            .rgb(rim)
            .rows4(&rows),
        &children,
    )?;
    let mut paint = Paint::default();
    paint.set_shader(shader);
    c.draw_rect(Rect::from_wh(view.d as f32, view.d as f32), &paint);
    Ok(())
}

/// The portion of an arc drawn at progress `ua`, as runs of consecutive visible points.
fn visible_runs(view: &Globe, route: &[RoutePoint], shown: usize) -> Vec<Vec<(f64, f64)>> {
    let mut runs = Vec::new();
    let mut run = Vec::new();
    for p in route.iter().take(shown) {
        let (x, y, visible) = view.project(p.lon, p.lat, p.height);
        if visible {
            run.push((x, y));
        } else {
            if run.len() > 1 {
                runs.push(std::mem::take(&mut run));
            }
            run.clear();
        }
    }
    if run.len() > 1 {
        runs.push(run);
    }
    runs
}

fn draw_globe_arc(c: &Canvas, view: &Globe, arc: &GlobeArc, t: f64) {
    let ua = pow_io(clamp((t - arc.at) / arc.dur), 3.0);
    if ua <= 0.0 {
        return;
    }
    let route = great_circle(arc.a, arc.b, GLOBE_ROUTE_STEPS, arc.lift);
    let shown = ((route.len() as f64 * ua) as usize).max(2);
    let paint = stroke(arc.fill, arc.w);
    for run in visible_runs(view, &route, shown) {
        c.draw_path(&polyline(&run), &paint);
    }
}

/// Packets of money riding along an arc.
fn draw_globe_packets(st: &Stage, c: &Canvas, view: &Globe, arc: &GlobeArc, t: f64) -> Result<()> {
    let Some(packets) = &arc.packets else { return Ok(()) };
    if t < packets.at {
        return Ok(());
    }
    let route = great_circle(arc.a, arc.b, GLOBE_ROUTE_STEPS, arc.lift);
    for i in 0..packets.n {
        let k = ((t - packets.at) * packets.speed + f64::from(i) / f64::from(packets.n)) % 1.0;
        let p = route[(k * (route.len() - 1) as f64) as usize];
        let (x, y, visible) = view.project(p.lon, p.lat, p.height);
        if visible {
            Bokeh::globe_packet(packets.fill).draw(st, c, x, y, (std::f64::consts::PI * k).sin().sqrt())?;
        }
    }
    Ok(())
}

fn draw_globe_pin(st: &Stage, c: &Canvas, view: &Globe, pin: &GlobePin, t: f64) -> Result<()> {
    let ua = clamp((t - pin.at) / MAP_PIN_SECONDS);
    if ua <= 0.0 {
        return Ok(());
    }
    let (x, y, visible) = view.project(pin.lon, pin.lat, 0.0);
    if !visible {
        return Ok(());
    }
    Bokeh::globe_pin(pin.fill).draw(st, c, x, y, ua)?;
    if let Some(text) = &pin.label {
        label(st, c, text, pin.lsize, 2.0, (x + pin.dx, y + pin.dy), ua, ua)?;
    }
    Ok(())
}
