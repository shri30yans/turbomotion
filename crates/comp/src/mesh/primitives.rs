//! Solid primitives: box, sphere, cylinder, cone, torus, and any outline extruded into depth. Each is a closed mesh
//! centred on the origin (x right, y down, z away from the viewer) with outward normals, so the same shading, culling
//! and painting that draw `text3d` letters draw these.

use std::f32::consts::{PI, TAU};

use skia_safe::Path;

use super::{glyphs, tessellate, Mesh, Part, Vert};

type V3 = [f32; 3];

/// Smoothness of curved surfaces: segments around a full turn.
pub const DEFAULT_SEGMENTS: u32 = 48;

/// Adds triangles oriented to a wanted outward direction, so winding never depends on how a grid was walked.
struct Build {
    m: Mesh,
}

impl Build {
    fn vert(&mut self, p: V3, n: V3, part: Part) -> u32 {
        self.m.verts.push(Vert { p, n, part });
        (self.m.verts.len() - 1) as u32
    }

    fn tri(&mut self, a: u32, b: u32, c: u32) {
        let v = |i: u32| self.m.verts[i as usize];
        let (va, vb, vc) = (v(a), v(b), v(c));
        let (u, w) = ([vb.p[0] - va.p[0], vb.p[1] - va.p[1], vb.p[2] - va.p[2]], [vc.p[0] - va.p[0], vc.p[1] - va.p[1], vc.p[2] - va.p[2]]);
        let g = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
        if g.iter().map(|x| x * x).sum::<f32>() < 1e-14 {
            return;
        }
        let want: V3 = [0, 1, 2].map(|k| va.n[k] + vb.n[k] + vc.n[k]);
        if g[0] * want[0] + g[1] * want[1] + g[2] * want[2] >= 0.0 { self.m.tris.push([a, b, c]) } else { self.m.tris.push([a, c, b]) }
    }

    fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.tri(a, b, c);
        self.tri(a, c, d);
    }

    /// A grid of `(rows + 1) x (cols + 1)` vertices from `f(row_fraction, col_fraction) -> (point, normal)`, with the
    /// last column optionally wrapping onto the first so the seam shares normals.
    fn grid(&mut self, rows: u32, cols: u32, part: Part, f: impl Fn(f32, f32) -> (V3, V3)) {
        let base = self.m.verts.len() as u32;
        for r in 0..=rows {
            for c in 0..=cols {
                let (p, n) = f(r as f32 / rows as f32, c as f32 / cols as f32);
                self.vert(p, n, part);
            }
        }
        let at = |r: u32, c: u32| base + r * (cols + 1) + c;
        for r in 0..rows {
            for c in 0..cols {
                self.quad(at(r, c), at(r, c + 1), at(r + 1, c + 1), at(r + 1, c));
            }
        }
    }

    fn finish(mut self) -> Mesh {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for v in &self.m.verts {
            for k in 0..3 {
                lo[k] = lo[k].min(v.p[k]);
                hi[k] = hi[k].max(v.p[k]);
            }
        }
        (self.m.min, self.m.max) = if self.m.verts.is_empty() { ([0.0; 3], [0.0; 3]) } else { (lo, hi) };
        std::mem::take(&mut self.m)
    }
}

/// A box `w x h x d`: front and back faces take the front and back colours, the four walls the side colour.
pub fn cuboid(w: f32, h: f32, d: f32) -> Mesh {
    let mut b = Build { m: Mesh::default() };
    let (x, y, z) = (w / 2.0, h / 2.0, d / 2.0);
    // (normal, u axis, v axis, part): the face is centre + n*half + u*s*half_u + v*t*half_v
    let faces: [(V3, V3, V3, Part); 6] = [
        ([0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], Part::Front),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], Part::Back),
        ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], Part::Side),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], Part::Side),
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], Part::Side),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], Part::Side),
    ];
    let half = [x, y, z];
    for (n, u, v, part) in faces {
        let corner = |s: f32, t: f32| -> V3 { [0, 1, 2].map(|k| n[k] * half[k] + u[k] * s * half[k] + v[k] * t * half[k]) };
        let ids = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(s, t)| b.vert(corner(s, t), n, part));
        b.quad(ids[0], ids[1], ids[2], ids[3]);
    }
    b.finish()
}

/// A sphere of diameter `d`, poles on the y axis.
pub fn sphere(d: f32, segments: u32) -> Mesh {
    let r = d / 2.0;
    let mut b = Build { m: Mesh::default() };
    let rings = (segments / 2).max(4);
    b.grid(rings, segments.max(8), Part::Front, |v, u| {
        let (theta, phi) = (v * PI, u * TAU);
        // the poles are exact points, so the triangles that fan out of them are not slivers
        let ring = if v == 0.0 || v == 1.0 { 0.0 } else { theta.sin() };
        let n = [ring * phi.cos(), -theta.cos(), ring * phi.sin()];
        (n.map(|c| c * r), n)
    });
    b.finish()
}

/// A torus lying in the xy plane (a ring facing the viewer): `d` is the ring's outer diameter, `tube` the thickness of its band.
pub fn torus(d: f32, tube: f32, segments: u32) -> Mesh {
    let tube_r = (tube / 2.0).clamp(0.5, d / 4.0);
    let big_r = d / 2.0 - tube_r;
    let mut b = Build { m: Mesh::default() };
    b.grid((segments / 2).max(8), segments.max(12), Part::Front, |v, u| {
        let (a, t) = (u * TAU, v * TAU);
        let n = [t.cos() * a.cos(), t.cos() * a.sin(), t.sin()];
        ([(big_r + tube_r * t.cos()) * a.cos(), (big_r + tube_r * t.cos()) * a.sin(), tube_r * t.sin()], n)
    });
    b.finish()
}

/// A cylinder (or a cone when `top` is 0) of diameter `d`, height `h`, standing on the y axis; `top` is the top diameter.
pub fn frustum(d: f32, top: f32, h: f32, segments: u32) -> Mesh {
    let (rb, rt) = (d / 2.0, top / 2.0);
    let mut b = Build { m: Mesh::default() };
    let slope = (rb - rt) / h.max(1e-3);
    let seg = segments.max(8);
    b.grid(1, seg, Part::Side, |v, u| {
        let a = u * TAU;
        let r = rb + (rt - rb) * v;
        let n = {
            let l = (1.0 + slope * slope).sqrt();
            [a.cos() / l, -slope / l, a.sin() / l]
        };
        ([r * a.cos(), h / 2.0 - h * v, r * a.sin()], n)
    });
    for (y, r, ny) in [(h / 2.0, rb, 1.0f32), (-h / 2.0, rt, -1.0)] {
        if r < 1e-3 {
            continue;
        }
        let centre = b.vert([0.0, y, 0.0], [0.0, ny, 0.0], Part::Front);
        let ring: Vec<u32> = (0..=seg).map(|i| b.vert([r * (i as f32 / seg as f32 * TAU).cos(), y, r * (i as f32 / seg as f32 * TAU).sin()], [0.0, ny, 0.0], Part::Front)).collect();
        for w in ring.windows(2) {
            b.tri(centre, w[0], w[1]);
        }
    }
    b.finish()
}

/// An SVG path (filled, non-zero rule) scaled to `width` px across and pushed `depth` deep, centred on its middle.
pub fn extruded(path: &Path, width: f32, depth: f32, bevel: f32) -> Option<Mesh> {
    let contours = glyphs::flatten(path, 0.0, 0.0);
    if contours.is_empty() {
        return None;
    }
    let pts = || contours.iter().flatten();
    let (lo, hi) = pts().fold(([f32::MAX; 2], [f32::MIN; 2]), |(lo, hi), p| ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])]));
    let k = width / (hi[0] - lo[0]).max(1e-3);
    let centre = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
    let scaled: Vec<glyphs::Contour> = contours.iter().map(|c| c.iter().map(|p| [(p[0] - centre[0]) * k, (p[1] - centre[1]) * k]).collect()).collect();
    let mut m = Mesh::default();
    tessellate::extrude(&scaled, depth, bevel, &mut m);
    let mut b = Build { m };
    Some(std::mem::take(&mut b.m)).map(|m| Build { m }.finish())
}
