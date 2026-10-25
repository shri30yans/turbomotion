//! Compositing options every effect instance has, so no shader re-implements them: the original picture stays visible
//! and the effect's result is laid over it, only inside a mask, with a blend mode and an amount.
//!
//! ```json
//! {"effect": "halftone", "size": 12,
//!  "composite": {"mix": 0.8, "blend": "screen", "mask": {"kind": "luma", "lo": 0.5, "softness": 0.2}}}
//! ```
//!
//! * `mix` 0..1 (animatable, default 1): how much of the laid-over result shows.
//! * `blend` (default `normal`): how the result is laid over the input; any layer blend mode.
//! * `mask` (default `none`, everywhere): where the result shows, as `"none"`, `"alpha"` or `{"kind": ...}`:
//!   `alpha` (where the input has content), `luma` (a brightness band of the input: `lo`, `hi`, `softness`, `invert`),
//!   `edges` (outlines of the input: `strength`, `width` px) or `layer` (another layer's `channel`, `luma` or `alpha`,
//!   aligned on screen like a track matte; `invert` flips it).
//!
//! Without a `composite` the effect's result replaces its input, as before.

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Map, Value};
use turbomotion_core::{effects::Blend, suggest::hint};

use crate::anim::prop::Prop;

/// Which part of another layer's picture is the mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Luma,
    Alpha,
}

#[derive(Debug, Clone)]
pub struct CompositePlan {
    pub mix: Prop,
    pub blend: Blend,
    pub mask: MaskPlan,
}

#[derive(Debug, Clone)]
pub enum MaskPlan {
    Everywhere,
    Alpha,
    Luma { lo: Prop, hi: Prop, softness: Prop, invert: bool },
    Edges { strength: Prop, width: Prop },
    /// `index` is filled in when the composition's layer ids are known.
    Layer { id: String, index: Option<usize>, channel: Channel, invert: bool },
}

/// Defaults, named once: the result shows in full; the luma band covers every brightness with a gentle edge; edges are
/// drawn at full strength a pixel or two wide.
const MIX: f64 = 1.0;
const LUMA_LO: f64 = 0.0;
const LUMA_HI: f64 = 1.0;
const LUMA_SOFTNESS: f64 = 0.05;
const EDGE_STRENGTH: f64 = 1.0;
const EDGE_WIDTH: f64 = 1.5;

fn object<'a>(v: &'a Value, what: &str, allowed: &[&str]) -> Result<&'a Map<String, Value>> {
    let o = v.as_object().ok_or_else(|| anyhow!("{what} is an object"))?;
    for k in o.keys() {
        if !allowed.contains(&k.as_str()) {
            bail!("{what} has no key {k:?}.{} Its keys are: {}", hint(k, allowed), allowed.join(", "));
        }
    }
    Ok(o)
}

fn prop(o: &Map<String, Value>, key: &str, default: f64) -> Result<Prop> {
    o.get(key).map_or(Ok(Prop::constant(default)), |v| Prop::parse(v).map_err(|e| anyhow!("{key}: {e:#}")))
}

fn flag(o: &Map<String, Value>, key: &str) -> Result<bool> {
    o.get(key).map_or(Ok(false), |v| v.as_bool().ok_or_else(|| anyhow!("{key} {v} is not true or false")))
}

/// The kinds of mask, and which keys each takes.
#[derive(Clone, Copy)]
enum MaskKind {
    None,
    Alpha,
    Luma,
    Edges,
    Layer,
}

impl MaskKind {
    fn parse(name: &str) -> Result<MaskKind> {
        Ok(match name {
            "none" => MaskKind::None,
            "alpha" => MaskKind::Alpha,
            "luma" => MaskKind::Luma,
            "edges" => MaskKind::Edges,
            "layer" => MaskKind::Layer,
            other => bail!("mask kind {other:?}: use none, alpha, luma, edges or layer"),
        })
    }

    fn keys(self) -> &'static [&'static str] {
        match self {
            MaskKind::None | MaskKind::Alpha => &["kind"],
            MaskKind::Luma => &["kind", "lo", "hi", "softness", "invert"],
            MaskKind::Edges => &["kind", "strength", "width"],
            MaskKind::Layer => &["kind", "layer", "channel", "invert"],
        }
    }
}

/// A mask is `"none"`, `"alpha"` or an object with a `kind`.
fn parse_mask(v: &Value) -> Result<MaskPlan> {
    let (kind, name) = match v.as_str() {
        Some(word @ ("none" | "alpha")) => (MaskKind::parse(word)?, word),
        Some(other) => bail!("{other:?} is a bare word only for none and alpha; luma, edges and layer take options, written {{\"kind\": {other:?}, ...}}"),
        None => {
            let name = v.get("kind").and_then(Value::as_str).ok_or_else(|| anyhow!("a mask is \"none\", \"alpha\" or an object with \"kind\": none, alpha, luma, edges or layer"))?;
            (MaskKind::parse(name)?, name)
        }
    };
    let empty = Map::new();
    let o = if v.is_object() { object(v, &format!("a {name} mask"), kind.keys())? } else { &empty };
    Ok(match kind {
        MaskKind::None => MaskPlan::Everywhere,
        MaskKind::Alpha => MaskPlan::Alpha,
        MaskKind::Luma => MaskPlan::Luma { lo: prop(o, "lo", LUMA_LO)?, hi: prop(o, "hi", LUMA_HI)?, softness: prop(o, "softness", LUMA_SOFTNESS)?, invert: flag(o, "invert")? },
        MaskKind::Edges => MaskPlan::Edges { strength: prop(o, "strength", EDGE_STRENGTH)?, width: prop(o, "width", EDGE_WIDTH)? },
        MaskKind::Layer => {
            let id = o.get("layer").and_then(Value::as_str).filter(|id| !id.is_empty()).ok_or_else(|| anyhow!("a layer mask needs \"layer\": <layer id>"))?.to_string();
            let channel = match o.get("channel") {
                None => Channel::Luma,
                Some(c) => match c.as_str() {
                    Some("luma") => Channel::Luma,
                    Some("alpha") => Channel::Alpha,
                    _ => bail!("channel {c} is not an option; the channels are \"luma\" and \"alpha\""),
                },
            };
            MaskPlan::Layer { id, index: None, channel, invert: flag(o, "invert")? }
        }
    })
}

/// Parse an effect's `composite` value.
pub fn parse(v: &Value) -> Result<CompositePlan> {
    let o = object(v, "composite", &["mix", "blend", "mask"])?;
    let blend = o.get("blend").map_or(Ok(Blend::Normal), |b| serde_json::from_value(b.clone()).map_err(|e| anyhow!("blend: {e}")))?;
    let mask = o.get("mask").map_or(Ok(MaskPlan::Everywhere), |m| parse_mask(m).map_err(|e| anyhow!("mask: {e:#}")))?;
    Ok(CompositePlan { mix: prop(o, "mix", MIX)?, blend, mask })
}

/// What the documentation says about composite options.
pub fn catalogue() -> Value {
    json!({
        "summary": "Every effect, built-in or plugin, takes an optional `composite`: the original picture stays visible and the effect's result is laid over it, only inside a mask, with a blend mode and an amount. Without it the result replaces the input.",
        "keys": {
            "mix": "0..1, animatable (default 1): how much of the laid-over result shows",
            "blend": "any layer blend mode (default normal)",
            "mask": "none (default) | alpha | {kind: luma, lo, hi, softness, invert} | {kind: edges, strength, width} | {kind: layer, layer: id, channel: luma|alpha, invert}",
        },
        "example": {"effect": "halftone", "size": 12, "composite": {"mix": 0.8, "blend": "screen", "mask": {"kind": "luma", "lo": 0.5, "softness": 0.2}}},
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(v: Value) -> String {
        format!("{:#}", parse(&v).expect_err("rejected"))
    }

    #[test]
    fn a_bad_option_names_what_was_written_and_lists_the_valid_ones() {
        let e = error(json!({"mask": {"kind": "layer", "layer": "m", "channel": "red"}}));
        assert!(e.contains("channel \"red\"") && e.contains("\"luma\"") && e.contains("\"alpha\""), "{e}");
        let e = error(json!({"mask": {"kind": "luma", "invert": "yes"}}));
        assert!(e.contains("invert \"yes\""), "{e}");
        let e = error(json!({"mix": 1, "blur": 2}));
        assert!(e.contains("\"blur\"") && e.contains("mix, blend, mask"), "{e}");
        let e = error(json!({"blend": "multiplyy"}));
        assert!(e.contains("multiplyy") && e.contains("multiply"), "{e}");
        let e = error(json!({"mask": {"kind": "stripes"}}));
        assert!(e.contains("\"stripes\"") && e.contains("luma, edges or layer"), "{e}");
        let e = error(json!({"mask": {"kind": "luma", "strength": 1}}));
        assert!(e.contains("\"strength\"") && e.contains("lo, hi, softness, invert"), "{e}");
    }

    #[test]
    fn the_defaults_are_a_full_mix_with_a_normal_blend_everywhere() {
        let plan = parse(&json!({})).expect("an empty composite");
        assert_eq!(plan.blend, Blend::Normal);
        assert!(matches!(plan.mask, MaskPlan::Everywhere));
    }
}
