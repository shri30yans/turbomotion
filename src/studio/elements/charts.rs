//! Data graphics: the growing line chart, 3D-look bars, the jar of cash and the grid of stock tiles.

use anyhow::{ensure, Context, Result};
use skia_safe::{BlendMode, Canvas, Color, Paint, PaintStyle, Path, RRect, Rect};

use crate::studio::{
    anim::{blit, Blit, Timing},
    color::{fill, fill_a, mix, skia_a},
    ease::{self, clamp, expo_io, expo_out, keyed},
    fx::Bokeh,
    layer::{over, place, Layer, Place},
    model::{Bars, Jar, LineChart, Tiles, GREEN, RED, STEEL, WHITE},
    pyfmt,
    rng::RandomState,
    stage::Stage,
    typeset::TextSpec,
};

/// The visible part of a line chart's series: `(x, y)` in chart pixels, y measured down from the top.
struct Series {
    xs: Vec<f64>,
    ys: Vec<f64>,
    lo: f64,
    hi: f64,
}

fn series(e: &LineChart, tm: &Timing, w: f64, h: f64) -> Result<Series> {
    if let (Some(keys), Some(span)) = (&e.keys, e.span) {
        let (lo, hi) = (e.ymin.context("a keyed line chart needs ymin")?, e.ymax.context("a keyed line chart needs ymax")?);
        let end = tm.t.min(span[1]);
        let n = (((end - span[0]) * 30.0) as usize).max(2);
        let stop = end.max(span[0] + 1e-3);
        let times: Vec<f64> = (0..n).map(|i| span[0] + (stop - span[0]) * i as f64 / (n - 1) as f64).collect();
        let xs = times.iter().map(|t| (t - span[0]) / (span[1] - span[0]) * w).collect();
        let ys = times.iter().map(|t| h - (keyed(keys, *t) - lo) / (hi - lo) * h).collect();
        return Ok(Series { xs, ys, lo, hi });
    }
    let data = e.data.as_ref().context("a line chart needs `data` or `keys` with `span`")?;
    let u = ease::named(&e.ease, (tm.t - tm.t0()) / e.draw).with_context(|| format!("unknown ease {:?}", e.ease))?;
    let (min, max) = data.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    let (lo, hi) = (e.ymin.unwrap_or(min * 0.96), e.ymax.unwrap_or(max * 1.04));
    let n = data.len();
    let at = |i: usize| (w * i as f64 / (n - 1) as f64, h - (data[i] - lo) / (hi - lo) * h);
    let k = u * (n - 1) as f64;
    let i = k as usize;
    let mut pts: Vec<(f64, f64)> = (0..=i.min(n - 1)).map(at).collect();
    if i < n - 1 {
        let (a, b, f) = (at(i), at(i + 1), k - i as f64);
        pts.push((a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f));
    }
    Ok(Series { xs: pts.iter().map(|p| p.0).collect(), ys: pts.iter().map(|p| p.1).collect(), lo, hi })
}

/// A line chart that draws its history: from a fixed `data` series over `draw` seconds, or word-synced from keyed values.
pub fn linechart(st: &Stage, cv: &Canvas, e: &LineChart, tm: &Timing) -> Result<()> {
    let [x0, y0, x1, y1] = e.rect;
    let (w, h) = (x1 - x0, y1 - y0);
    let uo = tm.out_prog();
    if tm.t < tm.t0() || uo >= 1.0 {
        return Ok(());
    }
    let s = series(e, tm, w, h)?;
    ensure!(s.xs.len() >= 2, "a line chart needs two points");
    let baseline_y = e.baseline.map(|b| h - (b - s.lo) / (s.hi - s.lo) * h);
    let below = |y: f64| baseline_y.is_some_and(|b| b < y);
    const MARGIN: f64 = 40.0;
    let layer = Layer::render(cv, w as i32 + 80, h as i32 + 80, |c| {
        let mut grid = fill_a(STEEL, 40.0 / 255.0);
        grid.set_style(PaintStyle::Stroke).set_stroke_width(1.0);
        for gy in 0..5 {
            let y = (MARGIN + f64::from(gy) * h / 4.0) as f32;
            c.draw_line((MARGIN as f32, y), ((MARGIN + w) as f32, y), &grid);
        }
        if let Some(by) = baseline_y {
            let mut base = fill_a(WHITE, 110.0 / 255.0);
            base.set_style(PaintStyle::Stroke).set_stroke_width(1.5);
            c.draw_line((MARGIN as f32, (MARGIN + by) as f32), ((MARGIN + w) as f32, (MARGIN + by) as f32), &base);
        }
        let pt = |i: usize| ((MARGIN + s.xs[i]) as f32, (MARGIN + s.ys[i]) as f32);
        let mut area = Path::new();
        area.move_to(pt(0));
        (1..s.xs.len()).for_each(|i| {
            area.line_to(pt(i));
        });
        area.line_to(((MARGIN + s.xs[s.xs.len() - 1]) as f32, (MARGIN + h) as f32)).line_to(((MARGIN + s.xs[0]) as f32, (MARGIN + h) as f32)).close();
        c.draw_path(&area, &fill_a(e.fill, 28.0 / 255.0));
        let mut line = fill(e.fill);
        line.set_style(PaintStyle::Stroke).set_stroke_width(7.0);
        for j in 0..s.xs.len() - 1 {
            line.set_color(if below(s.ys[j]) { color_of(e.fill2) } else { color_of(e.fill) });
            c.draw_line(pt(j), pt(j + 1), &line);
        }
    })?;
    over(cv, &layer, (x0 - MARGIN) as i32, (y0 - MARGIN) as i32, tm.opacity() * (1.0 - uo));
    let (hx, hy) = (x0 + s.xs[s.xs.len() - 1], y0 + s.ys[s.ys.len() - 1]);
    let head = if below(s.ys[s.ys.len() - 1]) { e.fill2 } else { e.fill };
    Bokeh::chart_head(head).draw(st, cv, hx, hy, 1.0 - uo)?;
    if let Some(template) = &e.value_label {
        let value = s.lo + (h - s.ys[s.ys.len() - 1]) / h * (s.hi - s.lo);
        let label = st.type_.layer(cv, &TextSpec::new(pyfmt::format_v(template, value)?, "dinc", e.vsize, WHITE).pad(4))?;
        place(cv, &label, &Place::at(hx + 20.0, hy - 54.0).opacity(1.0 - uo));
    }
    Ok(())
}

fn color_of(c: [u8; 3]) -> Color {
    Color::from_rgb(c[0], c[1], c[2])
}

/// Bars with a 3D look that grow in one after another.
pub fn bars(st: &Stage, cv: &Canvas, e: &Bars, tm: &Timing) -> Result<()> {
    let [x0, y0, x1, y1] = e.rect;
    let (w, h) = (x1 - x0, y1 - y0);
    let uo = tm.out_prog();
    if uo >= 1.0 {
        return Ok(());
    }
    let vmax = e.vmax.unwrap_or_else(|| e.items.iter().map(|i| i.v).fold(f64::MIN, f64::max));
    let n = e.items.len() as f64;
    let bw = w / (n + (n + 1.0) * e.gap);
    let grown: Vec<f64> = e.items.iter().enumerate().map(|(i, it)| expo_out(clamp((tm.t - it.t0.unwrap_or(tm.t0() + i as f64 * 0.35)) / e.grow))).collect();
    const ORIGIN: f64 = 100.0;
    let (dx, dy) = (e.depth * 0.6, -e.depth * 0.45);
    let layer = Layer::render(cv, (w + 200.0) as i32, (h + 200.0) as i32, |c| {
        let mut axis = fill_a(STEEL, 150.0 / 255.0);
        axis.set_style(PaintStyle::Stroke).set_stroke_width(1.5);
        c.draw_line((ORIGIN as f32, (ORIGIN + h) as f32), ((ORIGIN + w) as f32, (ORIGIN + h) as f32), &axis);
        for (i, it) in e.items.iter().enumerate() {
            let u = grown[i];
            if u <= 0.0 {
                continue;
            }
            let bh = h * it.v / vmax * u;
            let x = ORIGIN + e.gap * bw + i as f64 * (bw + e.gap * bw);
            let (yb, yt) = (ORIGIN + h, ORIGIN + h - bh);
            let base = it.fill.unwrap_or(STEEL);
            let poly = |pts: [(f64, f64); 4], color: [u8; 3]| {
                let mut p = Path::new();
                p.move_to((pts[0].0 as f32, pts[0].1 as f32));
                pts[1..].iter().for_each(|q| {
                    p.line_to((q.0 as f32, q.1 as f32));
                });
                p.close();
                c.draw_path(&p, &fill(color));
            };
            poly([(x + bw, yt), (x + bw + dx, yt + dy), (x + bw + dx, yb + dy), (x + bw, yb)], mix(base, [0, 0, 0], 0.45));
            poly([(x, yt), (x + dx, yt + dy), (x + bw + dx, yt + dy), (x + bw, yt)], mix(base, [255, 255, 255], 0.35));
            c.draw_rect(Rect::from_ltrb(x as f32, yt as f32, (x + bw) as f32, yb as f32), &fill(base));
            c.draw_rect(Rect::from_ltrb(x as f32, yt as f32, (x + 8.0) as f32, yb as f32), &fill(mix(base, [255, 255, 255], 0.25)));
        }
    })?;
    over(cv, &layer, (x0 - ORIGIN) as i32, (y0 - ORIGIN) as i32, tm.opacity() * (1.0 - uo));
    for (i, it) in e.items.iter().enumerate() {
        let u = grown[i];
        if u <= 0.0 {
            continue;
        }
        let bx = x0 + e.gap * bw + i as f64 * (bw + e.gap * bw) + bw / 2.0;
        let bh = h * it.v / vmax * u;
        let label = st.type_.layer(cv, &TextSpec::new(it.label.to_uppercase(), "avn_b", e.lsize, STEEL).tracking(2.0).maxw(Some((bw * 2.2).trunc())).pad(4))?;
        place(cv, &label, &Place::at(bx, y1 + 36.0).opacity(u * (1.0 - uo)));
        let value = st.type_.layer(cv, &TextSpec::new(&it.text, "dinc", e.vsize, WHITE).pad(4))?;
        place(cv, &value, &Place::at(bx + 12.0, y1 - bh - 52.0).opacity(clamp((u - 0.5) * 2.0) * (1.0 - uo)));
    }
    Ok(())
}

/// A glass jar whose liquid level follows keyed values.
pub fn jar(st: &Stage, cv: &Canvas, e: &Jar, tm: &Timing) -> Result<()> {
    let uo = tm.out_prog();
    let u = tm.prog(None);
    if u <= 0.0 || uo >= 1.0 {
        return Ok(());
    }
    let level = keyed(&e.levels, tm.t);
    const PAD: f64 = 40.0;
    let (w, h) = (e.w, e.h);
    let layer = Layer::render(cv, (w + 2.0 * PAD) as i32, (h + 2.0 * PAD) as i32, |c| {
        let (left, top, right, bottom) = (PAD as f32, PAD as f32, (PAD + w) as f32, (PAD + h) as f32);
        let rrect = |l: f32, t: f32, r: f32, b: f32, radius: f32| RRect::new_rect_xy(Rect::from_ltrb(l, t, r, b), radius, radius);
        let fill_h = (h * level) as f32;
        let replace = |mut p: Paint| {
            p.set_blend_mode(BlendMode::Src);
            p
        };
        if fill_h > 1.0 {
            let wobble = ((tm.t * 3.0).sin() * 3.0) as f32;
            c.draw_rrect(rrect(left + 4.0, bottom - fill_h, right - 4.0, bottom - 4.0, 9.0), &replace(fill_a(e.fill, 205.0 / 255.0)));
            let mut surface = Paint::default();
            surface.set_anti_alias(true).set_color4f(skia_a(mix(e.fill, WHITE, 0.4), 230.0 / 255.0), None).set_blend_mode(BlendMode::Src);
            c.draw_oval(Rect::from_ltrb(left + 4.0, bottom - fill_h - 6.0 + wobble, right - 4.0, bottom - fill_h + 6.0 + wobble), &surface);
        }
        let mut glass = fill_a(WHITE, 190.0 / 255.0);
        glass.set_style(PaintStyle::Stroke).set_stroke_width(2.5);
        c.draw_rrect(rrect(left + 1.25, top + 1.25, right - 1.25, bottom - 1.25, 15.0), &replace(glass));
        c.draw_rrect(rrect(left + 10.0, top + 9.0, left + 17.0, bottom - 20.0, 3.0), &replace(fill_a(WHITE, 60.0 / 255.0)));
        c.draw_rrect(rrect(left + 12.0, top - 8.0, right - 12.0, top + 3.0, 4.0), &replace(fill(STEEL)));
    })?;
    blit(cv, &layer, e.x, e.y, tm, Blit::new("rise"))?;
    if let Some(label) = &e.label {
        let text = st.type_.layer(cv, &TextSpec::new(label.to_uppercase(), "avn_b", 30.0, WHITE).tracking(4.0).pad(4))?;
        place(cv, &text, &Place::at(e.x, e.y + e.h / 2.0 + 62.0).opacity(clamp(u * 3.0) * (1.0 - uo)));
    }
    Ok(())
}

/// A grid of stock tiles that flicker green and red, then gather into a single index tile.
pub fn tiles(cv: &Canvas, e: &Tiles, tm: &Timing) -> Result<()> {
    let (cols, rows) = (e.cols as usize, e.rows as usize);
    let [x0, y0, x1, y1] = e.rect;
    let (tw, th) = ((x1 - x0) / cols as f64, (y1 - y0) / rows as f64);
    let mut rng = RandomState::new(5);
    let phase: Vec<f64> = (0..rows * cols).map(|_| rng.rand() * 6.28).collect();
    let speed: Vec<f64> = (0..rows * cols).map(|_| 0.6 + rng.rand() * 1.4).collect();
    let merge_t = e.merge_at.unwrap_or(1e9);
    let uo = tm.out_prog();
    if uo >= 1.0 {
        return Ok(());
    }
    let mk = if tm.t >= merge_t { clamp((tm.t - merge_t) / 0.9) } else { 0.0 };
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let layer = Layer::render(cv, (x1 - x0) as i32 + 40, (y1 - y0) as i32 + 40, |c| {
        let mut paint = Paint::default();
        paint.set_anti_alias(false).set_blend_mode(BlendMode::Src);
        for r in 0..rows {
            for col in 0..cols {
                let idx = r * cols + col;
                let appear = clamp((tm.t - tm.t0() - idx as f64 * 0.004) / 0.4);
                if appear <= 0.0 {
                    continue;
                }
                let v = (tm.t * speed[idx] + phase[idx]).sin();
                let up = if tm.t < merge_t { v > 0.0 } else { true };
                let k = 0.35 + 0.35 * v.abs();
                let base = if up { GREEN } else { RED };
                let shade = base.map(|ch| (f64::from(ch) * k + 14.0 * (1.0 - k)) as u8);
                let gx = x0 + col as f64 * tw + tw / 2.0;
                let gy = y0 + r as f64 * th + th / 2.0;
                let gx = gx + (cx - gx) * expo_io(mk) * 0.9;
                let gy = gy + (e.merge_y.unwrap_or(cy) - gy) * expo_io(mk);
                let s = (1.0 - 0.9 * expo_io(mk)) * appear;
                let (px, py) = (gx - x0 + 20.0, gy - y0 + 20.0);
                let (left, top) = ((px - tw * 0.44 * s) as i32, (py - th * 0.44 * s) as i32);
                let (right, bottom) = ((px + tw * 0.44 * s) as i32, (py + th * 0.44 * s) as i32);
                if right > left && bottom > top {
                    paint.set_color(Color::from_rgb(shade[0], shade[1], shade[2]));
                    c.draw_rect(Rect::from_ltrb(left as f32, top as f32, (right + 1) as f32, (bottom + 1) as f32), &paint);
                }
            }
        }
    })?;
    over(cv, &layer, (x0 - 20.0) as i32, (y0 - 20.0) as i32, (1.0 - uo) * tm.opacity());
    Ok(())
}
