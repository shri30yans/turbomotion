//! A layer faces one way: its `aim`, the orientation along `along`, or the auto-orient of its position's path.

use crate::{anim::prop::VecProp, schema::Common};

/// What decides where the layer faces, when more than one thing does.
pub(super) fn conflict(c: &Common) -> Option<String> {
    let mut deciders = Vec::new();
    if c.aim.is_some() {
        deciders.push("\"aim\"");
    }
    if c.along.as_ref().is_some_and(|a| a.orient) {
        deciders.push("\"along.orient\"");
    }
    if c.pos.as_ref().is_some_and(|p| matches!(p.0, VecProp::Joined { auto_orient: true, .. })) {
        deciders.push("the \"auto_orient\" of \"pos\"");
    }
    (deciders.len() > 1).then(|| format!("{} both decide where the layer faces; use one", deciders.join(" and ")))
}
