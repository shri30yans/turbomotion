//! A `mesh` layer at one instant: the cached triangle mesh its geometry makes, the colours of its faces and its
//! screen-space bounds. Every geometry (text, box, sphere, torus, cylinder, extruded path) ends here, so lighting,
//! sorting and painting are written once.

use std::rc::Rc;

use anyhow::{anyhow, Context, Result};
use serde::Serialize;

use super::{mat4::Mat4, CompPlan, Evaluator, LayerEnv};
use crate::{
    anim::prop::Rgba,
    mesh::{glyphs, primitives, Mesh, MeshKey},
    plan::LayerPlan,
    schema::{Align, Geometry, Material, MeshLayer},
    text::layout::Spec,
};

#[derive(Serialize)]
pub struct MeshState {
    /// The geometry's name: `text`, `box`, `sphere`, `torus`, `cylinder` or `extrude`.
    pub geometry: &'static str,
    pub color: Rgba,
    pub side: Rgba,
    pub back: Rgba,
    /// Mesh extent `[x0, y0, z0, x1, y1, z1]` around the layer's anchor.
    pub extent: [f32; 6],
    #[serde(skip)]
    pub material: Material,
    #[serde(skip)]
    pub mesh: Rc<Mesh>,
}

pub fn state(ev: &Evaluator, c: &CompPlan, l: &LayerPlan, m: &MeshLayer, t: f64) -> Result<MeshState> {
    let lt = t - l.start;
    let env = |salt: u32| LayerEnv::new(ev, c, l.index, t, 0).with_salt(salt);
    let color = m.color.as_ref().map_or(Ok([255.0, 255.0, 255.0, 1.0]), |p| p.0.at(lt, &env(63)))?;
    let side = match &m.side {
        Some(p) => p.0.at(lt, &env(64))?,
        None => [color[0] * 0.55, color[1] * 0.55, color[2] * 0.55, color[3]],
    };
    let back = m.back.as_ref().map_or(Ok(side), |p| p.0.at(lt, &env(65)))?;
    let (name, mesh) = mesh_for(ev, &m.geometry, m.segments.unwrap_or(primitives::DEFAULT_SEGMENTS), lt, &env)?;
    let extent = [mesh.min[0], mesh.min[1], mesh.min[2], mesh.max[0], mesh.max[1], mesh.max[2]];
    Ok(MeshState { geometry: name, color, side, back, extent, material: l.def.common.material.unwrap_or_default(), mesh })
}

/// The mesh for a geometry at this instant, built on first use. An animated size or depth rebuilds per distinct value.
fn mesh_for<'a>(ev: &Evaluator, g: &'a Geometry, segments: u32, lt: f64, env: &impl Fn(u32) -> LayerEnv<'a>) -> Result<(&'static str, Rc<Mesh>)> {
    let hund = |v: f64| (v * 100.0).round() as i64;
    let num = |p: &crate::schema::P, salt: u32| -> Result<f64> { p.0.at(lt, &env(salt)) };
    let opt = |p: &Option<crate::schema::P>, salt: u32, default: f64| -> Result<f64> { p.as_ref().map_or(Ok(default), |p| num(p, salt)) };
    let cached = |key: MeshKey, build: &dyn Fn() -> Result<Mesh>| -> Result<Rc<Mesh>> {
        if let Some(m) = ev.meshes.borrow().get(&key) {
            return Ok(m.clone());
        }
        let mesh = Rc::new(build()?);
        ev.meshes.borrow_mut().insert(key, mesh.clone());
        Ok(mesh)
    };
    let object = |shape: &str, path: Option<&String>, dims: [f64; 5]| MeshKey::Object { shape: shape.into(), path: path.cloned(), dims: dims.map(hund), segments };
    Ok(match g {
        Geometry::Text { text, font, weight, italic, size, depth, bevel } => {
            let size = num(size, 60)?.max(1.0);
            let depth = opt(depth, 61, 0.2 * size)?.max(0.0);
            let bevel = opt(bevel, 62, 0.0)?.max(0.0);
            let key = MeshKey::Text { text: text.clone(), font: font.clone(), weight: weight.unwrap_or(400), italic: *italic, size: hund(size), depth: hund(depth), bevel: hund(bevel) };
            let mesh = cached(key, &|| {
                let spec = Spec::new(text, font.as_deref(), *weight, *italic, size, 0.0, None, Align::Left, None);
                let layout = ev.text.layout(ev.plan, &spec).with_context(|| format!("mesh text {text:?}"))?;
                Ok(Mesh::build(&glyphs::outlines(&layout), depth as f32, bevel as f32))
            })?;
            ("text", mesh)
        }
        Geometry::Box { size, height, depth } => {
            let w = num(size, 66)?.max(1.0);
            let (h, d) = (opt(height, 67, w)?.max(1.0), opt(depth, 68, w)?.max(1.0));
            ("box", cached(object("box", None, [w, h, d, 0.0, 0.0]), &|| Ok(primitives::cuboid(w as f32, h as f32, d as f32)))?)
        }
        Geometry::Sphere { size } => {
            let d = num(size, 66)?.max(1.0);
            ("sphere", cached(object("sphere", None, [d, 0.0, 0.0, 0.0, 0.0]), &|| Ok(primitives::sphere(d as f32, segments)))?)
        }
        Geometry::Torus { size, tube } => {
            let d = num(size, 66)?.max(1.0);
            let tube = opt(tube, 67, d / 4.0)?.max(1.0);
            ("torus", cached(object("torus", None, [d, tube, 0.0, 0.0, 0.0]), &|| Ok(primitives::torus(d as f32, tube as f32, segments)))?)
        }
        Geometry::Cylinder { size, height, top } => {
            let d = num(size, 66)?.max(1.0);
            let h = num(height, 67)?.max(1.0);
            let top = opt(top, 68, d)?.max(0.0);
            ("cylinder", cached(object("cylinder", None, [d, h, top, 0.0, 0.0]), &|| Ok(primitives::frustum(d as f32, top as f32, h as f32, segments)))?)
        }
        Geometry::Extrude { d, size, depth, bevel } => {
            let width = num(size, 66)?.max(1.0);
            let depth = opt(depth, 67, 0.3 * width)?.max(0.0);
            let bevel = opt(bevel, 68, 0.0)?.max(0.0);
            let mesh = cached(object("extrude", Some(d), [width, depth, bevel, 0.0, 0.0]), &|| {
                let path = skia_safe::Path::from_svg(d).ok_or_else(|| anyhow!("could not read the SVG path {d:?}"))?;
                primitives::extruded(&path, width as f32, depth as f32, bevel as f32).ok_or_else(|| anyhow!("the SVG path {d:?} encloses no area to extrude"))
            })?;
            ("extrude", mesh)
        }
    })
}

/// Screen bounding box of the mesh's box corners; `None` (and not visible) when a corner is behind the camera.
pub fn bounds(screen: &Mat4, t: &MeshState, visible: &mut bool) -> Option<[f64; 4]> {
    let e = t.extent.map(f64::from);
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for k in 0..8 {
        let Some(p) = screen.project(e[(k & 1) * 3], e[1 + ((k >> 1) & 1) * 3], e[2 + ((k >> 2) & 1) * 3]) else {
            *visible = false;
            return None;
        };
        b = [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])];
    }
    Some(b)
}
