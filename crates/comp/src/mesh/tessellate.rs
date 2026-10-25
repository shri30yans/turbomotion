//! Extrusion: fill the glyph's front and back with triangles (counters stay open), then stand walls on every contour
//! edge. A bevel pulls each cap inward by a miter offset and joins it to the wall with a 45 degree ring.

use lyon_tessellation::{geom::point, path::Path, BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, VertexBuffers};

use super::{glyphs::Contour, Mesh, Part, Vert};

/// Edge normals closer than this (cosine) share a smooth vertex normal; sharper corners stay crisp.
const SMOOTH_COS: f32 = 0.8;

type V2 = [f32; 2];

fn area(c: &Contour) -> f32 {
    (0..c.len()).map(|i| c[i][0] * c[(i + 1) % c.len()][1] - c[(i + 1) % c.len()][0] * c[i][1]).sum::<f32>() / 2.0
}

/// Unit outward normal (out of the solid) of each edge `i -> i+1`; `flip` is +1 or -1 by which side the glyph's fill is on.
fn edge_normals(c: &Contour, flip: f32) -> Vec<V2> {
    (0..c.len())
        .map(|i| {
            let (a, b) = (c[i], c[(i + 1) % c.len()]);
            unit([flip * (b[1] - a[1]), -flip * (b[0] - a[0])])
        })
        .collect()
}

fn unit(v: V2) -> V2 {
    let l = (v[0] * v[0] + v[1] * v[1]).sqrt().max(1e-9);
    [v[0] / l, v[1] / l]
}

/// Normals at both ends of each edge: averaged with the neighbour when the corner is gentle, the edge's own otherwise.
fn end_normals(en: &[V2]) -> Vec<(V2, V2)> {
    let n = en.len();
    let at = |prev: V2, cur: V2| if prev[0] * cur[0] + prev[1] * cur[1] > SMOOTH_COS { unit([prev[0] + cur[0], prev[1] + cur[1]]) } else { cur };
    (0..n).map(|i| (at(en[(i + n - 1) % n], en[i]), at(en[(i + 1) % n], en[i]))).collect()
}

/// Move the contour inward by `b` (miter join, capped so spikes do not shoot out).
fn inset(c: &Contour, en: &[V2], b: f32) -> Contour {
    let n = c.len();
    (0..n)
        .map(|i| {
            let (p, q) = (en[(i + n - 1) % n], en[i]);
            let k = b / (1.0 + p[0] * q[0] + p[1] * q[1]).max(0.25);
            [c[i][0] - (p[0] + q[0]) * k, c[i][1] - (p[1] + q[1]) * k]
        })
        .collect()
}

struct Builder<'a> {
    m: &'a mut Mesh,
}

impl Builder<'_> {
    fn vert(&mut self, p: V2, z: f32, n: [f32; 3], part: Part) -> u32 {
        self.m.verts.push(Vert { p: [p[0], p[1], z], n, part });
        (self.m.verts.len() - 1) as u32
    }

    /// Add a triangle wound so its geometric normal agrees with `want` (degenerate ones are dropped).
    fn tri(&mut self, a: u32, b: u32, c: u32, want: [f32; 3]) {
        let v = |i: u32| self.m.verts[i as usize].p;
        let (pa, pb, pc) = (v(a), v(b), v(c));
        let (u, w) = ([pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]], [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]]);
        let g = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
        if g[0] * g[0] + g[1] * g[1] + g[2] * g[2] < 1e-12 {
            return;
        }
        if g[0] * want[0] + g[1] * want[1] + g[2] * want[2] >= 0.0 { self.m.tris.push([a, b, c]) } else { self.m.tris.push([a, c, b]) }
    }

    /// Fill `contours` (non-zero rule, like the font) as a flat face at depth `z` facing `nz`.
    fn cap(&mut self, contours: &[Contour], z: f32, nz: f32, part: Part) {
        let mut pb = Path::builder();
        for c in contours {
            pb.begin(point(c[0][0], c[0][1]));
            c[1..].iter().for_each(|p| {
                pb.line_to(point(p[0], p[1]));
            });
            pb.end(true);
        }
        let mut buf: VertexBuffers<V2, u32> = VertexBuffers::new();
        let opts = FillOptions::default().with_fill_rule(FillRule::NonZero).with_tolerance(0.05);
        let mut sink = BuffersBuilder::new(&mut buf, |v: FillVertex| v.position().to_array());
        if FillTessellator::new().tessellate_path(&pb.build(), &opts, &mut sink).is_err() {
            return;
        }
        let base: Vec<u32> = buf.vertices.iter().map(|p| self.vert(*p, z, [0.0, 0.0, nz], part)).collect();
        for t in buf.indices.chunks_exact(3) {
            self.tri(base[t[0] as usize], base[t[1] as usize], base[t[2] as usize], [0.0, 0.0, nz]);
        }
    }

    /// A strip of quads along a contour from outline `a` at depth `za` to outline `b` at `zb`. `tilt` is the normals' z
    /// component (0 for a wall, +-sin 45 degrees for a bevel ring) and the horizontal part is scaled to keep them unit.
    fn band(&mut self, a: &Contour, za: f32, b: &Contour, zb: f32, ends: &[(V2, V2)], tilt: f32) {
        let flat = (1.0 - tilt * tilt).sqrt();
        let n = a.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let (ns, ne) = ends[i];
            let (n0, n1) = ([ns[0] * flat, ns[1] * flat, tilt], [ne[0] * flat, ne[1] * flat, tilt]);
            let (v0, v1) = (self.vert(a[i], za, n0, Part::Side), self.vert(a[j], za, n1, Part::Side));
            let (v2, v3) = (self.vert(b[j], zb, n1, Part::Side), self.vert(b[i], zb, n0, Part::Side));
            let want = [n0[0] + n1[0], n0[1] + n1[1], tilt * 2.0];
            self.tri(v0, v1, v2, want);
            self.tri(v0, v2, v3, want);
        }
    }
}

/// Extrude one glyph's contours between `z = -depth/2` (front) and `+depth/2` (back) into `m`.
pub fn extrude(contours: &[Contour], depth: f32, bevel: f32, m: &mut Mesh) {
    if contours.is_empty() {
        return;
    }
    // The biggest contour is an outer edge; its winding says which side of every edge is solid.
    let main = contours.iter().map(area).max_by(|a, b| a.abs().total_cmp(&b.abs())).unwrap_or(1.0);
    let flip = if main > 0.0 { 1.0 } else { -1.0 };
    let (zf, zb) = (-depth / 2.0, depth / 2.0);
    let b = bevel.min(depth / 2.0 - 0.01).max(0.0);
    let normals: Vec<Vec<V2>> = contours.iter().map(|c| edge_normals(c, flip)).collect();
    let caps: Vec<Contour> = if b > 0.0 { contours.iter().zip(&normals).map(|(c, en)| inset(c, en, b)).collect() } else { contours.to_vec() };
    let mut bd = Builder { m };
    bd.cap(&caps, zf, -1.0, Part::Front);
    bd.cap(&caps, zb, 1.0, Part::Back);
    let k = std::f32::consts::FRAC_1_SQRT_2;
    for ((c, en), cap) in contours.iter().zip(&normals).zip(&caps) {
        let ends = end_normals(en);
        if zb - zf - 2.0 * b > 1e-3 {
            bd.band(c, zf + b, c, zb - b, &ends, 0.0);
        }
        if b > 0.0 {
            bd.band(c, zf + b, cap, zf, &ends, -k);
            bd.band(c, zb - b, cap, zb, &ends, k);
        }
    }
}
