//! Marks that annotate a scene: arrows and the pulses that ride them, packets of money, attention rings, the flash, the
//! scrim and the red danger vignette.

use anyhow::Result;
use skia_safe::{Canvas, Paint, PaintStyle, Path, Rect};

use crate::studio::{
    anim::Timing,
    color::{fill, skia_a},
    ease::{clamp, expo_out},
    fx::{self, Bokeh},
    layer::{over, place, Layer, Place},
    model::{Arrow, Flash, Packets, Ring, Rgb, Scrim, VignetteRed, H, INK, W},
    path,
    shader::Uniforms,
    stage::Stage,
    typeset::TextSpec,
};

fn full() -> Rect {
    Rect::from_wh(W as f32, H as f32)
}

fn tint(c: Rgb, alpha: f64) -> Paint {
    let mut p = Paint::default();
    p.set_color4f(skia_a(c, alpha), None);
    p
}

/// A bright flash that decays quadratically over `dur` seconds from the element's start.
pub fn flash(cv: &Canvas, e: &Flash, tm: &Timing) -> Result<()> {
    let u = tm.t - tm.t0();
    if (0.0..e.dur).contains(&u) {
        cv.draw_rect(full(), &tint(e.fill, (1.0 - u / e.dur).powi(2) * e.a));
    }
    Ok(())
}

/// A dimming wash that fades in over 0.6 s and out with the element.
pub fn scrim(cv: &Canvas, e: &Scrim, tm: &Timing) -> Result<()> {
    let k = tm.prog(Some(0.6)) * (1.0 - tm.out_prog()) * e.a;
    if k > 0.0 {
        cv.draw_rect(full(), &tint(e.fill, k));
    }
    Ok(())
}

/// Red pooling toward the frame edge: danger closing in.
pub fn vignette_red(st: &Stage, cv: &Canvas, e: &VignetteRed, tm: &Timing) -> Result<()> {
    let k = tm.prog(Some(0.5)) * (1.0 - tm.out_prog()) * e.a;
    if k <= 0.0 {
        return Ok(());
    }
    let shader = st.shaders.edge_tint.instance(Uniforms::new().v2(f64::from(W), f64::from(H)).rgb(crate::studio::model::RED).f(k), &[])?;
    let mut paint = Paint::default();
    paint.set_shader(shader);
    cv.draw_rect(full(), &paint);
    Ok(())
}

/// Expanding rings that fade as they grow, staggered 0.22 s apart, for `dur` seconds.
pub fn ring(st: &Stage, cv: &Canvas, e: &Ring, tm: &Timing) -> Result<()> {
    let u = tm.t - tm.t0();
    if u < 0.0 || u > e.dur {
        return Ok(());
    }
    for i in 0..e.n {
        let lag = f64::from(i) * 0.22;
        if u <= lag {
            continue;
        }
        let k = ((u - lag) % 1.1) / 1.1;
        let shader = st.shaders.ring.instance(
            Uniforms::new().v2(e.x, e.y).f(e.r * (1.0 + 1.8 * k)).f(5.0).f(1.0).f((1.0 - k) * 0.8).rgb(e.fill).f(0.0),
            &[],
        )?;
        let mut paint = Paint::default();
        paint.set_shader(shader);
        cv.draw_rect(full(), &paint);
    }
    Ok(())
}

/// A line that draws itself along a smooth path, with an optional arrow head, glow, dashes, riding pulses and a label.
pub fn arrow(st: &Stage, cv: &Canvas, e: &Arrow, tm: &Timing) -> Result<()> {
    let pts = path::sample(&e.pts, 60);
    let u = expo_out(tm.prog(Some(tm.common.d.unwrap_or(0.8))));
    let uo = tm.out_prog();
    if u <= 0.0 || uo >= 1.0 {
        return Ok(());
    }
    let n = ((pts.len() as f64 * u) as usize).max(2);
    let seg = &pts[..n];
    let (min_x, max_x) = seg.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p[0]), hi.max(p[0])));
    let (min_y, max_y) = seg.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p[1]), hi.max(p[1])));
    let x0 = ((min_x - 60.0) as i32).max(0);
    let y0 = ((min_y - 60.0) as i32).max(0);
    let (x1, y1) = (((max_x + 60.0) as i32).min(W), ((max_y + 60.0) as i32).min(H));
    if x1 <= x0 || y1 <= y0 {
        return Ok(());
    }
    let local = |p: &[f64; 2]| ((p[0] - f64::from(x0)) as f32, (p[1] - f64::from(y0)) as f32);
    let line = Layer::render(cv, x1 - x0, y1 - y0, |c| {
        let mut stroke = fill(e.fill);
        stroke.set_style(PaintStyle::Stroke).set_stroke_width(e.w as f32).set_stroke_join(skia_safe::PaintJoin::Round);
        if e.dash {
            // one segment drawn, two skipped
            for i in (0..seg.len() - 1).step_by(3) {
                c.draw_line(local(&seg[i]), local(&seg[(i + 1).min(seg.len() - 1)]), &stroke);
            }
        } else {
            let mut path = Path::new();
            path.move_to(local(&seg[0]));
            seg[1..].iter().for_each(|p| {
                path.line_to(local(p));
            });
            c.draw_path(&path, &stroke);
        }
        if e.head && u > 0.97 && seg.len() >= 4 {
            let (a, b) = (seg[seg.len() - 4], seg[seg.len() - 1]);
            let ang = (b[1] - a[1]).atan2(b[0] - a[0]);
            let tip = local(&b);
            let wing = |da: f64| (tip.0 - (e.hs * (ang + da).cos()) as f32, tip.1 - (e.hs * (ang + da).sin()) as f32);
            let mut head = Path::new();
            head.move_to(tip).line_to(wing(-0.45)).line_to(wing(0.45)).close();
            c.draw_path(&head, &fill(e.fill));
        }
    })?;
    over(cv, &line, x0, y0, tm.opacity() * (1.0 - uo));
    if e.glow {
        let centre = (f64::from(x0) + f64::from(line.width()) / 2.0, f64::from(y0) + f64::from(line.height()) / 2.0);
        fx::halo(st, cv, &line, e.fill, &Place::at(centre.0, centre.1), 14.0, 0.9)?;
    }
    if let Some(count) = e.pulse.filter(|_| u >= 0.99) {
        for i in 0..count {
            let k = ((tm.t - tm.t0()) * e.speed + f64::from(i) / f64::from(count)).rem_euclid(1.0);
            let p = pts[(k * (pts.len() - 1) as f64) as usize];
            Bokeh::pulse(e.pc).draw(st, cv, p[0], p[1], 1.0 - uo)?;
        }
    }
    if let Some(label) = &e.label {
        let mid = pts[pts.len() / 2];
        let text = st.type_.layer(cv, &TextSpec::new(label, "avn_b", e.lsize, e.lfill).tracking(1.0).pad(6))?;
        place(cv, &text, &Place::at(mid[0] + e.lx, mid[1] + e.ly).opacity(clamp((u - 0.6) * 3.0) * (1.0 - uo)));
    }
    Ok(())
}

/// Coins or blocks travelling along a path.
pub fn packets(st: &Stage, cv: &Canvas, e: &Packets, tm: &Timing) -> Result<()> {
    if tm.t < tm.t0() {
        return Ok(());
    }
    let uo = tm.out_prog();
    if uo >= 1.0 {
        return Ok(());
    }
    let pts = path::sample(&e.pts, 120);
    let fade_in = clamp((tm.t - tm.t0()) / 0.4);
    for i in 0..e.n {
        let k = ((tm.t - tm.t0()) * e.speed + f64::from(i) / f64::from(e.n)).rem_euclid(1.0);
        let p = pts[(k * (pts.len() - 1) as f64) as usize];
        let r = e.r * (0.8 + 0.4 * (k * std::f64::consts::PI).sin());
        let opacity = fade_in * (1.0 - uo) * (std::f64::consts::PI * k.min(1.0)).sin().sqrt();
        Bokeh::packet(e.fill, r).draw(st, cv, p[0], p[1], opacity)?;
        if let Some(sym) = &e.sym {
            let mark = st.type_.layer(cv, &TextSpec::new(sym, "avn_b", r * 1.5, INK).pad(2))?;
            place(cv, &mark, &Place::at(p[0], p[1]).opacity(fade_in * (1.0 - uo) * 0.9));
        }
    }
    Ok(())
}
