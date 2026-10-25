//! The pose of a layer or a camera: one type, one matrix, one inverse.
//!
//! After Effects keeps two different rotations on a 3D layer, and this module holds both exactly as it does:
//!
//! * **Orientation** is a *quaternion* (shown as X, Y and Z angles). It interpolates along the shortest turn between
//!   keys and cannot wind: a key at 350 degrees after one at 0 goes 10 degrees the other way.
//! * **X, Y and Z Rotation** are three *Euler angles* applied in a fixed order. Each is its own property, winds past
//!   360 degrees (720 is two turns) and interpolates per axis.
//!
//! The world turn of a layer is `Rz(heading) · facing · Orientation · Rotation`, and a camera uses the same product:
//! its Z Rotation is its roll (After Effects has no separate roll).
//!
//! Composition and evidence. Orientation and X, Y and Z Rotation use the SAME Euler composition, `Rx · Ry · Rz`
//! (the Z turn reaches the point first); they differ only in how they interpolate. Sources:
//!
//! * `Rx · Ry · Rz` for Orientation, with the quaternion and its shortest-arc slerp: *measured in After Effects 2026*
//!   (py-aep's goldens, reproduced in the tests below). Only that composition reproduces them.
//! * The same composition for Rotation, nested inside Orientation (`Orientation · Rotation`): taken from py-aep's
//!   resolver, Skottie (`TransformAdapter3D::totalMatrix`: "orientation and axis-wise rotation map onto the same
//!   property; the difference is how they get interpolated") and lottie-web, which agree. Not measured for Rotation.
//! * Rotation tweens are per axis and wind past 360 degrees (Adobe documentation).
//! * Right-handed rotations on a y-down, z-away world, so a positive Z turn is clockwise on screen: measured for
//!   the Z Rotation of a 2D layer, and for X and Y by the camera datum in the tests (Orientation (30, 40, 0) points
//!   a camera along `[0.643, -0.383, 0.663]`).
//! * `heading` (a flat layer's path auto-orient, `atan2(dy, dx)`) turns about the parent's z axis outside everything
//!   else (py-aep). A 3D layer aims its depth axis along the path instead, see `eval::Aim`.
//! * `facing` is outside the Orientation (py-aep: the aim rotation is applied outside both).
//!
//! `tools/ae-export/dump_layer_matrices.jsx` and `tests/golden/ae_rotation.json` let the owner confirm the rest.

use anyhow::{bail, Result};
use serde::Serialize;

use super::{mat4::Mat4, salt, LayerEnv};
use crate::{
    anim::prop::Prop,
    schema::Common,
};

/// `|sin(Y)|` above which X and Z of an Euler read-back are one axis (Y within about 0.0001 degrees of +-90).
const GIMBAL: f64 = 1.0 - 1e-12;

/// How far apart (as a fraction of the segment) the axes of one Orientation key may be and still count as one turn.
const PROGRESS_AGREEMENT: f64 = 1e-9;

/// Cosine of the angle between two rotations above which they are the same one, so slerp is a plain blend.
const SAME_ROTATION: f64 = 1.0 - 1e-9;

/// A unit quaternion `w + xi + yj + zk`: a rotation with no gimbal and no winding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Quat {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quat {
    pub const IDENTITY: Quat = Quat { w: 1.0, x: 0.0, y: 0.0, z: 0.0 };

    /// A turn of `deg` degrees about a unit axis, right-handed.
    pub fn axis_angle(axis: [f64; 3], deg: f64) -> Quat {
        let (s, c) = (deg.to_radians() / 2.0).sin_cos();
        Quat { w: c, x: axis[0] * s, y: axis[1] * s, z: axis[2] * s }
    }

    pub fn mul(&self, o: &Quat) -> Quat {
        Quat {
            w: self.w * o.w - self.x * o.x - self.y * o.y - self.z * o.z,
            x: self.w * o.x + self.x * o.w + self.y * o.z - self.z * o.y,
            y: self.w * o.y - self.x * o.z + self.y * o.w + self.z * o.x,
            z: self.w * o.z + self.x * o.y - self.y * o.x + self.z * o.w,
        }
    }

    pub fn dot(&self, o: &Quat) -> f64 {
        self.w * o.w + self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn normalized(self) -> Quat {
        let n = self.dot(&self).sqrt();
        Quat { w: self.w / n, x: self.x / n, y: self.y / n, z: self.z / n }
    }

    /// An Orientation given as X, Y and Z angles in degrees: the turn `Rx · Ry · Rz`.
    pub fn from_orientation(deg: [f64; 3]) -> Quat {
        Quat::axis_angle([1.0, 0.0, 0.0], deg[0]).mul(&Quat::axis_angle([0.0, 1.0, 0.0], deg[1])).mul(&Quat::axis_angle([0.0, 0.0, 1.0], deg[2]))
    }

    /// The X, Y and Z angles of an Orientation, degrees: Y within -90..90, X and Z within -180..180. At the
    /// gimbal (Y of +-90) X and Z turn about the same axis, and After Effects gives the whole combination to Z
    /// (source: py-aep, measured in After Effects 2026: Orientation (45, 90, 30) reads back as (0, 90, 75)).
    pub fn to_orientation(&self) -> [f64; 3] {
        let m = self.to_mat4().0;
        let y = m[2].clamp(-1.0, 1.0).asin();
        let (x, z) = if m[2].abs() > GIMBAL { (0.0, m[4].atan2(m[5])) } else { ((-m[6]).atan2(m[10]), (-m[1]).atan2(m[0])) };
        [x.to_degrees(), y.to_degrees(), z.to_degrees()]
    }

    /// The rotation matrix of this quaternion.
    pub fn to_mat4(&self) -> Mat4 {
        let Quat { w, x, y, z } = self.normalized();
        Mat4([
            1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - w * z), 2.0 * (x * z + w * y), 0.0,
            2.0 * (x * y + w * z), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - w * x), 0.0,
            2.0 * (x * z - w * y), 2.0 * (y * z + w * x), 1.0 - 2.0 * (x * x + y * y), 0.0,
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    /// Spherical interpolation along the *shortest* turn. `u` outside 0..1 (an overshooting ease) keeps turning.
    pub fn slerp(&self, to: &Quat, u: f64) -> Quat {
        let mut b = *to;
        let mut d = self.dot(&b);
        if d < 0.0 {
            // q and -q are the same rotation; take the one on this side
            b = Quat { w: -b.w, x: -b.x, y: -b.y, z: -b.z };
            d = -d;
        }
        let d = d.min(1.0);
        let (k0, k1) = if d > SAME_ROTATION {
            (1.0 - u, u)
        } else {
            let theta = d.acos();
            let s = theta.sin();
            (((1.0 - u) * theta).sin() / s, (u * theta).sin() / s)
        };
        Quat { w: k0 * self.w + k1 * b.w, x: k0 * self.x + k1 * b.x, y: k0 * self.y + k1 * b.y, z: k0 * self.z + k1 * b.z }.normalized()
    }
}

/// The one Euler composition, for Rotation (and what an Orientation's quaternion is built from): `Rx · Ry · Rz`,
/// the Z turn first. Angles may exceed 360.
pub fn euler_matrix(deg: [f64; 3]) -> Mat4 {
    Mat4::rot_x(deg[0]).mul(&Mat4::rot_y(deg[1])).mul(&Mat4::rot_z(deg[2]))
}

/// Everything that turns a layer or a camera.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Pose {
    /// A flat layer's path auto-orient: degrees about the parent's z axis, clockwise on screen, outside everything else.
    pub heading: f64,
    /// After Effects' Orientation.
    pub orientation: Quat,
    /// After Effects' X, Y and Z Rotation, degrees; a camera's `[2]` is its roll.
    pub rotation: [f64; 3],
}

impl Pose {
    pub const IDENTITY: Pose = Pose { heading: 0.0, orientation: Quat::IDENTITY, rotation: [0.0; 3] };

    /// A pose from the angles an author writes: Orientation `[x, y, z]` and Rotation `[x, y, z]`.
    pub fn from_angles(orientation: [f64; 3], rotation: [f64; 3]) -> Pose {
        Pose { heading: 0.0, orientation: Quat::from_orientation(orientation), rotation }
    }

    /// THE matrix: the turn from the layer's own space to its parent's, `Rz(heading) · facing · O · R`.
    ///
    /// `facing` is the turn that points the layer at its aim (or, for a camera, at its point of interest), already a
    /// pure rotation; pass `Mat4::IDENTITY` for none.
    pub fn matrix(&self, facing: &Mat4) -> Mat4 {
        Mat4::rot_z(self.heading).mul(facing).mul(&self.orientation.to_mat4()).mul(&euler_matrix(self.rotation))
    }

    /// The turn that undoes `matrix`: its rotation block transposed, never a second formula.
    pub fn inverse(&self, facing: &Mat4) -> Mat4 {
        self.matrix(facing).transpose_rotation()
    }
}

/// The Orientation at layer time `lt`, from its three axis properties.
///
/// Plain keys (no expression, no loop) are interpolated as a rotation (source: py-aep, After Effects 2026 measured
/// goldens in the tests): the keyframes of the three axes are merged into one list, each is turned into a
/// quaternion, and the values between are a slerp along the shortest arc whose progress is that of the axes (which
/// must agree), so an ease on the tween shapes the turn. An expression acts on a number, not on a rotation, so a
/// property with an expression or a loop is evaluated per axis, as After Effects does, and read as Euler angles.
pub fn orientation_at<'a>(axes: &[Prop; 3], lt: f64, env: &dyn Fn(u32) -> LayerEnv<'a>) -> Result<Quat> {
    if axes.iter().all(Prop::is_plain) {
        return slerp_keys(axes, lt);
    }
    let mut deg = [0.0; 3];
    for (i, p) in axes.iter().enumerate() {
        deg[i] = p.at(lt, &env(salt::axis(salt::ORIENTATION, i)))?;
    }
    Ok(Quat::from_orientation(deg))
}

/// Plain keyed axes at `t`: the keys of all three merged, a slerp between the two that surround `t`.
///
/// An Orientation key holds all three angles and one ease, so every axis that moves across a segment must be at the
/// same progress; axes eased differently cannot be one rotation, and that is an error rather than a guess. (Keys at
/// different times are fine: with the times merged, equal eases give every axis the same progress.)
fn slerp_keys(axes: &[Prop; 3], t: f64) -> Result<Quat> {
    let mut times: Vec<f64> = axes.iter().flat_map(|p| p.keys().iter().map(|k| k.t)).collect();
    times.sort_by(f64::total_cmp);
    times.dedup();
    let at = |t: f64| [axes[0].base(t), axes[1].base(t), axes[2].base(t)];
    let (first, last) = (times[0], times[times.len() - 1]);
    if t <= first || t >= last {
        return Ok(Quat::from_orientation(at(t)));
    }
    let i = times.iter().rposition(|k| *k <= t).expect("t is after the first key");
    let (a, b, now) = (at(times[i]), at(times[i + 1]), at(t));
    let mut progress = None;
    for k in (0..3).filter(|k| a[*k] != b[*k]) {
        let u = (now[k] - a[k]) / (b[k] - a[k]);
        match progress {
            None => progress = Some(u),
            Some(first) if (first - u).abs() > PROGRESS_AGREEMENT => {
                bail!("the axes of an orientation are eased differently between t={} and t={}; key all three together as one vector", times[i], times[i + 1])
            }
            Some(_) => {}
        }
    }
    let from = Quat::from_orientation(a);
    Ok(match progress {
        None => from,
        Some(u) => from.slerp(&Quat::from_orientation(b), u),
    })
}

/// The rotation and orientation of a layer or camera at layer time `lt`: the one evaluation both share.
///
/// `heading` is the path auto-orient turn, added by the caller that knows the path.
pub fn pose_at<'a>(cm: &Common, lt: f64, env: &dyn Fn(u32) -> LayerEnv<'a>) -> Result<Pose> {
    let mut pose = Pose::IDENTITY;
    if let Some(r) = &cm.rotation {
        for (i, p) in r.0.iter().enumerate() {
            pose.rotation[i] = p.at(lt, &env(salt::axis(salt::ROTATION, i)))?;
        }
    }
    if let Some(o) = &cm.orientation {
        pose.orientation = orientation_at(&o.0, lt, env)?;
    }
    Ok(pose)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn axes(v: Value) -> [Prop; 3] {
        serde_json::from_value::<crate::schema::Orient>(v).expect("an orientation").0
    }

    fn at(a: &[Prop; 3], t: f64) -> Quat {
        slerp_keys(a, t).expect("the axes agree")
    }

    fn close(a: &Mat4, b: &Mat4) {
        for (x, y) in a.0.iter().zip(b.0) {
            assert!((x - y).abs() < 1e-9, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn orientation_30_40_0_points_a_camera_along_the_measured_direction() {
        // measured: After Effects 2026, a camera with Orientation (30, 40, 0) looks along [0.643, -0.383, 0.663]
        let d = Quat::from_orientation([30.0, 40.0, 0.0]).to_mat4().apply(0.0, 0.0, 1.0);
        assert!((d[0] - 0.643).abs() < 1e-3 && (d[1] + 0.383).abs() < 1e-3 && (d[2] - 0.663).abs() < 1e-3, "{d:?}");
    }

    #[test]
    fn the_quaternion_is_the_euler_product_rx_ry_rz() {
        // the quaternion is Rx · Ry · Rz (Z placement measured by the goldens)
        for (x, y, z) in [(30.0, 40.0, 0.0), (10.0, -20.0, 70.0), (-170.0, 85.0, 200.0), (0.0, 0.0, 45.0), (90.0, 0.0, 0.0)] {
            close(&Quat::from_orientation([x, y, z]).to_mat4(), &Mat4::rot_x(x).mul(&Mat4::rot_y(y)).mul(&Mat4::rot_z(z)));
        }
    }

    // The goldens below are measured in After Effects 2026 and quoted from py-aep (forticheprod/py-aep, MIT), which
    // records them from real projects.

    #[test]
    fn golden_orientation_keys_blend_as_a_rotation_in_after_effects_2026() {
        // AE 2026 measured (via py-aep): [0,0,0] -> [45,90,30] at progress 0.5 reads [22.083206323481, 42.8193869722279,
        // 22.083206323481]; a per-axis blend would read [22.5, 45, 15]
        let a = axes(json!({"from": [0, 0, 0], "to": [45, 90, 30], "dur": 1, "ease": "none"}));
        let got = at(&a, 0.5).to_orientation();
        for (g, want) in got.iter().zip([22.083_206_323_481, 42.819_386_972_227_9, 22.083_206_323_481]) {
            assert!((g - want).abs() < 1e-9, "{got:?}");
        }
    }

    #[test]
    fn golden_orientation_takes_the_short_way_round_in_after_effects_2026() {
        // AE 2026 measured (via py-aep): [0,0,10] -> [0,0,350] at progress 0.5 reads Z = 0 (or 360)
        let a = axes(json!({"from": [0, 0, 10], "to": [0, 0, 350], "dur": 1, "ease": "none"}));
        let z = at(&a, 0.5).to_orientation()[2];
        assert!(z.abs() < 1e-9, "{z}");
    }

    #[test]
    fn golden_orientation_at_the_gimbal_reads_back_on_z_in_after_effects_2026() {
        // AE 2026 measured (via py-aep, by reparenting): Orientation [45,90,30] reads back as [0,90,75]
        let got = Quat::from_orientation([45.0, 90.0, 30.0]).to_orientation();
        for (g, want) in got.iter().zip([0.0, 90.0, 75.0]) {
            assert!((g - want).abs() < 1e-9, "{got:?}");
        }
    }

    #[test]
    fn the_orientation_angles_come_back_out_of_the_quaternion() {
        // derived: round trip inside Y -90..90, X and Z -180..180
        for deg in [[30.0, 40.0, 0.0], [-120.0, 60.0, 150.0], [0.0, 0.0, -45.0]] {
            let back = Quat::from_orientation(deg).to_orientation();
            for (a, b) in deg.iter().zip(back) {
                assert!((a - b).abs() < 1e-9, "{deg:?} -> {back:?}");
            }
        }
        // a Z of 200 is the same turn as -160, which is what After Effects shows
        let back = Quat::from_orientation([0.0, 0.0, 200.0]).to_orientation();
        assert!((back[2] + 160.0).abs() < 1e-9, "{back:?}");
    }

    #[test]
    fn a_rotation_uses_the_same_composition_as_an_orientation_and_winds() {
        // source: py-aep, Skottie and lottie-web compose both as Rx · Ry · Rz; not measured in AE for Rotation
        for deg in [[30.0, 40.0, 50.0], [-170.0, 85.0, 200.0]] {
            close(&euler_matrix(deg), &Quat::from_orientation(deg).to_mat4());
        }
        // measured (the Orientation datum, same composition): (30, 40, 0) turns the view axis to [0.643, -0.383, 0.663]
        let d = euler_matrix([30.0, 40.0, 0.0]).apply(0.0, 0.0, 1.0);
        assert!((d[0] - 0.643).abs() < 1e-3 && (d[1] + 0.383).abs() < 1e-3 && (d[2] - 0.663).abs() < 1e-3, "{d:?}");
        // derived: 720 degrees is two full turns, the same matrix as none
        close(&euler_matrix([0.0, 0.0, 720.0]), &Mat4::IDENTITY);
        // the pose nests Orientation outside Rotation (source: lottie-web)
        let pose = Pose::from_angles([10.0, 20.0, 30.0], [40.0, 50.0, 60.0]);
        close(&pose.matrix(&Mat4::IDENTITY), &euler_matrix([10.0, 20.0, 30.0]).mul(&euler_matrix([40.0, 50.0, 60.0])));
    }

    #[test]
    fn the_inverse_is_the_transpose_of_the_matrix_and_undoes_it() {
        // derived
        for (o, r, heading) in [([30.0, 40.0, 12.0], [5.0, -20.0, 70.0], 0.0), ([0.0, 0.0, 45.0], [0.0; 3], 30.0), ([-100.0, 80.0, 10.0], [400.0, 130.0, -250.0], -20.0)] {
            let mut pose = Pose::from_angles(o, r);
            pose.heading = heading;
            let facing = Mat4::face([1.0, -2.0, 3.0]);
            let m = pose.matrix(&facing);
            close(&pose.inverse(&facing), &m.transpose_rotation());
            close(&m.mul(&pose.inverse(&facing)), &Mat4::IDENTITY);
        }
    }

    #[test]
    fn orientation_keys_take_the_shortest_turn() {
        // source: the golden above, AE 2026 measured: shortest path
        let a = axes(json!([0, 0, {"keys": [[0, 0, "none"], [1, 350, "none"]]}]));
        let mid = at(&a, 0.5).to_orientation();
        assert!((mid[2] + 5.0).abs() < 1e-6, "halfway to 350 is -5, not 175: {mid:?}");
        // per-axis Euler would pass through 175
        assert!((a[2].base(0.5) - 175.0).abs() < 1e-9);
        // a turn under half a revolution is the plain linear one
        let b = axes(json!([0, 0, {"from": 0, "to": 90, "dur": 1, "ease": "none"}]));
        assert!((at(&b, 0.5).to_orientation()[2] - 45.0).abs() < 1e-6);
    }

    #[test]
    fn an_orientation_tween_of_a_triple_goes_by_one_slerp() {
        // source: the golden above: from (0,0,0) to (90,90,0): the midpoint is the half-angle rotation about the common axis, not (45,45,0)
        let a = axes(json!({"from": [0, 0, 0], "to": [90, 90, 0], "dur": 1, "ease": "none"}));
        let mid = at(&a, 0.5);
        let want = Quat::from_orientation([0.0; 3]).slerp(&Quat::from_orientation([90.0, 90.0, 0.0]), 0.5);
        close(&mid.to_mat4(), &want.to_mat4());
        let euler = Quat::from_orientation([45.0, 45.0, 0.0]);
        assert!((mid.dot(&euler).abs() - 1.0).abs() > 1e-3, "per-axis Euler would land elsewhere");
        // the ends are the keyed angles
        close(&at(&a, 1.0).to_mat4(), &Quat::from_orientation([90.0, 90.0, 0.0]).to_mat4());
        close(&at(&a, 0.0).to_mat4(), &Mat4::IDENTITY);
        // keys of whole vectors work the same
        let k = axes(json!({"keys": [[0, [0, 0, 0], "none"], [1, [90, 90, 0], "none"]]}));
        close(&at(&k, 0.5).to_mat4(), &mid.to_mat4());
    }

    #[test]
    fn an_orientation_ease_shapes_the_turn_and_all_three_forms_still_parse() {
        let a = axes(json!({"from": [0, 0, 0], "to": [0, 0, 80], "dur": 1, "ease": "power2.in"}));
        let u = crate::anim::ease::Ease::parse("power2.in").unwrap().at(0.5);
        assert!((at(&a, 0.5).to_orientation()[2] - 80.0 * u).abs() < 1e-6);
        // an expression or a loop on one axis makes the whole Orientation per-axis
        assert!(!axes(json!([10, 20, "value + 5"])).iter().all(Prop::is_plain));
        assert!(!axes(json!([0, 0, {"keys": [[0, 0], [1, 90]], "loop": "cycle"}])).iter().all(Prop::is_plain));
        // the constant triple is the plain Euler product
        let c = axes(json!([10, 20, 30]));
        close(&at(&c, 5.0).to_mat4(), &Quat::from_orientation([10.0, 20.0, 30.0]).to_mat4());
    }

    #[test]
    fn orientation_axes_keyed_differently_are_an_error() {
        // the same keys with different eases: at 0.5 s the axes are at different progress, which no one rotation can say
        let a = axes(json!([{"keys": [[0, 0, "none"], [1, 40, "none"]]}, {"keys": [[0, 0, "none"], [1, 40, "power2.in"]]}, 0]));
        let e = format!("{:#}", slerp_keys(&a, 0.5).unwrap_err());
        assert!(e.contains("eased differently"), "{e}");
        // an axis that does not move imposes nothing
        assert!(slerp_keys(&axes(json!([{"keys": [[0, 0, "none"], [1, 40, "none"]]}, 7, 0])), 0.5).is_ok());
    }

    #[test]
    fn rotation_keys_interpolate_per_axis_and_wind_past_a_turn() {
        // source: Adobe documentation: X, Y and Z Rotation are Euler angles, per axis, and may exceed 360
        let r: crate::schema::Rot = serde_json::from_value(json!([0, 0, {"from": 0, "to": 720, "dur": 1, "ease": "none"}])).unwrap();
        assert!((r.0[2].base(0.25) - 180.0).abs() < 1e-9 && (r.0[2].base(0.75) - 540.0).abs() < 1e-9);
        close(&euler_matrix([0.0, 0.0, r.0[2].base(0.5)]), &Mat4::rot_z(360.0));
        // a vector tween of a rotation interpolates each axis on its own
        let v: crate::schema::Rot = serde_json::from_value(json!({"from": [0, 0, 0], "to": [180, 90, 360], "dur": 1, "ease": "none"})).unwrap();
        assert!((v.0[0].base(0.5) - 90.0).abs() < 1e-9 && (v.0[1].base(0.5) - 45.0).abs() < 1e-9 && (v.0[2].base(0.5) - 180.0).abs() < 1e-9);
        // a lone number is the Z Rotation
        let z: crate::schema::Rot = serde_json::from_value(json!(33)).unwrap();
        assert!(z.0[0].base(0.0) == 0.0 && z.0[2].base(0.0) == 33.0);
        // a scalar tween object is still the Z Rotation
        let zt: crate::schema::Rot = serde_json::from_value(json!({"from": 0, "to": 90, "dur": 1, "ease": "none"})).unwrap();
        assert!((zt.0[2].base(0.5) - 45.0).abs() < 1e-9);
    }

    #[test]
    fn slerp_ends_and_extrapolation() {
        let (a, b) = (Quat::from_orientation([0.0; 3]), Quat::from_orientation([0.0, 0.0, 90.0]));
        close(&a.slerp(&b, 0.0).to_mat4(), &a.to_mat4());
        close(&a.slerp(&b, 1.0).to_mat4(), &b.to_mat4());
        // an overshooting ease keeps turning
        close(&a.slerp(&b, 1.5).to_mat4(), &Mat4::rot_z(135.0));
        // identical keys do not divide by zero
        close(&a.slerp(&a, 0.3).to_mat4(), &Mat4::IDENTITY);
    }
}
