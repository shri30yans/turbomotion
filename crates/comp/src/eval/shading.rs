//! What a 3D layer's face is to the camera and the lights: which side is hidden, and the plane-to-screen numbers lighting needs.

use super::{mat4::Mat4, CameraState, LitState};
use crate::schema::{Cull, Material};

/// Whether the camera sees the side of a layer that `cull` hides.
///
/// The layer's readable side is its local `-z`, so the camera is on it when it lies against the layer's local
/// `+z` axis.
pub(super) fn culled(world: &Mat4, cam: &CameraState, cull: Cull) -> bool {
    let w = &world.0;
    let normal = [w[2], w[6], w[10]];
    let origin = [w[3], w[7], w[11]];
    let to_camera = [cam.pos[0] - origin[0], cam.pos[1] - origin[1], cam.pos[2] - origin[2]];
    let towards = normal[0] * to_camera[0] + normal[1] * to_camera[1] + normal[2] * to_camera[2];
    match cull {
        Cull::None => false,
        Cull::Front => towards < 0.0,
        Cull::Back => towards > 0.0,
    }
}

pub(super) fn lit_state(world: &Mat4, screen: &Mat4, mat: Material) -> Option<LitState> {
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
