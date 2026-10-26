//! Evaluation: `Plan × time → Frame`. Everything a renderer needs to draw one instant, as plain numbers: where each
//! layer is, how opaque it is, which part of a stroke is drawn. No GPU is involved, so `inspect` and `check` use
//! this directly, and a test can assert that a title is inside the safe area at 1.2 s without drawing a pixel.

pub mod buffer;
pub mod env;
pub mod fx;
pub mod mat4;
pub mod particles;
pub mod pose;
pub mod salt;
pub mod media;
pub mod shape;
pub mod text;
pub mod mesh;
pub mod transit;
mod travel;

#[cfg(test)]
mod light_tests;
#[cfg(test)]
mod tests;

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
    /// Meshes, built once per distinct geometry and size.
    pub(crate) meshes: std::cell::RefCell<std::collections::HashMap<crate::mesh::MeshKey, std::rc::Rc<crate::mesh::Mesh>>>,
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
    /// The composition's `look` chain.
    pub look: Vec<fx::FxState>,
    pub layers: Vec<LayerState>,
}

#[derive(Serialize, Clone)]
pub struct CameraState {
    pub pos: [f64; 3],
    pub poi: [f64; 3],
    pub zoom: f64,
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
    /// The kind of layer: `"solid"`, `"text"`, `"camera"`, `"light"`, ...
    pub kind: &'static str,
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
    /// The layer stands in the composition's shared 3D space.
    pub in_3d: bool,
    /// Blur from an entrance or exit preset, px.
    pub extra_blur: f64,
    pub extra_smear: f64,
    /// A wipe / iris from an enter preset, in the layer box 0..1.
    pub clip: Option<transit::Clip>,
    pub masks: Vec<MaskState>,
    pub fx: Vec<fx::FxState>,
    /// Present when the layer runs its effects and masks on its own pixels (`effect_space: layer`): its picture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local: Option<buffer::LocalBuffer>,
    /// Present when lights shade this layer.
    pub lit: Option<LitState>,
    /// Present when the layer draws a floor reflection of itself.
    pub reflect: Option<ReflectState>,
    pub content: Content,
}

#[derive(Serialize, Clone, Copy)]
pub struct ReflectState {
    pub strength: f64,
    pub fade: f64,
    pub gap: f64,
    pub blur: f64,
}

/// Where a lit layer stands in the world, for the lighting shader and for `inspect`.
#[derive(Serialize, Clone)]
pub struct LitState {
    pub origin: [f64; 3],
    pub axis_u: [f64; 3],
    pub axis_v: [f64; 3],
    /// Unit normal (unflipped; the shader turns it toward the viewer).
    pub normal: [f64; 3],
    pub material: [f64; 4],
    /// Screen pixels to layer-plane coordinates, row-major 3x3.
    #[serde(skip)]
    pub inverse: [f64; 9],
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Content {
    None,
    Solid { color: Rgba, size: [f64; 2] },
    Shape { items: Vec<ShapeState> },
    Text(text::TextState),
    Mesh(mesh::MeshState),
    Image { view: media::View, parallax: Option<media::ParallaxState> },
    Video { view: media::View, src_time: f64 },
    Precomp { frame: Option<Box<Frame>> },
    Adjustment,
    Texture { size: [f64; 2], shader: fx::PluginState },
    Particles(particles::ParticleState),
    Pack { name: String, params: serde_json::Value, size: [f64; 2] },
}

impl<'p> Evaluator<'p> {
    pub fn new(plan: &'p Plan) -> Self {
        Evaluator { plan, text: crate::text::layout::Engine::new(), signals: Default::default(), meshes: Default::default() }
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
            let st = self.layer(c, l, t, &mut cache, camera.as_ref(), !lights.is_empty()).with_context(|| format!("{} at {t:.3}s", l.label))?;
            layers.push(st);
        }
        let look = fx::chain(self, c, 0, &c.look, t, t).context("look")?;
        Ok(Frame { comp: c.name.clone(), t, width: c.width, height: c.height, bg, camera, lights, look, layers })
    }

    /// The analysed sound of the audio or video layer with id `id` (anywhere in the file), and the file time at `t_comp`.
    pub fn signal(&self, id: &str, comp: &CompPlan, t_comp: f64) -> Result<(std::rc::Rc<crate::audio::signal::Signal>, f64)> {
        let (c, l) = std::iter::once(comp)
            .chain(self.plan.comps.values())
            .find_map(|c| c.layers.iter().find(|l| l.def.common.id.as_deref() == Some(id)).map(|l| (c, l)))
            .ok_or_else(|| anyhow!("no audio or video layer with id {id:?} to analyse"))?;
        let (path, src_in, speed, start) = match (&l.def.kind, &l.media) {
            (Kind::Audio(a), crate::plan::Media::Audio { path, rate, .. }) => (path, a.src_in, a.speed * rate, l.start),
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
        let vec = |p: &Option<crate::schema::Pos>, default: Vec<f64>, property: u32| -> Result<Vec<f64>> {
            match p {
                None => Ok(default),
                Some(p) => p.0.at_with(lt, |i| env(salt::axis(property, i))),
            }
        };
        let raster = matches!(l.def.kind, Kind::Solid(_) | Kind::Image(_) | Kind::Video(_) | Kind::Texture(_) | Kind::Precomp(_) | Kind::Pack(_));
        let default_pos = if raster || matches!(l.def.kind, Kind::Mesh(_)) { vec![cw / 2.0, ch / 2.0, 0.0] } else { vec![0.0, 0.0, 0.0] };
        let mut pos = vec(&cm.pos, default_pos, salt::POS)?;
        pos.resize(3, 0.0);
        let mut anchor = vec(&cm.anchor, vec![], salt::ANCHOR)?;
        let size = self.layer_size_at(c, l, t);
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
            Some(s) => s.0.at_with(lt, |i| env(salt::axis(salt::SCALE, i)))?,
        };
        if scale.len() == 2 {
            scale.push(1.0);
        }
        let prop = |p: &Option<crate::schema::P>, d: f64, salt: u32| -> Result<f64> { p.as_ref().map_or(Ok(d), |p| p.0.at(lt, &env(salt))) };
        let mut pose = pose::pose_at(cm, lt, &env)?;
        let mut aim = match &cm.aim {
            None => None,
            Some(p) => {
                let mut v = [0.0; 3];
                for (slot, x) in v.iter_mut().zip(p.0.at_with(lt, |i| env(salt::axis(salt::AIM, i)))?) {
                    *slot = x;
                }
                Some(Aim::Point(v))
            }
        };
        if let (Some(a), Some(geo)) = (&cm.along, &l.along) {
            let u = a.t.0.at(lt, &env(salt::ALONG_T))?;
            let (p, ang) = shape::point_on(&geo.path, u);
            pos[0] += p[0];
            pos[1] += p[1];
            if a.orient {
                travel::face(&mut pose, &mut aim, l.in_3d, [ang.to_radians().cos(), ang.to_radians().sin(), 0.0], ang);
            }
        }
        if let Some(dir) = cm.pos.as_ref().and_then(|p| p.0.heading(lt)).filter(|d| *d != [0.0; 3]) {
            travel::face(&mut pose, &mut aim, l.in_3d, dir, travel::heading_of(dir));
        }
        if let Some(w) = &cm.wrap {
            if let Some([a, b]) = w.x {
                let span = b - a;
                if span.abs() > 1e-9 {
                    pos[0] = a + (pos[0] - a).rem_euclid(span);
                }
            }
            if let Some([a, b]) = w.y {
                let span = b - a;
                if span.abs() > 1e-9 {
                    pos[1] = a + (pos[1] - a).rem_euclid(span);
                }
            }
        }
        Ok(Props {
            pos: [pos[0], pos[1], pos[2]],
            anchor: [anchor[0], anchor[1], anchor[2]],
            scale: [scale[0], scale[1], scale[2]],
            pose,
            aim,
            skew: prop(&cm.skew, 0.0, salt::SKEW)?,
            opacity: prop(&cm.opacity, 1.0, salt::OPACITY)?.clamp(0.0, 1.0),
        })
    }

    /// The natural size of a layer in its own space.
    pub fn layer_size(&self, c: &CompPlan, l: &LayerPlan) -> [f64; 2] {
        self.layer_size_at(c, l, l.start)
    }

    fn layer_size_at(&self, c: &CompPlan, l: &LayerPlan, t: f64) -> [f64; 2] {
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
            Kind::Text(tx) => match text::layout_of(self, c, l, tx, t) {
                Ok(lay) => [f64::from(lay.width).max(1.0), f64::from(lay.height).max(1.0)],
                Err(_) => full,
            },
            _ => full,
        }
    }

    /// The matrix of a layer inside its parent: `T(pos) · pose · skew · scale · T(-anchor)`.
    ///
    /// `parent` is the parent's world matrix, needed only by `Aim::Point`, which is in composition (world) space
    /// like After Effects' auto-orient toward the point of interest: it is brought into the parent's space through
    /// the parent's inverse, so the layer faces that world point whatever its parent does.
    fn local_matrix(p: &Props, parent: Option<&Mat4>) -> Result<Mat4> {
        let facing = match p.aim {
            None => Mat4::IDENTITY,
            Some(Aim::Point(target)) => {
                let t = match parent {
                    Some(w) => {
                        let a = w.inverse().ok_or_else(|| anyhow!("a layer aims at a point, but its parent is collapsed (a zero scale) so the point has no place in it"))?.apply(target[0], target[1], target[2]);
                        [a[0], a[1], a[2]]
                    }
                    None => target,
                };
                Mat4::face([t[0] - p.pos[0], t[1] - p.pos[1], t[2] - p.pos[2]])
            }
            Some(Aim::Travel(dir)) => Mat4::face([-dir[0], -dir[1], -dir[2]]),
        };
        Ok(Mat4::translate(p.pos[0], p.pos[1], p.pos[2])
            .mul(&p.pose.matrix(&facing))
            .mul(&Mat4::skew_x(p.skew))
            .mul(&Mat4::scale(p.scale[0], p.scale[1], p.scale[2]))
            .mul(&Mat4::translate(-p.anchor[0], -p.anchor[1], -p.anchor[2])))
    }

    /// World matrix of a layer (through its parents) and its own transform props.
    fn world(&self, c: &CompPlan, i: usize, t: f64, cache: &mut Vec<Option<(Mat4, Mat4)>>) -> Result<Mat4> {
        if let Some((w, _)) = cache[i] {
            return Ok(w);
        }
        let l = &c.layers[i];
        let parent = l.parent.map(|p| self.world(c, p, t, cache)).transpose()?;
        let local = Self::local_matrix(&self.local_props(c, l, t, 0)?, parent.as_ref())?;
        let rest = match parent {
            Some(pw) => pw.mul(&local),
            None => local,
        };
        cache[i] = Some((rest, local));
        let w = self.apply_flip(c, l, t, rest, cache)?;
        cache[i] = Some((w, local));
        Ok(w)
    }

    fn flip_progress(&self, l: &LayerPlan, t: f64) -> Result<Option<f64>> {
        let Some(flip) = &l.def.common.flip else { return Ok(None) };
        Ok(Some(flip_u(l, t, flip)?))
    }

    /// GSAP Flip: lerp this layer's origin from `from`'s origin onto its rest origin.
    fn apply_flip(&self, c: &CompPlan, l: &LayerPlan, t: f64, world: Mat4, cache: &mut Vec<Option<(Mat4, Mat4)>>) -> Result<Mat4> {
        let Some(src_i) = l.flip_from else { return Ok(world) };
        let flip = l.def.common.flip.as_ref().expect("flip_from is set with flip");
        let u = flip_u(l, t, flip)?;
        if u >= 1.0 - 1e-12 {
            return Ok(world);
        }
        let src_w = self.world(c, src_i, t, cache)?;
        let src_p = self.local_props(c, &c.layers[src_i], t, 0)?;
        let dst_p = self.local_props(c, l, t, 0)?;
        let s = src_w.apply(src_p.anchor[0], src_p.anchor[1], src_p.anchor[2]);
        let d = world.apply(dst_p.anchor[0], dst_p.anchor[1], dst_p.anchor[2]);
        Ok(Mat4::translate((s[0] + (d[0] - s[0]) * u) - d[0], (s[1] + (d[1] - s[1]) * u) - d[1], (s[2] + (d[2] - s[2]) * u) - d[2]).mul(&world))
    }

    fn layer(&self, c: &CompPlan, l: &LayerPlan, t: f64, cache: &mut Vec<Option<(Mat4, Mat4)>>, camera: Option<&CameraState>, any_lights: bool) -> Result<LayerState> {
        let lt = t - l.start;
        let active = is_active(l, t);
        let cm = &l.def.common;
        let world = self.world(c, l.index, t, cache)?;
        let props = self.local_props(c, l, t, 0)?;
        let size = self.layer_size_at(c, l, t);

        // entrance / exit
        let mut m = transit::Mod::NONE;
        let dur = l.end - l.start;
        let frame = [f64::from(c.width), f64::from(c.height)];
        if let Some(tr) = &cm.enter {
            m = m.combine(transit_mod(tr, lt - tr.delay, true, frame)?);
        }
        if let Some(tr) = &cm.exit {
            m = m.combine(transit_mod(tr, dur - lt - tr.delay, false, frame)?);
        }
        let pre = if m != transit::Mod::NONE {
            // scale and rotate about the layer's centre of mass: its anchor
            let a = props.anchor;
            Mat4::translate(m.dx, m.dy, 0.0).mul(&Mat4::translate(a[0], a[1], 0.0)).mul(&Mat4::rot_z(m.rotation)).mul(&Mat4::scale(m.scale, m.scale, 1.0)).mul(&Mat4::translate(-a[0], -a[1], 0.0))
        } else {
            Mat4::IDENTITY
        };
        let world = if l.in_3d || m == transit::Mod::NONE { world.mul(&pre) } else { pre.mul(&world) };

        let (screen, depth) = match (l.in_3d, camera) {
            (true, Some(cam)) => {
                // depth of the layer's anchor point (where `pos` puts it), not of its corner
                let a = world.apply(props.anchor[0], props.anchor[1], props.anchor[2]);
                let z = cam.view.apply(a[0], a[1], a[2])[2];
                (cam.view_proj.mul(&world), z)
            }
            _ => (world, 0.0),
        };

        let (content, local_bounds) = self.content(c, l, t, lt, size, active)?;
        // depth of field: a 3D layer away from the focal distance softens in proportion to how far off it is
        let dof_blur = match (l.in_3d, camera.and_then(|c| c.dof)) {
            (true, Some([focus, blur])) if focus > 1.0 => blur * (depth - focus).abs() / focus,
            _ => 0.0,
        };
        let lit = if any_lights && l.in_3d && cm.lit.unwrap_or(true) && !matches!(l.def.kind, Kind::Camera(_) | Kind::Light(_) | Kind::Null | Kind::Adjustment | Kind::Audio(_) | Kind::Mesh(_)) {
            lit_state(&world, &screen, cm.material.unwrap_or_default())
        } else {
            None
        };
        let mut visible = active && !cm.hidden && !l.is_matte_source && !matches!(l.def.kind, Kind::Camera(_) | Kind::Light(_) | Kind::Null | Kind::Audio(_));
        if visible && l.in_3d && cm.cull.unwrap_or_default() != crate::schema::Cull::None {
            if let Some(cam) = camera {
                visible = !culled(&world, cam, cm.cull.unwrap_or_default());
            }
        }
        let origin = screen.project(0.0, 0.0, 0.0).map_or([f64::NAN, f64::NAN], |p| [p[0], p[1]]);
        let mut bounds = match &content {
            Content::Mesh(t) => mesh::bounds(&screen, t, &mut visible),
            _ => local_bounds.and_then(|b| project_bounds(&screen, b, &mut visible)),
        };
        let env_r = LayerEnv::new(self, c, l.index, t, 0);
        let reflect = match &cm.reflect {
            None => None,
            Some(r) => {
                let strength = r.strength.as_ref().map_or(Ok(0.4), |p| p.0.at(lt, &env_r))?.clamp(0.0, 1.0);
                let fade = r.fade.unwrap_or(0.55).clamp(0.05, 1.0);
                let gap = r.gap.unwrap_or(8.0).max(0.0);
                let blur = r.blur.unwrap_or(3.0).max(0.0);
                if let Some(b) = bounds.as_mut() {
                    b[3] += gap + (b[3] - b[1]).max(0.0) * fade;
                }
                Some(ReflectState { strength, fade, gap, blur })
            }
        };
        let fx_states = fx::chain(self, c, l.index, &l.fx, t, lt)?;
        let local = buffer::for_layer(l, &fx_states, &self.plan.fx, lt, local_bounds, |b| project_bounds(&screen, b, &mut true))?;
        let flip_u = self.flip_progress(l, t)?;
        if let Some(u) = flip_u {
            if u <= 1e-9 {
                visible = false;
            }
        }
        Ok(LayerState {
            index: l.index,
            id: cm.id.clone(),
            label: l.label.clone(),
            kind: l.def.kind.name(),
            active,
            visible,
            local_t: lt,
            opacity: props.opacity * m.opacity * if flip_u.is_some_and(|u| u <= 1e-9) { 0.0 } else { 1.0 },
            origin,
            bounds,
            blend: cm.blend.map(|b| b.name().to_string()),
            depth,
            blend_mode: cm.blend,
            screen,
            world,
            in_3d: l.in_3d,
            extra_blur: m.blur + dof_blur,
            extra_smear: m.smear,
            clip: m.clip,
            masks: shape::masks(self, c, l, t)?,
            fx: fx_states,
            local,
            lit,
            reflect,
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
                let b = st.placed_bounds.unwrap_or([0.0, 0.0, f64::from(st.width), f64::from(st.height)]);
                (Content::Text(st), Some(b))
            }
            Kind::Mesh(m) => (Content::Mesh(mesh::state(self, c, l, m, t)?), None),
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
            Kind::Pack(p) => (Content::Pack { name: p.name.clone(), params: p.params.clone(), size }, Some([0.0, 0.0, size[0], size[1]])),
            Kind::Particles(p) => {
                let st = particles::state(self, c, l, p, t, lt)?;
                (Content::Particles(st), Some([0.0, 0.0, size[0], size[1]]))
            }
            Kind::Texture(_) => {
                let plan = l.texture.as_ref().ok_or_else(|| anyhow!("texture layer without a planned shader"))?;
                (Content::Texture { size, shader: fx::plugin(self, c, l.index, plan, t, lt, salt::TEXTURE_PLUGIN)? }, Some([0.0, 0.0, size[0], size[1]]))
            }
            _ => (Content::None, None),
        })
    }

    fn camera(&self, c: &CompPlan, l: &LayerPlan, cam: &crate::schema::Camera, t: f64) -> Result<CameraState> {
        let lt = t - l.start;
        let env = |salt: u32| LayerEnv::new(self, c, l.index, t, 0).with_salt(salt);
        let (w, h) = (f64::from(c.width), f64::from(c.height));
        let zoom = match (&cam.zoom, &cam.fov) {
            (Some(z), _) => z.0.at(lt, &env(salt::CAMERA_ZOOM))?,
            (None, Some(f)) => (w / 2.0) / (f.0.at(lt, &env(salt::CAMERA_FOV))?.to_radians() / 2.0).tan(),
            (None, None) => crate::schema::default_camera_zoom(w),
        }
        .max(1.0);
        let vec3 = |p: &Option<crate::schema::Pos>, d: [f64; 3], property: u32| -> Result<[f64; 3]> {
            match p {
                None => Ok(d),
                Some(p) => {
                    let mut v = [d[0], d[1], d[2]];
                    for (slot, x) in v.iter_mut().zip(p.0.at_with(lt, |i| env(salt::axis(property, i)))?) {
                        *slot = x;
                    }
                    Ok(v)
                }
            }
        };
        let pos = vec3(&l.def.common.pos, [w / 2.0, h / 2.0, -zoom], salt::POS)?;
        let poi = vec3(&cam.poi, [w / 2.0, h / 2.0, 0.0], salt::CAMERA_POI)?;
        let dof = cam.dof.as_ref().map(|d| Ok::<_, anyhow::Error>([d.focus.0.at(lt, &env(salt::CAMERA_DOF_FOCUS))?, d.blur.0.at(lt, &env(salt::CAMERA_DOF_BLUR))?])).transpose()?;
        // The Point of Interest aims the camera; its Orientation and X, Y and Z Rotation turn it about its own
        // position, inside that aim. That is the layer's own pose, evaluated by the same function: the Z Rotation
        // is the roll.
        let pose = pose::pose_at(&l.def.common, lt, &env)?;
        let facing = Mat4::look_at(pos, poi).transpose_rotation();
        let view = pose.inverse(&facing).mul(&Mat4::translate(-pos[0], -pos[1], -pos[2]));
        let view_proj = Mat4::pinhole(zoom, w / 2.0, h / 2.0).mul(&view);
        Ok(CameraState { pos, poi, zoom, dof, view_proj, view })
    }

    fn light(&self, c: &CompPlan, l: &LayerPlan, light: &crate::schema::Light, t: f64) -> Result<LightState> {
        let lt = t - l.start;
        let env = |salt: u32| LayerEnv::new(self, c, l.index, t, 0).with_salt(salt);
        let (w, h) = (f64::from(c.width), f64::from(c.height));
        let vec3 = |p: &Option<crate::schema::Pos>, d: [f64; 3], property: u32| -> Result<[f64; 3]> {
            match p {
                None => Ok(d),
                Some(p) => {
                    let mut v = [d[0], d[1], d[2]];
                    for (slot, x) in v.iter_mut().zip(p.0.at_with(lt, |i| env(salt::axis(property, i)))?) {
                        *slot = x;
                    }
                    Ok(v)
                }
            }
        };
        let pos = vec3(&l.def.common.pos, [w / 2.0, h / 2.0, -600.0], salt::POS)?;
        let target = vec3(&light.target, [w / 2.0, h / 2.0, 0.0], salt::LIGHT_TARGET)?;
        let dir = {
            let d = [target[0] - pos[0], target[1] - pos[1], target[2] - pos[2]];
            let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-9);
            [d[0] / n, d[1] / n, d[2] / n]
        };
        let col = light.color.as_ref().map_or(Ok([255.0, 255.0, 255.0, 1.0]), |c| c.0.at(lt, &env(salt::LIGHT_COLOR)).map(|c| c.map(f64::from)))?;
        let intensity = light.intensity.as_ref().map_or(Ok(1.0), |i| i.0.at(lt, &env(salt::LIGHT_INTENSITY)))?.max(0.0);
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

/// Whether the camera sees the side of a layer that `cull` hides.
///
/// The layer's readable side is its local `-z`, so the camera is on it when it lies against the layer's local
/// `+z` axis.
fn culled(world: &Mat4, cam: &CameraState, cull: crate::schema::Cull) -> bool {
    let w = &world.0;
    let normal = [w[2], w[6], w[10]];
    let origin = [w[3], w[7], w[11]];
    let to_camera = [cam.pos[0] - origin[0], cam.pos[1] - origin[1], cam.pos[2] - origin[2]];
    let towards = normal[0] * to_camera[0] + normal[1] * to_camera[1] + normal[2] * to_camera[2];
    match cull {
        crate::schema::Cull::None => false,
        crate::schema::Cull::Front => towards < 0.0,
        crate::schema::Cull::Back => towards > 0.0,
    }
}

fn lit_state(world: &Mat4, screen: &Mat4, mat: crate::schema::Material) -> Option<LitState> {
    let w = &world.0;
    let origin = [w[3], w[7], w[11]];
    let axis_u = [w[0], w[4], w[8]];
    let axis_v = [w[1], w[5], w[9]];
    let cross = [axis_u[1] * axis_v[2] - axis_u[2] * axis_v[1], axis_u[2] * axis_v[0] - axis_u[0] * axis_v[2], axis_u[0] * axis_v[1] - axis_u[1] * axis_v[0]];
    let n = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    if n < 1e-9 {
        return None;
    }
    // layer plane (u, v) -> screen: the columns of the screen matrix for u, v and the origin
    let s = &screen.0;
    let h = [s[0], s[1], s[3], s[4], s[5], s[7], s[12], s[13], s[15]];
    let det = h[0] * (h[4] * h[8] - h[5] * h[7]) - h[1] * (h[3] * h[8] - h[5] * h[6]) + h[2] * (h[3] * h[7] - h[4] * h[6]);
    if det.abs() < 1e-12 {
        return None;
    }
    let d = 1.0 / det;
    let inverse = [
        (h[4] * h[8] - h[5] * h[7]) * d, (h[2] * h[7] - h[1] * h[8]) * d, (h[1] * h[5] - h[2] * h[4]) * d,
        (h[5] * h[6] - h[3] * h[8]) * d, (h[0] * h[8] - h[2] * h[6]) * d, (h[2] * h[3] - h[0] * h[5]) * d,
        (h[3] * h[7] - h[4] * h[6]) * d, (h[1] * h[6] - h[0] * h[7]) * d, (h[0] * h[4] - h[1] * h[3]) * d,
    ];
    Some(LitState { origin, axis_u, axis_v, normal: [cross[0] / n, cross[1] / n, cross[2] / n], material: [mat.ambient, mat.diffuse, mat.specular, mat.shininess.max(1.0)], inverse })
}

/// What turns a layer's face.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum Aim {
    /// Face a point in composition (world) space: the layer's `aim`.
    Point([f64; 3]),
    /// A 3D layer on a path with `along.orient`: its depth axis (local +z) runs along this direction of travel,
    /// in the parent's space. (A flat layer just turns by the path's heading, see `Pose::heading`.)
    Travel([f64; 3]),
}

/// The evaluated transform properties of one layer.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Props {
    pub pos: [f64; 3],
    pub anchor: [f64; 3],
    pub scale: [f64; 3],
    /// Orientation, X, Y and Z Rotation, and the path heading.
    pub pose: pose::Pose,
    pub aim: Option<Aim>,
    pub skew: f64,
    pub opacity: f64,
}

fn flip_u(l: &LayerPlan, t: f64, flip: &crate::schema::Flip) -> Result<f64> {
    let lt = t - l.start;
    if lt < flip.at {
        return Ok(0.0);
    }
    let ease = flip.ease.as_deref().map_or_else(|| Ease::parse("expo.inOut"), Ease::parse)?;
    Ok(ease.at(((lt - flip.at) / flip.dur.max(1e-6)).clamp(0.0, 1.0)))
}

pub fn is_active(l: &LayerPlan, t: f64) -> bool {
    t >= l.start - 1e-9 && t < l.end - 1e-9
}

/// `since` is seconds since an entrance began, or seconds left before an exit ends.
fn transit_mod(tr: &Transit, since: f64, entering: bool, frame: [f64; 2]) -> Result<transit::Mod> {
    let ease: Ease = transit::ease_of(tr, entering)?;
    let u = (since / tr.dur.max(1e-6)).clamp(0.0, 1.0);
    // An exit eases the hidden amount 0 -> 1 as the layer's end approaches, so it accelerates away.
    let shown = if entering { ease.at(u) } else { 1.0 - ease.at(1.0 - u) };
    transit::apply(tr, shown, entering, frame)
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
