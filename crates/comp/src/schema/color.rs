//! The composition key `"color"`: what the numbers of a composition's pictures mean and what its frames leave in.
//!
//! ```json
//! {"color": {"linear_light": true, "working_profile": "srgb", "space": "srgb", "depth": "half"}}
//! ```
//!
//! Every field is optional and the default (display-encoded sRGB in and out) changes nothing. A nested composition takes
//! the root's values field by field unless it says its own; `space` is the finished frame's, so only the root has it.

use anyhow::{bail, Result};
use serde::Deserialize;
use turbomotion_core::color::{Pipeline, Primaries, Profile, Transfer};

/// The pictures' numeric depth. Every picture is half-float, so there is one.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Depth {
    #[default]
    Half,
}

impl Depth {
    pub const NAMES: [&'static str; 1] = ["half"];
}

/// The colours the three channels span.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Gamut {
    Srgb,
    DisplayP3,
}

impl Gamut {
    pub const NAMES: [&'static str; 2] = ["srgb", "display_p3"];

    fn primaries(self) -> Primaries {
        match self {
            Self::Srgb => Primaries::Srgb,
            Self::DisplayP3 => Primaries::DisplayP3,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ColorBlock {
    /// Numeric depth of every picture; `"half"` (half-float) is the only one.
    pub depth: Depth,
    /// Root only. The gamut the finished frame is encoded in, with the sRGB curve: `"srgb"` (default) or `"display_p3"`.
    pub space: Option<Gamut>,
    /// Blend, blur, resample and add in linear light instead of on display-encoded numbers (default false).
    pub linear_light: Option<bool>,
    /// The gamut pictures are kept in: `"srgb"` (default) or `"display_p3"` (images and video in it keep their colours).
    pub working_profile: Option<Gamut>,
}

/// The pipeline of composition `name` (the root is `""`) given its own `color` block and the root's.
pub fn pipeline(name: &str, own: Option<&ColorBlock>, root: Option<&ColorBlock>) -> Result<Pipeline> {
    let (own, root) = (own.cloned().unwrap_or_default(), root.cloned().unwrap_or_default());
    if !name.is_empty() && own.space.is_some() {
        bail!("composition {name:?}: color.space is the colour space the finished frame leaves in, so only the root composition has it");
    }
    let gamut = own.working_profile.or(root.working_profile).unwrap_or(Gamut::Srgb);
    let transfer = if own.linear_light.or(root.linear_light).unwrap_or(false) { Transfer::Linear } else { Transfer::Srgb };
    let output = root.space.unwrap_or(Gamut::Srgb);
    Ok(Pipeline { working: Profile { primaries: gamut.primaries(), transfer }, output: Profile { primaries: output.primaries(), transfer: Transfer::Srgb } })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(v: serde_json::Value) -> ColorBlock {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn the_names_are_the_ones_the_parser_reads() {
        for n in Gamut::NAMES {
            serde_json::from_value::<Gamut>(serde_json::json!(n)).unwrap();
        }
        for n in Depth::NAMES {
            serde_json::from_value::<Depth>(serde_json::json!(n)).unwrap();
        }
    }

    #[test]
    fn leaving_the_block_out_is_the_unmanaged_default() {
        assert_eq!(pipeline("", None, None).unwrap(), Pipeline::default());
        assert!(!pipeline("", Some(&ColorBlock::default()), None).unwrap().is_managed());
    }

    #[test]
    fn a_nested_composition_takes_the_roots_values_field_by_field() {
        let root = block(serde_json::json!({"linear_light": true, "working_profile": "display_p3"}));
        let inherits = pipeline("a", None, Some(&root)).unwrap();
        assert_eq!(inherits.working, Profile { primaries: Primaries::DisplayP3, transfer: Transfer::Linear });
        let overrides = pipeline("b", Some(&block(serde_json::json!({"linear_light": false}))), Some(&root)).unwrap();
        assert_eq!(overrides.working, Profile { primaries: Primaries::DisplayP3, transfer: Transfer::Srgb });
    }

    #[test]
    fn the_output_space_is_the_roots_and_a_nested_composition_cannot_set_one() {
        let root = block(serde_json::json!({"space": "display_p3"}));
        assert_eq!(pipeline("a", None, Some(&root)).unwrap().output.primaries, Primaries::DisplayP3);
        let e = pipeline("a", Some(&root), Some(&root)).unwrap_err().to_string();
        assert!(e.contains("\"a\"") && e.contains("only the root"), "{e}");
    }
}
