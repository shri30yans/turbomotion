//! The `box` key of a text layer: a filled rectangle fitted to the measured characters (a text bar, a pill, a chip, a
//! lower third). The measuring and the geometry are in `text::bounds` and `text::boxfit`; this is only what can be said.

use anyhow::{ensure, Result};
use serde::Deserialize;

use super::{
    text::Align,
    wrap::{Col, P},
};
use crate::anim::prop::{Prop, Value};

/// What a box is fitted to. Only the text today; a field of its own so another fit can be added without changing the key.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BoxFit {
    /// The measured bounds of the characters that are showing: a bar that is typed grows with the characters.
    Text,
}

/// A rectangle drawn behind the glyphs, inside the layer: it moves, scales, rotates and takes effects with the text.
///
/// ```json
/// "box": {"fit": "text", "padding": [28, 14], "radius": 12, "fill": "#e8402f", "min_width": 240}
/// ```
///
/// The box is as wide as the characters showing plus the padding on both sides, and as tall as the lines it covers
/// (ascent to descent of each line) plus the padding above and below. With no characters showing there is no box.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TextBox {
    /// What the box is fitted to: `"text"`.
    pub fit: BoxFit,
    /// `[x, y]`: pixels between the characters and the box's sides (`x`) and top and bottom (`y`). Each is a property
    /// and may animate; neither may be negative.
    #[serde(default = "no_padding")]
    pub padding: [P; 2],
    /// Corner radius in pixels, held to half the box's shorter side (a radius that large makes a pill).
    #[serde(default)]
    pub radius: f64,
    /// The box's colour; a property.
    pub fill: Col,
    /// The box is never narrower than this many pixels (padding included). When the text is narrower, `align` says
    /// which side stays put.
    #[serde(default)]
    pub min_width: f64,
    /// Where the text sits in a box wider than it: `left` keeps the left edge and grows to the right, `right` the
    /// reverse, `center` grows both ways. The text layer's own `align` when left out.
    #[serde(default)]
    pub align: Option<Align>,
    /// One box for every line that is showing, each as wide as its own line, instead of one box round the block.
    #[serde(default)]
    pub per_line: bool,
}

fn no_padding() -> [P; 2] {
    [P(Prop::constant(0.0)), P(Prop::constant(0.0))]
}

impl TextBox {
    /// The rules the box's numbers must keep, checked once when the composition is planned. What an expression or an
    /// ease overshoot does to a padding can only be known at a time, so evaluation checks it again.
    pub fn validate(&self) -> Result<()> {
        ensure!(self.radius.is_finite() && self.radius >= 0.0, "box radius is 0 or more pixels, not {}", self.radius);
        ensure!(self.min_width.is_finite() && self.min_width >= 0.0, "box min_width is 0 or more pixels, not {}", self.min_width);
        for (axis, p) in ["x", "y"].into_iter().zip(&self.padding).filter(|(_, p)| p.0.is_plain()) {
            for key in p.0.keys() {
                if let Value::F(v) = key.v {
                    ensure!(v.is_finite() && v >= 0.0, "box padding {axis} is 0 or more pixels, not {v}");
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::de::from_value_at;
    use serde_json::json;

    fn parse(v: serde_json::Value) -> Result<TextBox> {
        from_value_at(v)
    }

    #[test]
    fn a_box_with_a_fit_and_a_fill_has_the_defaults_the_docs_say() {
        let b = parse(json!({"fit": "text", "fill": "#fff"})).unwrap();
        assert_eq!((b.radius, b.min_width, b.per_line, b.align), (0.0, 0.0, false, None));
        assert!(b.padding.iter().all(|p| p.0.is_constant() && p.0.base(0.0) == 0.0));
        b.validate().unwrap();
    }

    #[test]
    fn a_misspelt_key_is_named_with_the_nearest_valid_one_and_a_wrong_fit_with_what_it_can_be() {
        let e = parse(json!({"fit": "text", "fill": "#fff", "paddin": [1, 1]})).unwrap_err().to_string();
        assert!(e.contains("unknown field `paddin`") && e.contains("Did you mean \"padding\"?"), "{e}");
        let e = parse(json!({"fit": "layer", "fill": "#fff"})).unwrap_err().to_string();
        assert!(e.starts_with("fit: unknown variant `layer`") && e.contains("expected `text`"), "{e}");
        let e = parse(json!({"fit": "text"})).unwrap_err().to_string();
        assert!(e.contains("missing field `fill`"), "{e}");
        let e = parse(json!({"fill": "#fff"})).unwrap_err().to_string();
        assert!(e.contains("missing field `fit`"), "{e}");
        let e = parse(json!({"fit": "text", "fill": "#fff", "padding": 12})).unwrap_err().to_string();
        assert!(e.contains("padding") && e.contains("array of length 2"), "{e}");
    }

    #[test]
    fn negative_or_non_finite_numbers_are_refused_with_what_to_change() {
        let bad = |extra: serde_json::Value| {
            let mut v = json!({"fit": "text", "fill": "#fff"});
            v.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            parse(v).unwrap().validate().unwrap_err().to_string()
        };
        assert_eq!(bad(json!({"radius": -4})), "box radius is 0 or more pixels, not -4");
        assert_eq!(bad(json!({"min_width": -1})), "box min_width is 0 or more pixels, not -1");
        assert_eq!(bad(json!({"padding": [-2, 4]})), "box padding x is 0 or more pixels, not -2");
        assert_eq!(bad(json!({"padding": [2, [[0, 4], [1, -6]]]})), "box padding y is 0 or more pixels, not -6");
        assert!(parse(json!({"fit": "text", "fill": "#fff", "padding": [[[0, 0], [1, 30]], 8]})).unwrap().validate().is_ok(), "a keyed padding is checked at every key");
    }
}
