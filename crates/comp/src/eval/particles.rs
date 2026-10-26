//! Particles as a pure function of time. Particle `k` of a continuous emitter is born at `(k + jitter) / rate`; its
//! life, speed, direction, size and spin come from hashes of `(seed, id)`; and its position at any age is the closed
//! form of velocity, gravity and drag, so a frame needs no history and a still equals the same frame of a render.

use anyhow::{Context, Result};
use serde::Serialize;

use super::{Evaluator, LayerEnv};
use crate::{
    anim::noise::{noise1, rand01},
    plan::{CompPlan, LayerPlan},
    schema::Particles,
};

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub x: f64,
    pub y: f64,
    /// Diameter, px.
    pub size: f64,
    pub rotation: f64,
    /// Colour 0..255 and alpha 0..1 (opacity over life and trail fade already applied).
    pub color: [f32; 4],
    pub vx: f64,
    pub vy: f64,
}

#[derive(Serialize)]
pub struct ParticleState {
    pub count: usize,
    pub shape: String,
    #[serde(skip)]
    pub items: Vec<Particle>,
}

/// A piecewise-linear curve over 0..1 life; flat outside its keys, `default` when empty.
fn curve(keys: &[[f64; 2]], u: f64, default: f64) -> f64 {
    match keys {
        [] => default,
        [only] => only[1],
        _ => {
            if u <= keys[0][0] {
                return keys[0][1];
            }
            for w in keys.windows(2) {
                if u <= w[1][0] {
                    let k = (u - w[0][0]) / (w[1][0] - w[0][0]).max(1e-9);
                    return w[0][1] + (w[1][1] - w[0][1]) * k;
                }
            }
            keys[keys.len() - 1][1]
        }
    }
}

fn gradient(stops: &[(f64, [f32; 4])], u: f64) -> [f32; 4] {
    match stops {
        [] => [255.0, 255.0, 255.0, 1.0],
        [only] => only.1,
        _ => {
            if u <= stops[0].0 {
                return stops[0].1;
            }
            for w in stops.windows(2) {
                if u <= w[1].0 {
                    let k = ((u - w[0].0) / (w[1].0 - w[0].0).max(1e-9)) as f32;
                    return std::array::from_fn(|i| w[0].1[i] + (w[1].1[i] - w[0].1[i]) * k);
                }
            }
            stops[stops.len() - 1].1
        }
    }
}

pub fn state(ev: &Evaluator, c: &CompPlan, l: &LayerPlan, p: &Particles, t: f64, lt: f64) -> Result<ParticleState> {
    let env = LayerEnv::new(ev, c, l.index, t, 0);
    let stops: Vec<(f64, [f32; 4])> = p.colors.iter().map(|(u, col)| col.0.at(0.0, &env).map(|rgba| (*u, rgba))).collect::<Result<_>>().context("particle colors")?;
    let mut items = vec![];
    let seed = p.seed;
    let r = |id: i64, ch: i32| rand01(seed, (id & 0x7fff_ffff) as i32, ch);
    let lerp = |a: [f64; 2], k: f64| a[0] + (a[1] - a[0]) * k;
    let one = |id: i64, born: f64, items: &mut Vec<Particle>| {
        let life = lerp(p.life, r(id, 1)).max(1e-3);
        let age = lt - born;
        if age < 0.0 || age >= life || items.len() >= p.max as usize {
            return;
        }
        // where it starts
        let (ex, ey) = match p.emitter.shape.as_str() {
            "line" => ((r(id, 2) - 0.5) * p.emitter.size[0], 0.0),
            "rect" => ((r(id, 2) - 0.5) * p.emitter.size[0], (r(id, 3) - 0.5) * p.emitter.size[1]),
            "circle" => {
                let (a, d) = (r(id, 2) * std::f64::consts::TAU, r(id, 3).sqrt() * p.emitter.size[0]);
                (a.cos() * d, a.sin() * d)
            }
            "ring" => {
                let a = r(id, 2) * std::f64::consts::TAU;
                (a.cos() * p.emitter.size[0], a.sin() * p.emitter.size[0])
            }
            _ => (0.0, 0.0),
        };
        let dir = (p.angle + (r(id, 4) - 0.5) * p.spread).to_radians();
        let speed = lerp(p.speed, r(id, 5));
        let (v0x, v0y) = (dir.cos() * speed, dir.sin() * speed);
        let spin = lerp(p.spin, r(id, 6));
        let size0 = lerp(p.size, r(id, 7));
        let ghosts = p.trail.max(1);
        for g in 0..ghosts {
            let a = age - f64::from(g) * p.trail_step;
            if a < 0.0 {
                break;
            }
            let u = (a / life).clamp(0.0, 1.0);
            let (gx, gy) = (p.gravity[0], p.gravity[1]);
            let (mut dx, mut dy, vx, vy);
            if p.drag.abs() > 1e-6 {
                let e = (-p.drag * a).exp();
                let k = (1.0 - e) / p.drag;
                dx = gx / p.drag * a + (v0x - gx / p.drag) * k;
                dy = gy / p.drag * a + (v0y - gy / p.drag) * k;
                vx = gx / p.drag + (v0x - gx / p.drag) * e;
                vy = gy / p.drag + (v0y - gy / p.drag) * e;
            } else {
                dx = v0x * a + 0.5 * gx * a * a;
                dy = v0y * a + 0.5 * gy * a * a;
                vx = v0x + gx * a;
                vy = v0y + gy * a;
            }
            if let Some(tb) = &p.turbulence {
                let phase = (ex + ey) / tb.scale.max(1.0) + r(id, 8) * 50.0;
                dx += tb.amount * noise1(seed.wrapping_add(11), phase + a * tb.speed);
                dy += tb.amount * noise1(seed.wrapping_add(23), phase + 31.7 + a * tb.speed);
            }
            let mut color = gradient(&stops, u);
            color[3] *= curve(&p.opacity_over_life, u, 1.0).clamp(0.0, 1.0) as f32 * (1.0 - g as f32 / ghosts as f32);
            let size = size0 * curve(&p.size_over_life, u, 1.0).max(0.0);
            items.push(Particle { x: ex + dx, y: ey + dy, size, rotation: spin * a + r(id, 9) * 360.0, color, vx, vy });
        }
    };
    let max_life = p.life[0].max(p.life[1]).max(1e-3);
    if p.rate > 0.0 {
        let first = ((lt - max_life) * p.rate).floor() as i64 - 1;
        let first = first.max(-((p.prewarm * p.rate).ceil() as i64));
        let last = (lt * p.rate).floor() as i64;
        for k in first..=last {
            one(k, (k as f64 + r(k, 10)) / p.rate, &mut items);
        }
    }
    for (j, b) in p.burst.iter().enumerate() {
        if lt >= b.at && lt - b.at < max_life {
            for i in 0..i64::from(b.count) {
                one(1_000_000 * (j as i64 + 1) + i, b.at, &mut items);
            }
        }
    }
    Ok(ParticleState { count: items.len(), shape: p.shape.clone(), items })
}

#[cfg(test)]
mod tests {
    use crate::{eval::{Content, Evaluator}, plan::{Plan, ROOT}};
    use serde_json::json;

    fn count(plan: &Plan, t: f64) -> usize {
        match &Evaluator::new(plan).frame(ROOT, t).unwrap().layers[0].content {
            Content::Particles(p) => p.count,
            _ => panic!("not particles"),
        }
    }

    #[test]
    fn a_continuous_emitter_reaches_a_steady_population_and_frames_are_independent() {
        let plan = Plan::from_value(json!({"dur": 6, "layers": [{"type": "particles", "rate": 100, "life": [1, 1], "speed": [0, 0]}]}), std::path::Path::new(".")).unwrap();
        assert_eq!(count(&plan, 0.0), 0);
        assert!((95..=105).contains(&count(&plan, 3.0)), "{}", count(&plan, 3.0));
        // evaluating other times first changes nothing
        let a = count(&plan, 3.0);
        let _ = (count(&plan, 1.0), count(&plan, 5.0));
        assert_eq!(a, count(&plan, 3.0));
    }

    #[test]
    fn negative_drag_accelerates_outward_in_closed_form() {
        let at = |drag: f64| {
            let plan = Plan::from_value(json!({"dur": 6, "layers": [{"type": "particles", "rate": 0, "burst": [{"at": 0, "count": 1}], "life": [3, 3], "speed": [100, 100], "angle": 0, "spread": 0, "drag": drag}]}), std::path::Path::new(".")).unwrap();
            match &Evaluator::new(&plan).frame(ROOT, 1.0).unwrap().layers[0].content {
                Content::Particles(p) => p.items[0].x,
                _ => panic!("not particles"),
            }
        };
        let (still, grow) = (at(0.0), at(-1.0));
        assert!((still - 100.0).abs() < 1e-6, "{still}");
        assert!((grow - 100.0 * (1f64.exp() - 1.0)).abs() < 1e-3, "{grow}");
    }

    #[test]
    fn bursts_prewarm_and_the_cap() {
        let plan = Plan::from_value(json!({"dur": 6, "layers": [{"type": "particles", "rate": 0, "burst": [{"at": 1, "count": 40}], "life": [2, 2]}]}), std::path::Path::new(".")).unwrap();
        assert_eq!((count(&plan, 0.5), count(&plan, 1.5), count(&plan, 3.5)), (0, 40, 0));
        let warm = Plan::from_value(json!({"dur": 6, "layers": [{"type": "particles", "rate": 50, "life": [2, 2], "prewarm": 2}]}), std::path::Path::new(".")).unwrap();
        assert!(count(&warm, 0.0) > 80);
        let capped = Plan::from_value(json!({"dur": 6, "layers": [{"type": "particles", "rate": 1000, "life": [3, 3], "max": 500}]}), std::path::Path::new(".")).unwrap();
        assert_eq!(count(&capped, 4.0), 500);
    }
}
