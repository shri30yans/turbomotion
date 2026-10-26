//! Painting the mesh: transform and shade every vertex once, cull the faces turned away, sort the rest far to near
//! and hand Skia one coloured-triangle batch. Sorting is by triangle centre (painter's algorithm), with one
//! structural fix: a visible cap can never be behind a wall of the same solid, so caps are drawn after the sides.

use skia_safe::{vertices::VertexMode, BlendMode, Canvas, Color, FilterMode, MipmapMode, Paint, Point, Rect, SamplingOptions, Vertices};
use turbomotion_core::layer::picture_info;

/// Supersampling factor for the silhouette.
const SSAA: f32 = 3.0;

use super::{Mesh, Part};
use crate::{eval::{mat4::Mat4, LightState}, schema::Material};

/// How one layer sees its mesh.
pub struct View<'a> {
    /// Mesh space to world space.
    pub world: &'a Mat4,
    /// Mesh space to screen pixels (with the perspective divide in `w`).
    pub screen: &'a Mat4,
    /// Camera position; `None` for an orthographic look down +z.
    pub eye: Option<[f64; 3]>,
    pub lights: &'a [LightState],
    pub material: Material,
    /// Front, back and side colours, straight RGB 0..1.
    pub colors: [[f32; 3]; 3],
}

struct Placed {
    xy: [f32; 2],
    depth: f64,
    color: [f32; 3],
    /// Unit normal in world space.
    n: [f64; 3],
    world: [f64; 3],
}

/// Triangles that survive culling, in painting order, as `(vertex indices, depth)`.
fn visible(mesh: &Mesh, view: &View) -> (Vec<Placed>, Vec<[usize; 3]>) {
    let w = &view.world.0;
    let linear = |n: [f32; 3]| {
        let n = n.map(f64::from);
        let r = [w[0] * n[0] + w[1] * n[1] + w[2] * n[2], w[4] * n[0] + w[5] * n[1] + w[6] * n[2], w[8] * n[0] + w[9] * n[1] + w[10] * n[2]];
        let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt().max(1e-12);
        [r[0] / l, r[1] / l, r[2] / l]
    };
    let placed: Vec<Option<Placed>> = mesh
        .verts
        .iter()
        .map(|v| {
            let [x, y, z, sw] = view.screen.apply(f64::from(v.p[0]), f64::from(v.p[1]), f64::from(v.p[2]));
            if sw < 1e-6 {
                return None;
            }
            let pw = view.world.apply(f64::from(v.p[0]), f64::from(v.p[1]), f64::from(v.p[2]));
            let n = linear(v.n);
            let base = view.colors[match v.part { Part::Front => 0, Part::Back => 1, Part::Side => 2 }];
            let color = super::shade::shade([pw[0], pw[1], pw[2]], n, view.eye, base, &view.material, view.lights);
            let depth = if view.eye.is_some() { sw } else { z };
            Some(Placed { xy: [(x / sw) as f32, (y / sw) as f32], depth, color, n, world: [pw[0], pw[1], pw[2]] })
        })
        .collect();
    let mut keep: Vec<([usize; 3], bool, f64)> = vec![];
    for t in &mesh.tris {
        let ids = t.map(|i| i as usize);
        let (Some(a), Some(b), Some(c)) = (&placed[ids[0]], &placed[ids[1]], &placed[ids[2]]) else { continue };
        let centre = [0, 1, 2].map(|k| (a.world[k] + b.world[k] + c.world[k]) / 3.0);
        let n = [0, 1, 2].map(|k| a.n[k] + b.n[k] + c.n[k]);
        let to_eye = view.eye.map_or([0.0, 0.0, -1.0], |e| [e[0] - centre[0], e[1] - centre[1], e[2] - centre[2]]);
        if n[0] * to_eye[0] + n[1] * to_eye[1] + n[2] * to_eye[2] <= 0.0 {
            continue;
        }
        let cap = mesh.verts[ids[0]].part != Part::Side;
        keep.push((ids, cap, (a.depth + b.depth + c.depth) / 3.0));
    }
    keep.sort_by(|x, y| x.1.cmp(&y.1).then(y.2.total_cmp(&x.2)));
    (placed.into_iter().map(|p| p.unwrap_or(Placed { xy: [0.0; 2], depth: 0.0, color: [0.0; 3], n: [0.0; 3], world: [0.0; 3] })).collect(), keep.into_iter().map(|k| k.0).collect())
}

/// Paint `mesh` onto `c` (a canvas in screen pixels, no extra matrix).
pub fn paint(c: &Canvas, mesh: &Mesh, view: &View) {
    let (placed, order) = visible(mesh, view);
    if order.is_empty() {
        return;
    }
    let mut pos: Vec<Point> = Vec::with_capacity(order.len() * 3);
    let mut col: Vec<Color> = Vec::with_capacity(order.len() * 3);
    for tri in &order {
        for &i in tri {
            let p = &placed[i];
            pos.push(Point::new(p.xy[0], p.xy[1]));
            col.push(Color::from_rgb((p.color[0] * 255.0 + 0.5) as u8, (p.color[1] * 255.0 + 0.5) as u8, (p.color[2] * 255.0 + 0.5) as u8));
        }
    }
    let verts = Vertices::new_copy(VertexMode::Triangles, &pos, &vec![Point::new(0.0, 0.0); pos.len()], &col, None);
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    // draw_vertices has no edge antialiasing, so the batch is drawn SSAA times larger into its own small surface and
    // filtered down; only the letter's bounding box is allocated.
    let b = pos.iter().fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |b, p| [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)]);
    let dest = Rect::new((b[0] - 2.0).floor(), (b[1] - 2.0).floor(), (b[2] + 2.0).ceil(), (b[3] + 2.0).ceil());
    let k = SSAA.min(4096.0 / dest.width().max(dest.height())).max(1.0);
    let info = picture_info(c, (dest.width() * k).ceil() as i32, (dest.height() * k).ceil() as i32);
    let Some(mut big) = c.new_surface(&info, None) else {
        c.draw_vertices(&verts, BlendMode::Dst, &paint);
        return;
    };
    big.canvas().clear(skia_safe::Color::TRANSPARENT);
    big.canvas().scale((k, k));
    big.canvas().translate((-dest.left, -dest.top));
    big.canvas().draw_vertices(&verts, BlendMode::Dst, &paint);
    c.draw_image_rect_with_sampling_options(big.image_snapshot(), None, dest, SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear), &Paint::default());
}
