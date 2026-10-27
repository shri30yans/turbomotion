//! The `taper` key of a shape item: the item's stroke gets thinner toward either end of the outline.
//!
//! ```json
//! {"d": "M 0 0 C 200 0 200 200 400 200", "stroke": {"color": "#fff", "width": 24, "cap": "round"},
//!  "taper": {"start": {"length": 120, "width": 0}, "end": {"length": {"fraction": 0.2}, "width": {"fraction": 0.25}}, "ease": "power2.out"}}
//! ```
//!
//! The outline is walked from its first point to its last. Each end names how long its taper is (`length`) and how wide
//! the stroke is at the very tip (`width`); the stroke is its full `width` everywhere between the two tapers. How the
//! stroke fills out from a tip is `ease`, the same at both tips. The width is measured across the stroke, so a tip of 0 is a
//! point. Building the outline is `shape::stroke`; this is only what can be written.

use anyhow::{ensure, Result};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::wrap::{Easing, P};
use crate::anim::prop::Value as Key;

/// A length or a width, in pixels or as a fraction of something.
#[derive(Debug, Clone)]
pub enum Extent {
    /// Pixels: a number, an expression, keys or a tween, like any property.
    Px(P),
    /// `{"fraction": 0.25}`: of the outline's length (a taper's `length`) or of the stroke's `width` (a taper's `width`).
    Fraction(P),
}

impl<'de> Deserialize<'de> for Extent {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        let fraction = v.as_object().filter(|o| o.len() == 1).and_then(|o| o.get("fraction"));
        let prop = |v: &Value| serde_json::from_value::<P>(v.clone()).map_err(serde::de::Error::custom);
        match fraction {
            Some(f) => prop(f).map(Extent::Fraction),
            None => prop(&v).map(Extent::Px),
        }
    }
}

impl Extent {
    pub fn property(&self) -> &P {
        match self {
            Extent::Px(p) | Extent::Fraction(p) => p,
        }
    }

    pub fn is_fraction(&self) -> bool {
        matches!(self, Extent::Fraction(_))
    }
}

/// One end of the stroke.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Tip {
    /// How far from the end the stroke takes to reach its full width.
    pub length: Extent,
    /// How wide the stroke is at the end itself (default 0: a point).
    #[serde(default)]
    pub width: Option<Extent>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Taper {
    /// The first point of the outline.
    pub start: Option<Tip>,
    /// The last point of the outline.
    pub end: Option<Tip>,
    /// How the stroke fills out from a tip, GSAP's names (default `linear`).
    #[serde(default)]
    pub ease: Option<Easing>,
}

impl Taper {
    /// The rules the numbers must keep, checked once when the composition is planned (a value that an expression or a
    /// key's overshoot makes negative is caught again by the evaluator).
    pub fn validate(&self) -> Result<()> {
        ensure!(self.start.is_some() || self.end.is_some(), "taper needs a `start` or an `end`: {{\"start\": {{\"length\": 80, \"width\": 0}}}}");
        for (end, tip) in [("start", &self.start), ("end", &self.end)].into_iter().filter_map(|(e, t)| t.as_ref().map(|t| (e, t))) {
            for (what, extent) in [("length", Some(&tip.length)), ("width", tip.width.as_ref())].into_iter().filter_map(|(w, x)| x.map(|x| (w, x))) {
                let p = &extent.property().0;
                if !p.is_plain() {
                    continue;
                }
                for key in p.keys() {
                    if let Key::F(v) = key.v {
                        ensure!(v.is_finite() && v >= 0.0, "taper {end} {what} is 0 or more, not {v}");
                    }
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

    fn parse(v: Value) -> Result<Taper, String> {
        let t: Taper = from_value_at(v).map_err(|e| format!("{e:#}"))?;
        t.validate().map_err(|e| format!("{e:#}"))?;
        Ok(t)
    }

    #[test]
    fn lengths_and_widths_are_pixels_or_fractions() {
        let t = parse(json!({"start": {"length": 120, "width": 2}, "end": {"length": {"fraction": 0.25}, "width": {"fraction": 0.5}}, "ease": "power2.out"})).unwrap();
        let (s, e) = (t.start.unwrap(), t.end.unwrap());
        assert!(!s.length.is_fraction() && !s.width.as_ref().unwrap().is_fraction());
        assert!(e.length.is_fraction() && e.width.as_ref().unwrap().is_fraction());
        assert!(t.ease.is_some());
        assert!(parse(json!({"end": {"length": [[0, 10], [1, 80]]}})).is_ok(), "a length may animate");
    }

    #[test]
    fn mistakes_name_the_field() {
        assert!(parse(json!({})).unwrap_err().contains("`start` or an `end`"));
        assert!(parse(json!({"start": {"width": 0}})).unwrap_err().contains("length"));
        assert!(parse(json!({"start": {"length": -4}})).unwrap_err().contains("taper start length is 0 or more, not -4"));
        assert!(parse(json!({"end": {"length": 10, "width": -1}})).unwrap_err().contains("taper end width"));
        assert!(parse(json!({"start": {"lenght": 10}})).unwrap_err().contains("Did you mean \"length\"?"));
        assert!(parse(json!({"start": {"length": 10}, "ease": "wobble"})).unwrap_err().contains("unknown ease"));
        assert!(parse(json!({"start": {"length": "x +"}})).is_err(), "an expression that does not parse is refused");
    }
}
