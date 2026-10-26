//! Facing along a path: the one place a direction of travel becomes a turn of the layer, for `along.orient` and a
//! position's `auto_orient` alike.

use super::{pose::Pose, Aim};

/// Turns a layer to face the way it travels. A flat layer turns about its parent's z axis by `heading` degrees; a 3D
/// layer's depth axis runs along `dir`, a unit vector.
pub(super) fn face(pose: &mut Pose, aim: &mut Option<Aim>, in_3d: bool, dir: [f64; 3], heading: f64) {
    if in_3d {
        *aim = Some(Aim::Travel(dir));
    } else {
        pose.heading = heading;
    }
}

/// The heading in degrees of a direction of travel in the xy plane.
pub(super) fn heading_of(dir: [f64; 3]) -> f64 {
    dir[1].atan2(dir[0]).to_degrees()
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::path::Path;

    use super::super::{Aim, Evaluator};
    use crate::plan::Plan;

    fn props(layer: serde_json::Value, t: f64) -> super::super::Props {
        let plan = Plan::from_value(json!({"dur": 3, "layers": [layer]}), Path::new(".")).unwrap_or_else(|e| panic!("{e:#}"));
        Evaluator::new(&plan).local_props(plan.root(), &plan.root().layers[0], t, 0).unwrap()
    }

    fn pos(auto_orient: bool) -> serde_json::Value {
        let mut p = json!({"keys": [{"t": 0, "v": [100, 100], "spatial": "linear", "ease": "none"}, {"t": 2, "v": [100, 300], "spatial": "linear", "ease": "none"}]});
        if auto_orient {
            p["auto_orient"] = json!("path");
        }
        p
    }

    #[test]
    fn a_flat_layer_turns_to_the_direction_of_its_path_about_z() {
        let p = props(json!({"type": "solid", "color": "#fff", "size": [50, 50], "pos": pos(true)}), 1.0);
        assert!((p.pose.heading - 90.0).abs() < 1e-9, "{}", p.pose.heading);
        assert!(props(json!({"type": "solid", "color": "#fff", "size": [50, 50], "pos": pos(false)}), 1.0).pose.heading == 0.0, "without auto_orient the layer does not turn");
    }

    #[test]
    fn a_3d_layer_points_its_depth_axis_along_the_path() {
        let p = props(json!({"type": "solid", "color": "#fff", "size": [50, 50], "three_d": true, "pos": {"keys": [{"t": 0, "v": [0, 0, 0], "spatial": "linear", "ease": "none"}, {"t": 2, "v": [0, 0, 100], "spatial": "linear", "ease": "none"}], "auto_orient": "path"}}), 1.0);
        let Some(Aim::Travel(d)) = p.aim else { panic!("a 3D layer on a path faces along it: {:?}", p.aim) };
        assert!((d[2] - 1.0).abs() < 1e-12 && d[0].abs() < 1e-12 && d[1].abs() < 1e-12, "{d:?}");
    }

    #[test]
    fn one_thing_decides_where_a_layer_faces() {
        for (extra, what) in [(json!({"aim": [1, 0, 0], "three_d": true}), "aim"), (json!({"along": {"path": {"points": [[0, 0], [0, 100]]}, "t": 0.5, "orient": true}}), "along.orient")] {
            let mut layer = json!({"type": "solid", "color": "#fff", "size": [50, 50], "pos": pos(true)});
            layer.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            let e = format!("{:#}", Plan::from_value(json!({"dur": 1, "layers": [layer]}), Path::new(".")).err().unwrap_or_else(|| panic!("{what} with auto_orient was accepted")));
            assert!(e.contains(what) && e.contains("auto_orient") && e.contains("use one"), "{e}");
        }
    }
}
