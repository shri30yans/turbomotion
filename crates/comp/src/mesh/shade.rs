//! Lighting one vertex: the same model as the flat-layer lighting shader (ambient, Lambert, Blinn-Phong with point,
//! spot and directional lights, `1 / (1 + (d/r)^2)` falloff) so a text3d letter and a lit plane agree. Vertex colours
//! are interpolated by the GPU, so highlights live on the tessellation; a soft studio-sky reflection is added to
//! make bevels and curved walls read as glossy even where the lights are dull.

use crate::{eval::LightState, schema::Material};

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt().max(1e-9);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// Colour of a surface point `p` with unit normal `n` (world space) for a viewer at `eye` (`None`: looking along +z).
pub fn shade(p: [f64; 3], n: [f64; 3], eye: Option<[f64; 3]>, base: [f32; 3], mat: &Material, lights: &[LightState]) -> [f32; 3] {
    let v = eye.map_or([0.0, 0.0, -1.0], |e| unit([e[0] - p[0], e[1] - p[1], e[2] - p[2]]));
    let (mut ambient, mut diffuse, mut spec) = ([0.0f64; 3], [0.0f64; 3], [0.0f64; 3]);
    if lights.is_empty() {
        // no lights: a gentle headlight so the form still reads
        let k = 0.45 + 0.55 * dot(n, v).max(0.0);
        ambient = [k; 3];
    }
    for l in lights {
        if l.kind == "ambient" {
            (0..3).for_each(|i| ambient[i] += l.color[i]);
            continue;
        }
        let (dir, atten) = if l.kind == "directional" {
            (unit([-l.dir[0], -l.dir[1], -l.dir[2]]), 1.0)
        } else {
            let d = [l.pos[0] - p[0], l.pos[1] - p[1], l.pos[2] - p[2]];
            let dist = dot(d, d).sqrt();
            let r = l.radius.max(1.0);
            let mut a = 1.0 / (1.0 + dist * dist / (r * r));
            let dir = unit(d);
            if l.kind == "spot" {
                let c = -dot(dir, unit(l.dir));
                let lo = l.cone.to_radians().cos();
                a *= smooth(lo, lo + (1.0 - lo) * 0.25, c);
            }
            (dir, a)
        };
        let nl = dot(n, dir).max(0.0);
        for (d, c) in diffuse.iter_mut().zip(l.color) {
            *d += c * nl * atten;
        }
        if nl > 0.0 {
            let h = unit([dir[0] + v[0], dir[1] + v[1], dir[2] + v[2]]);
            let s = dot(n, h).max(0.0).powf(mat.shininess.max(1.0)) * if l.shine > 0.0 { l.shine } else { 1.0 } * atten;
            (0..3).for_each(|i| spec[i] += l.color[i] * s);
        }
    }
    // reflection of a bright sky above the scene (y is down, so "up" is -y)
    let r = [2.0 * dot(n, v) * n[0] - v[0], 2.0 * dot(n, v) * n[1] - v[1], 2.0 * dot(n, v) * n[2] - v[2]];
    let sheen = smooth(-0.2, 0.9, -r[1]).powi(3) * (1.0 - dot(n, v).clamp(0.0, 1.0)).powf(1.5).max(0.15) * 0.6 * mat.specular;
    let mut out = [0.0f32; 3];
    for i in 0..3 {
        let c = f64::from(base[i]) * (ambient[i] * mat.ambient + diffuse[i] * mat.diffuse) + spec[i] * mat.specular + sheen;
        out[i] = c.clamp(0.0, 1.0) as f32;
    }
    out
}

fn smooth(lo: f64, hi: f64, x: f64) -> f64 {
    let t = ((x - lo) / (hi - lo).max(1e-9)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
