//! Where each character of a text layer sits once the line is laid out: along a path, on a cylinder or sphere, in a
//! scrolling row, and through the text's own perspective. Animators move characters about their rest place; this
//! module moves the rest place itself, so an animator's `x` slides along the path and a `rotation_y` flips a letter
//! standing on a drum.

use skia_safe::{ContourMeasure, ContourMeasureIter, Path};

use super::{
    animate::{Aff, GlyphState},
    layout::Layout,
};
use crate::{
    eval::mat4::Mat4,
    schema::{Align, Marquee, SurfaceKind, TextPath},
};

/// A path with its margins already evaluated at this instant.
pub struct OnPath {
    contour: ContourMeasure,
    pub reverse: bool,
    pub perpendicular: bool,
    pub force_align: bool,
    pub first: f64,
    pub last: f64,
    pub align: Align,
}

/// A text path's animated margins at one instant: pixels, and `travel` in fractions of the path's length.
pub struct Margins {
    pub first: f64,
    pub last: f64,
    pub travel: f64,
}

/// The path's first contour, when it has a length to walk.
fn walkable(path: &Path) -> Option<ContourMeasure> {
    ContourMeasureIter::new(path, false, None).next().filter(|c| c.length() > 0.0)
}

/// Text cannot be set on a path with no length; said once, when the composition is planned.
pub fn ensure_walkable(path: &Path) -> anyhow::Result<()> {
    walkable(path).map(drop).ok_or_else(|| anyhow::anyhow!("the path has no length to walk"))
}

impl OnPath {
    /// `None` when the path has no length to walk (see `ensure_walkable`).
    pub fn new(path: &Path, opts: &TextPath, margins: Margins, align: Align) -> Option<OnPath> {
        let contour = walkable(path)?;
        let len = f64::from(contour.length());
        Some(OnPath { contour, reverse: opts.reverse, perpendicular: opts.perpendicular, force_align: opts.force_align, first: margins.first + margins.travel * len, last: margins.last, align })
    }

    pub fn length(&self) -> f64 {
        f64::from(self.contour.length())
    }

    pub fn closed(&self) -> bool {
        self.contour.is_closed()
    }

    /// The point `s` pixels along the path (wrapping on a closed path) and the direction of travel in degrees, or `None`
    /// past either end of an open path.
    pub fn at(&self, s: f64) -> Option<([f32; 2], f32)> {
        let len = self.length();
        let mut d = if self.closed() { s.rem_euclid(len) } else if (0.0..=len).contains(&s) { s } else { return None };
        if self.reverse {
            d = len - d;
        }
        let (p, tan) = self.contour.pos_tan(d as f32)?;
        let mut deg = tan.y.atan2(tan.x).to_degrees();
        if self.reverse {
            deg += 180.0;
        }
        Some(([p.x, p.y], deg))
    }
}

/// A cylinder or sphere the characters stand on, axis through the layer's position.
pub struct OnSurface {
    pub kind: SurfaceKind,
    pub radius: f64,
    /// Degrees.
    pub turn: f64,
    pub back: bool,
}

/// Copies beyond what the frame width needs: one for the partly visible copy at each edge.
const EXTRA_COPIES: usize = 2;

/// A scrolling row: `copies` laid `period` apart, the first at `shift` (always at or left of 0, so the row has no gap).
pub struct Row {
    pub period: f64,
    pub shift: f64,
    pub copies: usize,
}

impl Row {
    pub fn new(m: &Marquee, layout: &Layout, t: f64, comp_width: f64) -> Row {
        let period = f64::from(layout.width) + m.gap;
        // `-(-x mod period)` lies in (-period, 0], so the first copy always starts at or before the layer's origin
        let shift = -(-(m.speed * t + m.offset)).rem_euclid(period);
        let covering = (comp_width / period).ceil() as usize + EXTRA_COPIES;
        Row { period, shift, copies: m.repeat.map_or(covering, |r| r.get() as usize) }
    }
}

#[derive(Default)]
pub struct Placement {
    pub path: Option<OnPath>,
    pub surface: Option<OnSurface>,
    pub row: Option<Row>,
    /// Viewer distance in px for glyphs with depth; `None` leaves the perspective to the layer's camera, or to nothing
    /// at all when the text is orthographic.
    pub perspective: Option<f64>,
}

impl Placement {
    /// Does this move characters away from where the layout put them?
    pub fn moves_glyphs(&self) -> bool {
        self.path.is_some() || self.surface.is_some() || self.row.is_some()
    }
}

/// Glyphs fainter than this are not drawn and do not count toward bounds.
pub const MIN_VISIBLE_OPACITY: f64 = 0.002;

/// A glyph whose clip-space `w` falls below this is behind (or on) the viewer and cannot be drawn.
pub const NEAR_W: f64 = 0.02;

/// Perspective about `(vx, vy)` for a viewer `d` px away: `w = 1 + z / d`.
fn perspective(d: f64, vx: f64, vy: f64) -> Mat4 {
    let mut p = Mat4::IDENTITY;
    p.0[14] = 1.0 / d;
    Mat4::translate(vx, vy, 0.0).mul(&p).mul(&Mat4::translate(-vx, -vy, 0.0))
}

/// Apply the placement to the animated glyphs. `shift` is the tracking each glyph gained from animators.
pub fn finish(layout: &Layout, glyphs: Vec<GlyphState>, shift: &[f32], pl: &Placement) -> Vec<GlyphState> {
    let copies = pl.row.as_ref().map_or(1, |r| r.copies);
    let mut out = Vec::with_capacity(glyphs.len() * copies);
    for k in 0..copies {
        let row_dx = pl.row.as_ref().map_or(0.0, |r| (r.shift + r.period * k as f64) as f32);
        for (g, tracking) in glyphs.iter().zip(shift) {
            let mut g = g.clone();
            let dx = tracking + row_dx;
            // the glyph first slides along its line by dx, then is carried to its place
            slide(&mut g, dx);
            if let Some(path) = &pl.path {
                on_path(layout, &mut g, dx, path);
            } else if let Some(surface) = &pl.surface {
                on_surface(layout, &mut g, dx, surface);
            }
            if let (Some(d), Some(m4)) = (pl.perspective, g.m4) {
                g.m4 = Some(perspective(d, f64::from(layout.anchor[0]), f64::from(layout.anchor[1])).mul(&Mat4(m4)).0);
            }
            out.push(g);
        }
    }
    out
}

fn slide(g: &mut GlyphState, dx: f32) {
    g.m = Aff::translate(dx, 0.0).mul(&g.m);
    g.m4 = g.m4.map(|m| Mat4::translate(f64::from(dx), 0.0, 0.0).mul(&Mat4(m)).0);
}

/// Where the glyph's centre line (centre of its box, `cx`, `cy`) starts out, after sliding by `dx`.
fn rest_centre(layout: &Layout, g: &GlyphState, dx: f32) -> (f32, f32) {
    let lg = &layout.glyphs[g.src];
    (lg.x + lg.w / 2.0 + dx, (lg.top + lg.bottom) / 2.0)
}

/// Arc length along the path of the glyph centre at layout x `cx`, by alignment.
fn arc_position(layout: &Layout, path: &OnPath, cx: f32) -> f64 {
    let len = path.length();
    if path.force_align {
        let stretch = (len - path.last - path.first) / f64::from(layout.width);
        return path.first + f64::from(cx) * stretch;
    }
    let from_edge = f64::from(cx - layout.anchor[0]);
    match path.align {
        Align::Left => path.first + from_edge,
        Align::Right => len - path.last + from_edge,
        Align::Center => (path.first + len - path.last) / 2.0 + from_edge,
    }
}

fn on_path(layout: &Layout, g: &mut GlyphState, dx: f32, path: &OnPath) {
    let (cx, _) = rest_centre(layout, g, dx);
    let Some((p, deg)) = path.at(arc_position(layout, path, cx)) else {
        g.opacity = 0.0;
        return;
    };
    let turn = if path.perpendicular { Aff::rotate(deg) } else { Aff::IDENTITY };
    let place = Aff::translate(layout.anchor[0] + p[0], layout.anchor[1] + p[1]).mul(&turn).mul(&Aff::translate(-cx, -g.y));
    g.m = place.mul(&g.m);
    g.m4 = g.m4.map(|m| place.to_mat4().mul(&Mat4(m)).0);
}

fn on_surface(layout: &Layout, g: &mut GlyphState, dx: f32, surface: &OnSurface) {
    let (cx, cy) = rest_centre(layout, g, dx);
    let (ax, ay) = (f64::from(layout.anchor[0]), f64::from(layout.anchor[1]));
    let r = surface.radius;
    let phi = f64::from(cx - layout.anchor[0]) / r + surface.turn.to_radians();
    let lat = match surface.kind {
        SurfaceKind::Cylinder => 0.0,
        SurfaceKind::Sphere => f64::from(cy - layout.anchor[1]) / r,
    };
    // the surface normal points at the viewer when its z component is negative
    if phi.cos() * lat.cos() <= 0.0 && !surface.back {
        g.opacity = 0.0;
    }
    let (px, py, pz) = match surface.kind {
        SurfaceKind::Cylinder => (ax + r * phi.sin(), f64::from(cy), r * (1.0 - phi.cos())),
        SurfaceKind::Sphere => (ax + r * lat.cos() * phi.sin(), ay + r * lat.sin(), r * (1.0 - lat.cos() * phi.cos())),
    };
    // tip the plane to the latitude about X, then swing it to the longitude about Y (fixed axes, so Y after X). A positive
    // Y rotation brings the right edge toward the viewer (the layer convention), so a plane facing right of centre needs a
    // negative one.
    let facing = Mat4::rot_y(-phi.to_degrees()).mul(&Mat4::rot_x(lat.to_degrees()));
    let place = Mat4::translate(px, py, pz).mul(&facing).mul(&Mat4::translate(-f64::from(cx), -f64::from(cy), 0.0));
    g.m4 = Some(place.mul(&g.m4.map_or_else(|| g.m.to_mat4(), Mat4)).0);
}

/// Where a glyph's box `[l, t, r, b]` lands after its transform: the 4 corners in layout space, or `None` when a
/// corner is behind the viewer.
pub fn corners(g: &GlyphState, rect: [f32; 4]) -> Option<[[f64; 2]; 4]> {
    let pts = [[rect[0], rect[1]], [rect[2], rect[1]], [rect[2], rect[3]], [rect[0], rect[3]]];
    let mut out = [[0.0; 2]; 4];
    for (o, p) in out.iter_mut().zip(pts) {
        let (x, y) = (f64::from(p[0]), f64::from(p[1]));
        *o = match g.m4 {
            Some(m) => {
                let [px, py, _, w] = Mat4(m).apply(x, y, 0.0);
                if w < NEAR_W {
                    return None;
                }
                [px / w, py / w]
            }
            None => {
                let [a, b, c, d, e, f] = g.m.0.map(f64::from);
                [a * x + c * y + e, b * x + d * y + f]
            }
        };
    }
    Some(out)
}

/// Bounding box `[l, t, r, b]` of the placed glyphs in layout space (the boxes of glyphs that show).
pub fn bounds(layout: &Layout, glyphs: &[GlyphState]) -> Option<[f64; 4]> {
    let mut acc: Option<[f64; 4]> = None;
    for g in glyphs.iter().filter(|g| g.opacity > MIN_VISIBLE_OPACITY) {
        let lg = &layout.glyphs[g.src];
        let Some(c) = corners(g, [lg.x, lg.top, lg.x + lg.w, lg.bottom]) else { continue };
        for p in c {
            acc = Some(match acc {
                None => [p[0], p[1], p[0], p[1]],
                Some(a) => [a[0].min(p[0]), a[1].min(p[1]), a[2].max(p[0]), a[3].max(p[1])],
            });
        }
    }
    acc
}
