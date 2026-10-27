//! The two choices every blur shares besides its shape: what lies beyond the picture's edge, and which way it blurs.

use serde::Deserialize;
use skia_safe::TileMode;

/// What lies beyond the picture's edge.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Edge {
    /// Nothing: the picture fades into transparency at its edge, as light that leaves the layer.
    #[default]
    Transparent,
    /// The picture's edge pixels, repeated outward.
    Repeat,
}

impl Edge {
    pub const NAMES: &'static [&'static str] = &["transparent", "repeat"];

    pub fn tile(self) -> TileMode {
        match self {
            Self::Transparent => TileMode::Decal,
            Self::Repeat => TileMode::Clamp,
        }
    }
}

/// Which way a blur spreads.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Axes {
    #[default]
    Both,
    Horizontal,
    Vertical,
}

impl Axes {
    pub const NAMES: &'static [&'static str] = &["both", "horizontal", "vertical"];

    /// `amount` along the axes this blurs, 0 along the other.
    pub fn along(self, amount: f64) -> [f64; 2] {
        match self {
            Self::Both => [amount; 2],
            Self::Horizontal => [amount, 0.0],
            Self::Vertical => [0.0, amount],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_are_the_ones_the_parser_reads_in_the_order_of_the_variants() {
        let edges: Vec<Edge> = Edge::NAMES.iter().map(|n| serde_json::from_value(serde_json::json!(n)).unwrap()).collect();
        assert_eq!(edges, [Edge::Transparent, Edge::Repeat]);
        let axes: Vec<Axes> = Axes::NAMES.iter().map(|n| serde_json::from_value(serde_json::json!(n)).unwrap()).collect();
        assert_eq!(axes, [Axes::Both, Axes::Horizontal, Axes::Vertical]);
    }

    #[test]
    fn an_axis_left_out_has_no_blur() {
        assert_eq!([Axes::Both, Axes::Horizontal, Axes::Vertical].map(|a| a.along(3.0)), [[3.0, 3.0], [3.0, 0.0], [0.0, 3.0]]);
    }
}
