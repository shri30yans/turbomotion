//! The descriptor of a platform effect, written from its registry entry.

use turbomotion_core::effects;

use super::{Descriptor, Kind, Param, PassInfo, Ty, PLATFORM_ORIGIN};

/// A colour's alpha when its default is written as `r, g, b`.
const OPAQUE: f64 = 1.0;

/// A platform effect's descriptor, written from its registry entry: parameters, defaults, ranges and docs are what the
/// effect itself is built from.
pub(super) fn descriptor(e: &effects::Entry) -> Descriptor {
    let params = e
        .params
        .iter()
        .map(|p| {
            let (ty, default, options) = match p.kind {
                effects::ParamKind::Float { default } => (Ty::Float, vec![default], vec![]),
                effects::ParamKind::Color { default } => (Ty::Color, default.map_or(vec![], |[r, g, b]| vec![f64::from(r), f64::from(g), f64::from(b), OPAQUE]), vec![]),
                // a choice's default is the index of the option, as a plugin's is
                effects::ParamKind::Choice { default, options } => (Ty::Choice, vec![default as f64], options.iter().map(|o| o.to_string()).collect()),
            };
            Param { name: p.name.to_string(), ty, optional: default.is_empty(), default, min: p.min, max: p.max, options, doc: p.doc.to_string() }
        })
        .collect();
    Descriptor { name: e.effect.to_string(), kind: Kind::Effect, origin: PLATFORM_ORIGIN.to_string(), summary: e.summary.to_string(), params, inputs: vec![], passes: vec![PassInfo { name: "main".into(), scale: 1.0, doc: String::new(), kernel: String::new() }], history: 0, example: (e.example)(), bounds: (if effects::grows_bounds(e.effect) { "grows" } else { "none" }).to_string(), space: e.encoding.name().to_string() }
}
