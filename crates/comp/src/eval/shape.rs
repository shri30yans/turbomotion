//! Shape layers at one instant: colours, stroke widths, trim ranges and per-item transforms as numbers. The renderer
//! turns them into Skia paints; `inspect` prints them.

use anyhow::Result;
use serde::Serialize;
use skia_safe::{ContourMeasureIter, Matrix, Path, Rect};

use super::{CompPlan, Evaluator, LayerEnv};
use crate::{
    anim::prop::Rgba,
    plan::{geometry::PathGeo, LayerPlan},
    schema::ShapeItem,
};

#[derive(Serialize, Clone)]
pub struct GradientState {
    pub radial: bool,
    pub stops: Vec<(f64, Rgba)>,
    pub p0: [f64; 2],
    pub p1: [f64; 2],
    pub radius: f64,
}

#[derive(Serialize, Clone)]
pub struct FillState {
    pub color: Option<Rgba>,
    pub gradient: Option<GradientState>,
    pub opacity: f64,
    pub even_odd: bool,
}

#[derive(Serialize, Clone)]
pub struct StrokeState {
    pub color: Rgba,
    pub width: f64,
    pub cap: String,
    pub join: String,
    pub dash: Option<Vec<f64>>,
    pub dash_offset: f64,
    pub opacity: f64,
}

#[derive(Serialize, Clone)]
pub struct RepeatState {
    pub copies: u32,
    pub offset: [f64; 2],
    pub rotation: f64,
    pub scale: f64,
    pub opacity: f64,
}

#[derive(Serialize, Clone)]
pub struct ShapeState {
    pub fill: Option<FillState>,
    pub stroke: Option<StrokeState>,
    /// `[start, end, offset]`, all 0..1.
    pub trim: Option<[f64; 3]>,
    pub morph: f64,
    pub offset: [f64; 2],
    pub rotation: f64,
    pub scale: [f64; 2],
    pub opacity: f64,
    pub repeat: Option<RepeatState>,
}

pub fn state(ev: &Evaluator, c: &CompPlan, l: &LayerPlan, it: &ShapeItem, geo: &PathGeo, k: usize, t: f64) -> Result<ShapeState> {
    let lt = t - l.start;
    let base = 100 + 20 * k as u32;
    let env = |salt: u32| LayerEnv::new(ev, c, l.index, t, 0).with_salt(base + salt);
    let fill = it
        .fill
        .as_ref()
        .map(|f| -> Result<FillState> {
            let gradient = f
                .gradient
                .as_ref()
                .map(|g| -> Result<GradientState> {
                    let pt = |p: &Option<crate::schema::Pos>, d: [f64; 2], salt: u32| -> Result<[f64; 2]> {
                        match p {
                            None => Ok(d),
                            Some(p) => {
                                let v = p.0.at(lt, &env(salt))?;
                                Ok([v[0], v.get(1).copied().unwrap_or(0.0)])
                            }
                        }
                    };
                    let mut stops = vec![];
                    for (s, col) in &g.stops {
                        stops.push((*s, col.0.at(lt, &env(2))?));
                    }
                    Ok(GradientState {
                        radial: g.kind == "radial",
                        stops,
                        p0: pt(if g.kind == "radial" { &g.center } else { &g.from }, [0.0, 0.0], 3)?,
                        p1: pt(&g.to, [0.0, 100.0], 5)?,
                        radius: g.radius.as_ref().map_or(Ok(100.0), |r| r.0.at(lt, &env(7)))?,
                    })
                })
                .transpose()?;
            Ok(FillState {
                color: f.color.as_ref().map(|c| c.0.at(lt, &env(1))).transpose()?,
                gradient,
                opacity: f.opacity.as_ref().map_or(Ok(1.0), |o| o.0.at(lt, &env(8)))?.clamp(0.0, 1.0),
                even_odd: f.even_odd,
            })
        })
        .transpose()?;
    let stroke = it
        .stroke
        .as_ref()
        .map(|s| -> Result<StrokeState> {
            Ok(StrokeState {
                color: s.color.0.at(lt, &env(10))?,
                width: s.width.0.at(lt, &env(11))?.max(0.0),
                cap: s.cap.clone().unwrap_or_else(|| "butt".into()),
                join: s.join.clone().unwrap_or_else(|| "miter".into()),
                dash: s.dash.clone(),
                dash_offset: s.dash_offset.as_ref().map_or(Ok(0.0), |d| d.0.at(lt, &env(12)))?,
                opacity: s.opacity.as_ref().map_or(Ok(1.0), |o| o.0.at(lt, &env(13)))?.clamp(0.0, 1.0),
            })
        })
        .transpose()?;
    let trim = it
        .trim
        .as_ref()
        .map(|tr| -> Result<[f64; 3]> {
            Ok([
                tr.start.as_ref().map_or(Ok(0.0), |p| p.0.at(lt, &env(14)))?.clamp(0.0, 1.0),
                tr.end.as_ref().map_or(Ok(1.0), |p| p.0.at(lt, &env(15)))?.clamp(0.0, 1.0),
                tr.offset.as_ref().map_or(Ok(0.0), |p| p.0.at(lt, &env(16)))?,
            ])
        })
        .transpose()?;
    let morph = it.path.morph.as_ref().map_or(Ok(0.0), |m| m.0.at(lt, &env(17)))?;
    let _ = geo;
    let offset = match &it.offset {
        None => [0.0, 0.0],
        Some(p) => {
            let v = p.0.at(lt, &env(18))?;
            [v[0], v.get(1).copied().unwrap_or(0.0)]
        }
    };
    let scale = match &it.scale {
        None => [1.0, 1.0],
        Some(s) => {
            let v = s.0.at(lt, &env(20))?;
            [v[0], v.get(1).copied().unwrap_or(v[0])]
        }
    };
    Ok(ShapeState {
        fill,
        stroke,
        trim,
        morph,
        offset,
        rotation: it.rotation.as_ref().map_or(Ok(0.0), |r| r.0.at(lt, &env(22)))?,
        scale,
        opacity: it.opacity.as_ref().map_or(Ok(1.0), |o| o.0.at(lt, &env(23)))?.clamp(0.0, 1.0),
        repeat: it.repeat.as_ref().map(|r| RepeatState { copies: r.copies.max(1), offset: r.offset.unwrap_or([0.0, 0.0]), rotation: r.rotation, scale: r.scale, opacity: r.opacity }),
    })
}

/// The matrix of an item inside its layer: offset · rotate · scale.
pub fn item_matrix(s: &ShapeState) -> Matrix {
    let mut m = Matrix::translate((s.offset[0] as f32, s.offset[1] as f32));
    m.pre_rotate(s.rotation as f32, None);
    m.pre_scale((s.scale[0] as f32, s.scale[1] as f32), None);
    m
}

/// Union of the items' outlines in layer space, widened by half the stroke.
pub fn bounds(items: &[ShapeState], geos: &[PathGeo]) -> Option<[f64; 4]> {
    let mut acc: Option<Rect> = None;
    for (s, g) in items.iter().zip(geos) {
        let path = g.at(s.morph);
        let pad = s.stroke.as_ref().map_or(0.0, |st| st.width / 2.0) as f32;
        let copies = s.repeat.as_ref().map_or(1, |r| r.copies);
        let mut m = item_matrix(s);
        for _ in 0..copies {
            let mut b = m.map_rect(path.bounds()).0;
            b.outset((pad, pad));
            acc = Some(acc.map_or(b, |a| Rect::from_ltrb(a.left.min(b.left), a.top.min(b.top), a.right.max(b.right), a.bottom.max(b.bottom))));
            if let Some(r) = &s.repeat {
                let mut step = Matrix::translate((r.offset[0] as f32, r.offset[1] as f32));
                step.pre_rotate(r.rotation as f32, None);
                step.pre_scale((r.scale as f32, r.scale as f32), None);
                m = Matrix::concat(&m, &step);
            }
        }
    }
    acc.map(|r| [f64::from(r.left), f64::from(r.top), f64::from(r.right), f64::from(r.bottom)])
}

/// The point at fraction `u` along the first contour of `path`, and the direction of travel in degrees.
pub fn point_on(path: &Path, u: f64) -> ([f64; 2], f64) {
    let Some(contour) = ContourMeasureIter::new(path, false, None).next() else { return ([0.0, 0.0], 0.0) };
    let d = (u.clamp(0.0, 1.0) as f32) * contour.length();
    match contour.pos_tan(d) {
        Some((p, tan)) => ([f64::from(p.x), f64::from(p.y)], f64::from(tan.y).atan2(f64::from(tan.x)).to_degrees()),
        None => ([0.0, 0.0], 0.0),
    }
}

#[derive(Serialize, Clone)]
pub struct MaskState {
    pub feather: f64,
    pub expansion: f64,
    pub opacity: f64,
    pub morph: f64,
}

/// The animated values of a layer's masks at `t`.
pub fn masks(ev: &Evaluator, c: &CompPlan, l: &LayerPlan, t: f64) -> Result<Vec<MaskState>> {
    let lt = t - l.start;
    l.def
        .common
        .masks
        .iter()
        .enumerate()
        .map(|(k, m)| {
            let env = |salt: u32| LayerEnv::new(ev, c, l.index, t, 0).with_salt(400 + 8 * k as u32 + salt);
            Ok(MaskState {
                feather: m.feather.as_ref().map_or(Ok(0.0), |p| p.0.at(lt, &env(0)))?.max(0.0),
                expansion: m.expansion.as_ref().map_or(Ok(0.0), |p| p.0.at(lt, &env(1)))?,
                opacity: m.opacity.as_ref().map_or(Ok(1.0), |p| p.0.at(lt, &env(2)))?.clamp(0.0, 1.0),
                morph: m.path.morph.as_ref().map_or(Ok(0.0), |p| p.0.at(lt, &env(3)))?,
            })
        })
        .collect()
}
