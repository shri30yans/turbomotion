//! Real 3D letters: glyph outlines extruded into a triangle mesh, shaded by the composition's lights and projected by
//! its camera. The mesh is built once per (text, face, size, depth, bevel) and cached by the evaluator, so a frame
//! only transforms and shades vertices.

pub mod draw;
pub mod glyphs;
pub mod primitives;
pub mod shade;
pub mod tessellate;

/// Which surface a vertex belongs to, which picks its colour (`color`, `back`, or `side` for walls and bevel).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Front,
    Back,
    Side,
}

/// A mesh vertex. Normals are per vertex (smoothed along curves, split at sharp corners).
#[derive(Clone, Copy, Debug)]
pub struct Vert {
    pub p: [f32; 3],
    pub n: [f32; 3],
    pub part: Part,
}

/// A closed triangle mesh centred on the origin: x right, y down, z away from the viewer (the front face is at `-depth/2`).
/// Every triangle's winding makes its geometric normal point out of the solid.
#[derive(Default, Debug)]
pub struct Mesh {
    pub verts: Vec<Vert>,
    pub tris: Vec<[u32; 3]>,
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// What identifies a mesh, in hundredths of a pixel so it can be hashed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MeshKey {
    Text { text: String, font: Option<String>, weight: u16, italic: bool, size: i64, depth: i64, bevel: i64 },
    /// An `object3d`: its shape name and SVG path (for `extrude`), then every number that shapes it.
    Object { shape: String, path: Option<String>, dims: [i64; 5], segments: u32 },
}

impl Mesh {
    /// Extrude every glyph outline and centre the result on the middle of the letters at mid-depth.
    pub fn build(glyphs: &[Vec<glyphs::Contour>], depth: f32, bevel: f32) -> Mesh {
        let mut m = Mesh::default();
        for g in glyphs {
            tessellate::extrude(g, depth, bevel, &mut m);
        }
        m.recentre();
        m
    }

    fn recentre(&mut self) {
        let (lo, hi) = self.extent();
        let (cx, cy) = ((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0);
        self.verts.iter_mut().for_each(|v| {
            v.p[0] -= cx;
            v.p[1] -= cy;
        });
        (self.min, self.max) = self.extent();
    }

    fn extent(&self) -> ([f32; 3], [f32; 3]) {
        if self.verts.is_empty() {
            return ([0.0; 3], [0.0; 3]);
        }
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for v in &self.verts {
            for k in 0..3 {
                lo[k] = lo[k].min(v.p[k]);
                hi[k] = hi[k].max(v.p[k]);
            }
        }
        (lo, hi)
    }
}

#[cfg(test)]
mod tests;
