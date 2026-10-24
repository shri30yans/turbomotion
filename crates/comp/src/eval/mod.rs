//! Evaluation: `Plan × time → Frame`. Everything a renderer needs to draw one instant, as plain numbers: where each
//! layer is, how opaque it is, which part of a stroke is drawn. No GPU is involved, so `inspect` and `check` use
//! this directly, and a test can assert that a title is inside the safe area at 1.2 s without drawing a pixel.

pub mod env;
pub mod mat4;
pub mod media;
pub mod shape;
pub mod text;
pub mod transit;

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use turbomotion_core::effects::Blend;

use crate::{
    anim::{ease::Ease, prop::Rgba},
    plan::{CompPlan, LayerPlan, Plan},
    schema::{Fit, Kind, Transit},
};
pub use env::LayerEnv;
use mat4::Mat4;
pub use shape::{MaskState, ShapeState};

pub struct Evaluator<'p> {
    pub plan: &'p Plan,
    pub text: crate::text::layout::Engine,
    signals: std::cell::RefCell<std::collections::HashMap<String, std::rc::Rc<crate::audio::signal::Signal>>>,
}

#[derive(Serialize)]
pub struct Frame {
    pub comp: String,
    pub t: f64,
    pub width: u32,
    pub height: u32,
    pub bg: Option<Rgba>,
    pub camera: Option<CameraState>,
    pub lights: Vec<LightState>,
    pub layers: Vec<LayerState>,
}

#[derive(Serialize, Clone)]
pub struct CameraState {
    pub pos: [f64; 3],
    pub poi: [f64; 3],
    pub zoom: f64,
    pub roll: f64,
    pub dof: Option<[f64; 2]>,
    #[serde(skip)]
    pub view_proj: Mat4,
    #[serde(skip)]
    pub view: Mat4,
}

#[derive(Serialize, Clone)]
pub struct LightState {
    pub kind: String,
    pub pos: [f64; 3],
    pub dir: [f64; 3],
    pub color: [f64; 3],
    pub radius: f64,
    pub cone: f64,
    pub shine: f64,
}

#[derive(Serialize)]
pub struct LayerState {
    pub index: usize,
    pub id: Option<String>,
    pub label: String,
    /// Inside its start..end at this time.
    pub active: bool,
    /// Will be drawn (active, not hidden, not only a matte, in front of the camera).
    pub visible: bool,
    pub local_t: f64,
    pub opacity: f64,
    /// Where the layer's origin lands on screen.
    pub origin: [f64; 2],
    /// Screen-space bounding box `[x0, y0, x1, y1]` of what the layer draws.
    pub bounds: Option<[f64; 4]>,
    pub blend: Option<String>,
    pub depth: f64,
    #[serde(skip)]
    pub blend_mode: Option<Blend>,
    #[serde(skip)]
    pub screen: Mat4,
    #[serde(skip)]
    pub world: Mat4,
    pub three_d: bool,
    /// Blur from an entrance or exit preset, px.
    pub extra_blur: f64,
    pub extra_smear: f64,
    pub masks: Vec<MaskState>,
    pub content: Content,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Content {
    None,
    Solid { color: Rgba, size: [f64; 2] },
    Shape { items: Vec<ShapeState> },
    Text(text::TextState),
    Image { view: media::View, parallax: Option<media::ParallaxState> },
    Video { view: media::View, src_time: f64 },
    Precomp { frame: Option<Box<Frame>> },
    Adjustment,
}

impl<'p> Evaluator<'p> {
    pub fn new(plan: &'p Plan) -> Self {
        Evaluator { plan, text: crate::text::layout::Engine::new(), signals: Default::default() }
    }

    pub fn frame(&self, comp: &str, t: f64) -> Result<Frame> {
        let c = self.plan.comps.get(comp).ok_or_else(|| anyhow!("no comp {comp:?}"))?;
        let mut cache: Vec<Option<(Mat4, Mat4)>> = vec![None; c.layers.len()];
        let mut layers = Vec::with_capacity(c.layers.len());
        let env0 = LayerEnv::new(self, c, 0, t, 0);
        let bg = c.bg.as_ref().map(|b| b.at(t, &env0)).transpose().context("bg")?;

        // The topmost active camera and all active lights.
        let mut camera = None;
        let mut lights = vec![];
        for l in c.layers.iter().filter(|l| is_active(l, t)) {
            match &l.def.kind {
                Kind::Camera(cam) => camera = Some(self.camera(c, l, cam, t).with_context(|| l.label.clone())?),
                Kind::Light(light) => lights.push(self.light(c, l, light, t).with_context(|| l.label.clone())?),
                _ => {}
            }
        }

        for l in &c.layers {
            let st = self.layer(c, l, t, &mut cache, camera.as_ref()).with_context(|| format!("{} at {t:.3}s", l.label))?;
            layers.push(st);
        }
        Ok(Frame { comp: c.name.clone(), t, width: c.width, height: c.height, bg, camera, lights, layers })
    }

    /// The analysed sound of the audio or video layer with id `id` (anywhere in the file), and the file time at `t_comp`.
    pub fn signal(&self, id: &str, comp: &CompPlan, t_comp: f64) -> Result<(std::rc::Rc<crate::audio::signal::Signal>, f64)> {
        let (c, l) = std::iter::once(comp)
            .chain(self.plan.comps.values())
            .find_map(|c| c.layers.iter().find(|l| l.def.common.id.as_deref() == Some(id)).map(|l| (c, l)))
            .ok_or_else(|| anyhow!("no audio or video layer with id {id:?} to analyse"))?;
        let (path, src_in, speed, start) = match (&l.def.kind, &l.media) {
            (Kind::Audio(a), crate::plan::Media::Audio { path, .. }) => (path, a.src_in, a.speed, l.start),
            (Kind::Video(v), crate::plan::Media::Video { path, .. }) => (path, v.src_in, v.speed, l.start),
            _ => return Err(anyhow!("layer {id:?} is not an audio or video layer")),
        };
        let _ = c;
        let key = path.display().to_string();
        let known = self.signals.borrow().get(&key).cloned();
        let sig = match known {
            Some(s) => s,
            None => {
                let s = std::rc::Rc::new(crate::audio::signal::Signal::load(path)?);
                self.signals.borrow_mut().insert(key, s.clone());
                s
            }
        };
        Ok((sig, src_in + (t_comp - start) * speed))
    }

    /// Transform properties of a layer at composition time `t`, before parenting.
    pub fn local_props(&self, c: &CompPlan, l: &LayerPlan, t: f64, depth: u32) -> Result<Props> {
        let lt = t - l.start;
        let cm = &l.def.common;
        let (cw, ch) = (f64::from(c.width), f64::from(c.height));
        let env = |salt: u32| LayerEnv::new(self, c, l.index, t, depth).with_salt(salt);
        let vec = |p: &Option<crate::schema::Pos>, default: Vec<f64>, salt: u32| -> Result<Vec<f64>> {
            match p {
                None => Ok(default),
                Some(p) => {
                    let mut v = Vec::new();
                    for (i, c) in p.0 .0.iter().enumerate() {
                        v.push(c.at(lt, &env(salt + i as u32))?);
                    }
                    Ok(v)
                }
            }
        };
        let raster = matches!(l.def.kind, Kind::Solid(_) | Kind::Image(_) | Kind::Video(_) | Kind::Texture(_) | Kind::Precomp(_));
        let default_pos = if raster { vec![cw / 2.0, ch / 2.0, 0.0] } else { vec![0.0, 0.0, 0.0] };
        let mut pos = vec(&cm.pos, default_pos, 10)?;
        pos.resize(3, 0.0);
        let mut anchor = vec(&cm.anchor, vec![], 20)?;
        let size = self.layer_size(c, l);
        if anchor.is_empty() {
            if let Kind::Text(tx) = &l.def.kind {
                let lay = text::layout_of(self, c, l, tx, t)?;
                anchor = vec![f64::from(lay.anchor[0]), f64::from(lay.anchor[1]), 0.0];
            } else {
                anchor = if raster { vec![size[0] / 2.0, size[1] / 2.0, 0.0] } else { vec![0.0, 0.0, 0.0] };
            }
        }
        anchor.resize(3, 0.0);
        let mut scale = match &cm.scale {
            None => vec![1.0, 1.0, 1.0],
            Some(s) => {
                let mut v = Vec::new();
                for (i, c) in s.0 .0.iter().enumerate() {
                    v.push(c.at(lt, &env(30 + i as u32))?);
                }
                v
            }
        };
        if scale.len() == 2 {
            scale.push(1.0);
        }
        let prop = |p: &Option<crate::schema::P>, d: f64, salt: u32| -> Result<f64> { p.as_ref().map_or(Ok(d), |p| p.0.at(lt, &env(salt))) };
        let mut rotation = prop(&cm.rotation, 0.0, 40)?;
        if let (Some(a), Some(geo)) = (&cm.along, &l.along) {
            let u = a.t.0.at(lt, &env(45))?;
            let (p, ang) = shape::point_on(&geo.path, u);
            pos[0] += p[0];
            pos[1] += p[1];
            if a.orient {
                rotation += ang;
            }
        }
        Ok(Props {
            pos: [pos[0], pos[1], pos[2]],
            anchor: [anchor[0], anchor[1], anchor[2]],
            scale: [scale[0], scale[1], scale[2]],
            rotation,
            rot_x: prop(&cm.rot_x, 0.0, 41)?,
            rot_y: prop(&cm.rot_y, 0.0, 42)?,
            opacity: prop(&cm.opacity, 1.0, 43)?.clamp(0.0, 1.0),
        })
    }

    /// The natural size of a layer in its own space.
    pub fn layer_size(&self, c: &CompPlan, l: &LayerPlan) -> [f64; 2] {
        let full = [f64::from(c.width), f64::from(c.height)];
        match &l.def.kind {
            Kind::Solid(s) => s.size.unwrap_or(full),
            Kind::Precomp(p) => self.plan.comps.get(&p.comp).map_or(full, |n| [f64::from(n.width), f64::from(n.height)]),
            Kind::Texture(t) => t.size.unwrap_or(full),
            Kind::Image(i) => match (&l.media, i.fit) {
                (crate::plan::Media::Image { info, .. }, Fit::None) => i.size.unwrap_or([f64::from(info.w), f64::from(info.h)]),
                _ => i.size.unwrap_or(full),
            },
            Kind::Video(v) => match (&l.media, v.fit) {
                (crate::plan::Media::Video { info, .. }, Fit::None) => v.size.unwrap_or([f64::from(info.w), f64::from(info.h)]),
                _ => v.size.unwrap_or(full),
            },
            _ => full,
        }
    }

    fn local_matrix(p: &Props) -> Mat4 {
        Mat4::translate(p.pos[0], p.pos[1], p.pos[2])
            .mul(&Mat4::rot_z(p.rotation))
            .mul(&Mat4::rot_y(p.rot_y))
            .mul(&Mat4::rot_x(p.rot_x))
            .mul(&Mat4::scale(p.scale[0], p.scale[1], p.scale[2]))
            .mul(&Mat4::translate(-p.anchor[0], -p.anchor[1], -p.anchor[2]))
    }

    /// World matrix of a layer (through its parents) and its own transform props.
    fn world(&self, c: &CompPlan, i: usize, t: f64, cache: &mut Vec<Option<(Mat4, Mat4)>>) -> Result<Mat4> {
        if let Some((w, _)) = cache[i] {
            return Ok(w);
        }
        let l = &c.layers[i];
        let local = Self::local_matrix(&self.local_props(c, l, t, 0)?);
        let w = match l.parent {
            Some(p) => self.world(c, p, t, cache)?.mul(&local),
            None => local,
        };
        cache[i] = Some((w, local));
        Ok(w)
    }

    fn layer(&self, c: &CompPlan, l: &LayerPlan, t: f64, cache: &mut Vec<Option<(Mat4, Mat4)>>, camera: Option<&CameraState>) -> Result<LayerState> {
        let lt = t - l.start;
        let active = is_active(l, t);
        let cm = &l.def.common;
        let world = self.world(c, l.index, t, cache)?;
        let props = self.local_props(c, l, t, 0)?;
        let size = self.layer_size(c, l);

        // entrance / exit
        let mut m = transit::Mod::NONE;
        let dur = l.end - l.start;
        if let Some(tr) = &cm.enter {
            m = m.combine(transit_mod(tr, lt - tr.delay, true)?);
        }
        if let Some(tr) = &cm.exit {
            m = m.combine(transit_mod(tr, dur - lt - tr.delay, false)?);
        }
        let pre = if m != transit::Mod::NONE {
            // scale and rotate about the layer's centre of mass: its anchor
            let a = props.anchor;
            Mat4::translate(m.dx, m.dy, 0.0).mul(&Mat4::translate(a[0], a[1], 0.0)).mul(&Mat4::rot_z(m.rotation)).mul(&Mat4::scale(m.scale, m.scale, 1.0)).mul(&Mat4::translate(-a[0], -a[1], 0.0))
        } else {
            Mat4::IDENTITY
        };
        let world = if cm.three_d || m == transit::Mod::NONE { world.mul(&pre) } else { pre.mul(&world) };

        let (screen, depth) = match (cm.three_d, camera) {
            (true, Some(cam)) => {
                let z = cam.view.apply(world.0[3], world.0[7], world.0[11])[2];
                (cam.view_proj.mul(&world), z)
            }
            _ => (world, 0.0),
        };

        let (content, local_bounds) = self.content(c, l, t, lt, size, active)?;
        let mut visible = active && !cm.hidden && !l.is_matte_source && !matches!(l.def.kind, Kind::Camera(_) | Kind::Light(_) | Kind::Null | Kind::Audio(_));
        let origin = screen.project(0.0, 0.0, 0.0).map_or([f64::NAN, f64::NAN], |p| [p[0], p[1]]);
        let bounds = local_bounds.and_then(|b| project_bounds(&screen, b, &mut visible));
        Ok(LayerState {
            index: l.index,
            id: cm.id.clone(),
            label: l.label.clone(),
            active,
            visible,
            local_t: lt,
            opacity: props.opacity * m.opacity,
            origin,
            bounds,
            blend: cm.blend.map(|b| format!("{b:?}").to_lowercase()),
            depth,
            blend_mode: cm.blend,
            screen,
            world,
            three_d: cm.three_d,
            extra_blur: m.blur,
            extra_smear: m.smear,
            masks: shape::masks(self, c, l, t)?,
            content,
        })
    }

    fn content(&self, c: &CompPlan, l: &LayerPlan, t: f64, lt: f64, size: [f64; 2], active: bool) -> Result<(Content, Option<[f64; 4]>)> {
        let env = LayerEnv::new(self, c, l.index, t, 0);
        Ok(match &l.def.kind {
            Kind::Solid(s) => (Content::Solid { color: s.color.0.at(lt, &env)?, size }, Some([0.0, 0.0, size[0], size[1]])),
            Kind::Shape(s) => {
                let items = s.items.iter().zip(&l.shapes).enumerate().map(|(k, (it, geo))| shape::state(self, c, l, it, geo, k, t).with_context(|| format!("item {k}"))).collect::<Result<Vec<_>>>()?;
                let b = shape::bounds(&items, &l.shapes);
                (Content::Shape { items }, b)
            }
            Kind::Precomp(p) => {
                let frame = if active {
                    let inner = self.plan.comps.get(&p.comp).ok_or_else(|| anyhow!("no comp {:?}", p.comp))?;
                    let mut it = match &p.remap {
                        Some(r) => r.0.at(lt, &env)?,
                        None => lt * p.speed + p.offset,
                    };
                    if p.looping && inner.dur > 0.0 {
                        it = it.rem_euclid(inner.dur);
                    }
                    Some(Box::new(self.frame(&p.comp, it.clamp(0.0, (inner.dur - 1e-6).max(0.0)))?))
                } else {
                    None
                };
                (Content::Precomp { frame }, Some([0.0, 0.0, size[0], size[1]]))
            }
            Kind::Text(tx) => {
                let st = text::state(self, c, l, tx, t)?;
                let b = [0.0, 0.0, f64::from(st.width), f64::from(st.height)];
                (Content::Text(st), Some(b))
            }
            Kind::Image(im) => {
                let (view, parallax) = media::image_state(self, c, l, im, t, size)?;
                (Content::Image { view, parallax }, Some([0.0, 0.0, size[0], size[1]]))
            }
            Kind::Video(v) => {
                let crate::plan::Media::Video { info, .. } = &l.media else { unreachable!("probed at plan time") };
                let view = media::video_view(v, l, size);
                (Content::Video { view, src_time: media::video_time(v, info.dur, lt) }, Some([0.0, 0.0, size[0], size[1]]))
            }
            Kind::Adjustment => (Content::Adjustment, None),
            _ => (Content::None, None),
        })
    }

    fn camera(&self, c: &CompPlan, l: &LayerPlan, cam: &crate::schema::Camera, t: f64) -> Result<CameraState> {
        let lt = t - l.start;
        let env = |salt: u32| LayerEnv::new(self, c, l.index, t, 0).with_salt(salt);
        let (w, h) = (f64::from(c.width), f64::from(c.height));
        let zoom = match (&cam.zoom, &cam.fov) {
            (Some(z), _) => z.0.at(lt, &env(1))?,
            (None, Some(f)) => (w / 2.0) / (f.0.at(lt, &env(2))?.to_radians() / 2.0).tan(),
            (None, None) => 1.5 * w,
        }
        .max(1.0);
        let vec3 = |p: &Option<crate::schema::Pos>, d: [f64; 3], salt: u32| -> Result<[f64; 3]> {
            match p {
                None => Ok(d),
                Some(p) => {
                    let mut v = [d[0], d[1], d[2]];
                    for (i, comp) in p.0 .0.iter().enumerate().take(3) {
                        v[i] = comp.at(lt, &env(salt + i as u32))?;
                    }
                    Ok(v)
                }
            }
        };
        let pos = vec3(&cam.pos, [w / 2.0, h / 2.0, -zoom], 10)?;
        let poi = vec3(&cam.poi, [w / 2.0, h / 2.0, 0.0], 20)?;
        let roll = cam.roll.as_ref().map_or(Ok(0.0), |r| r.0.at(lt, &env(30)))?;
        let dof = cam.dof.as_ref().map(|d| Ok::<_, anyhow::Error>([d.focus.0.at(lt, &env(31))?, d.blur.0.at(lt, &env(32))?])).transpose()?;
        let view = Mat4::look_at(pos, poi, roll);
        let view_proj = Mat4::pinhole(zoom, w / 2.0, h / 2.0).mul(&view);
        Ok(CameraState { pos, poi, zoom, roll, dof, view_proj, view })
    }

    fn light(&self, c: &CompPlan, l: &LayerPlan, light: &crate::schema::Light, t: f64) -> Result<LightState> {
        let lt = t - l.start;
        let env = |salt: u32| LayerEnv::new(self, c, l.index, t, 0).with_salt(salt);
        let (w, h) = (f64::from(c.width), f64::from(c.height));
        let vec3 = |p: &Option<crate::schema::Pos>, d: [f64; 3], salt: u32| -> Result<[f64; 3]> {
            match p {
                None => Ok(d),
                Some(p) => {
                    let mut v = [d[0], d[1], d[2]];
                    for (i, comp) in p.0 .0.iter().enumerate().take(3) {
                        v[i] = comp.at(lt, &env(salt + i as u32))?;
                    }
                    Ok(v)
                }
            }
        };
        let pos = vec3(&light.pos, [w / 2.0, h / 2.0, -600.0], 10)?;
        let target = vec3(&light.target, [w / 2.0, h / 2.0, 0.0], 20)?;
        let dir = {
            let d = [target[0] - pos[0], target[1] - pos[1], target[2] - pos[2]];
            let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-9);
            [d[0] / n, d[1] / n, d[2] / n]
        };
        let col = light.color.as_ref().map_or(Ok([255.0, 255.0, 255.0, 1.0]), |c| c.0.at(lt, &env(30)).map(|c| c.map(f64::from)))?;
        let intensity = light.intensity.as_ref().map_or(Ok(1.0), |i| i.0.at(lt, &env(31)))?.max(0.0);
        Ok(LightState {
            kind: format!("{:?}", light.kind).to_lowercase(),
            pos,
            dir,
            color: [col[0] / 255.0 * intensity, col[1] / 255.0 * intensity, col[2] / 255.0 * intensity],
            radius: light.radius.unwrap_or(1200.0),
            cone: light.cone.unwrap_or(40.0),
            shine: light.shine.unwrap_or(0.0),
        })
    }
}

/// The evaluated transform properties of one layer.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Props {
    pub pos: [f64; 3],
    pub anchor: [f64; 3],
    pub scale: [f64; 3],
    pub rotation: f64,
    pub rot_x: f64,
    pub rot_y: f64,
    pub opacity: f64,
}

pub fn is_active(l: &LayerPlan, t: f64) -> bool {
    t >= l.start - 1e-9 && t < l.end - 1e-9
}

/// `since` is seconds since an entrance began, or seconds left before an exit ends.
fn transit_mod(tr: &Transit, since: f64, entering: bool) -> Result<transit::Mod> {
    let ease: Ease = transit::ease_of(tr, entering)?;
    let u = (since / tr.dur.max(1e-6)).clamp(0.0, 1.0);
    // An exit eases the hidden amount 0 -> 1 as the layer's end approaches, so it accelerates away.
    let shown = if entering { ease.at(u) } else { 1.0 - ease.at(1.0 - u) };
    transit::apply(tr, shown)
}

fn project_bounds(m: &Mat4, b: [f64; 4], visible: &mut bool) -> Option<[f64; 4]> {
    let mut pts = vec![];
    for (x, y) in [(b[0], b[1]), (b[2], b[1]), (b[2], b[3]), (b[0], b[3])] {
        match m.project(x, y, 0.0) {
            Some(p) => pts.push(p),
            None => {
                *visible = false;
                return None;
            }
        }
    }
    let x0 = pts.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    let x1 = pts.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
    let y0 = pts.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
    let y1 = pts.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
    Some([x0, y0, x1, y1])
}
