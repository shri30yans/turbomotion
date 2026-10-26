//! Where a layer's effects and masks run: on the layer's own pixels or on the picture already placed in the frame.

use serde::Deserialize;

/// The composition key `"effect_space"`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EffectSpace {
    /// Every layer is painted at the size of the frame with its transform applied, then its effects and masks run on
    /// that picture: a 10 px blur is 10 screen pixels at any scale, and nothing outside the frame reaches an effect.
    #[default]
    Screen,
    /// After Effects' order: the layer's own pixels (grown to make room for what its effects spread), then its masks and
    /// effects, then the layer's transform. A blur scales and turns with the layer, and content outside the frame
    /// still feeds the effect.
    Layer,
}

impl EffectSpace {
    pub const NAMES: [&'static str; 2] = ["screen", "layer"];

    pub fn name(self) -> &'static str {
        Self::NAMES[usize::from(self == Self::Layer)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_are_the_ones_the_parser_reads() {
        for n in EffectSpace::NAMES {
            let v: EffectSpace = serde_json::from_value(serde_json::json!(n)).unwrap();
            assert_eq!(v.name(), n);
        }
        assert_eq!(EffectSpace::default(), EffectSpace::Screen);
    }
}
