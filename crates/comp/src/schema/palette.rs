//! A palette that colours the copies of a repeater: `palette` (the colours), `cycle` (how copies pick from them) and
//! `paint` (which paint of the copy takes the colour). One definition serves every place a copy is made: the layer
//! `repeat`, the shape item's `repeat` and the `swatch` a repeated layer carries. What copy `i` gets is `anim::cycle`;
//! the evaluator turns it into a colour (`eval::palette`).
//!
//! ```json
//! "repeat": {"count": 8, "offset": [0, -12], "palette": ["#ffd60a", "#ff4d8d", "#22d3ee"], "cycle": {"mode": "step", "offset": 1}}
//! ```

use anyhow::{anyhow, bail, ensure, Result};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use super::{de, fields, shape::ShapeItem, wrap::{Col, P}, Kind};
use crate::anim::cycle::{CycleMode, NonEmpty};

/// The keys that make a palette, as written next to a repeater's own.
pub const KEYS: [&str; 3] = ["palette", "cycle", "paint"];

/// How copies pick from the palette.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Cycle {
    /// `step` (default), `gradient` or `ping_pong`.
    pub mode: CycleMode,
    /// How many colours the palette is shifted on, a property: it may animate and may be negative or fractional.
    pub offset: Option<P>,
    /// Read the palette from its last colour to its first.
    pub reverse: bool,
}

/// Which paint of a copy the palette colours.
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    /// The fill; a text or solid layer's own colour.
    #[default]
    Fill,
    Stroke,
    Both,
}

impl Target {
    pub fn fill(self) -> bool {
        self != Target::Stroke
    }

    pub fn stroke(self) -> bool {
        self != Target::Fill
    }
}

/// The palette, how its colours are picked and which paint takes them.
#[derive(Debug, Clone)]
pub struct Cycling {
    pub palette: NonEmpty<Col>,
    pub cycle: Cycle,
    pub paint: Target,
}

impl Cycling {
    /// Remove `palette`, `cycle` and `paint` from `o` and read them. `None` when `o` has none of them.
    pub fn take(o: &mut Map<String, Value>) -> Result<Option<Cycling>> {
        let (palette, cycle, paint) = (o.remove("palette"), o.remove("cycle"), o.remove("paint"));
        let Some(palette) = palette else {
            ensure!(cycle.is_none() && paint.is_none(), "`cycle` and `paint` say how a palette is used: add a `palette` of colours");
            return Ok(None);
        };
        let list = palette.as_array().ok_or_else(|| anyhow!("palette is a list of colours like [\"#ffd60a\", \"#ff4d8d\"], found {palette}"))?;
        let colours = list.iter().enumerate().map(|(i, c)| de::from_value_at::<Col>(c.clone()).map_err(|e| anyhow!("palette[{i}]: {e:#}"))).collect::<Result<Vec<_>>>()?;
        let palette = NonEmpty::new(colours).ok_or_else(|| anyhow!("palette has no colours: give at least one"))?;
        let cycle = cycle.map_or_else(|| Ok(Cycle::default()), |c| de::from_value_at(c).map_err(|e| anyhow!("cycle: {e:#}")))?;
        let paint = paint.map_or_else(|| Ok(Target::default()), |p| de::from_value_at(p).map_err(|e| anyhow!("paint: {e:#}")))?;
        Ok(Some(Cycling { palette, cycle, paint }))
    }

    /// Which of its paints a shape item would take from the palette: `(fill, stroke)`. A fill takes it only when it is a
    /// flat colour, because a copy takes one colour.
    pub fn paints(&self, item: &ShapeItem) -> (bool, bool) {
        let flat = item.fill.as_ref().is_some_and(|f| f.color.is_some() && f.gradient.is_none() && f.metal.is_none());
        (self.paint.fill() && flat, self.paint.stroke() && item.stroke.is_some())
    }

    /// What a repeater inside a shape item asks of it: every paint the palette names must be there to be coloured.
    pub fn check_item(&self, item: &ShapeItem) -> Result<()> {
        let (fill, stroke) = self.paints(item);
        ensure!(fill || !self.paint.fill(), "the palette colours the fill, so the item needs a flat-colour fill (not a gradient or metal, and not none); use `paint: \"stroke\"` to colour the stroke only");
        ensure!(stroke || !self.paint.stroke(), "the palette colours the stroke but this shape item has no stroke; use `paint: \"fill\"` to colour the fill only");
        Ok(())
    }
}

/// One colour of a palette cycle, chosen by the layer's place in a row: what a layer repeater gives each copy. It is
/// also a key a hand-built row can use.
///
/// ```json
/// "swatch": {"palette": ["#ffd60a", "#ff4d8d"], "index": 3, "cycle": {"mode": "gradient"}}
/// ```
///
/// The layer draws with colour number `index` of the cycle instead of its own colour: a text layer's `color` (and its
/// `stroke` colour with `paint: "stroke"`), a solid's `color`, the fills and strokes of a shape layer's items.
#[derive(Debug, Clone)]
pub struct Swatch {
    pub cycling: Cycling,
    /// The copy's place in the row, counting from 0.
    pub index: usize,
}

impl Swatch {
    fn parse(v: &Value) -> Result<Swatch> {
        let o = v.as_object().ok_or_else(|| anyhow!("swatch is an object like {{\"palette\": [\"#fff\", \"#f00\"], \"index\": 0}}"))?;
        let valid = ["index", KEYS[0], KEYS[1], KEYS[2]];
        if let Some(message) = fields::stray_keys("swatch", o.keys(), &valid) {
            bail!("{message}");
        }
        let mut rest = o.clone();
        let index = rest.remove("index").and_then(|i| i.as_u64()).ok_or_else(|| anyhow!("swatch needs \"index\": the layer's place in the row, 0 or more"))?;
        let cycling = Cycling::take(&mut rest)?.ok_or_else(|| anyhow!("swatch needs a `palette` of colours"))?;
        Ok(Swatch { cycling, index: usize::try_from(index)? })
    }

    /// Whether a layer of this kind has what the swatch would colour, said once when the composition is planned.
    pub fn check(&self, kind: &Kind) -> Result<()> {
        let paint = self.cycling.paint;
        match kind {
            Kind::Solid(_) => ensure!(!paint.stroke(), "a solid has a colour and no stroke: leave `paint` out or use \"fill\""),
            Kind::Text(t) => {
                ensure!(t.fill.is_none() || !paint.fill(), "this text has a `fill`, which is what is drawn instead of `color`: the swatch colours `color`, so remove the fill");
                ensure!(t.stroke.is_some() || !paint.stroke(), "the swatch colours the stroke but this text has no `stroke`");
            }
            Kind::Shape(s) => {
                let covered = s.items.iter().any(|i| self.cycling.paints(i) != (false, false));
                ensure!(covered, "no item of this shape layer has a paint for the swatch to colour ({paint:?}): a fill must be a flat colour and a stroke must exist");
            }
            other => bail!("a swatch colours a text, solid or shape layer, not {} (a repeater's palette needs one of those)", other.name()),
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for Swatch {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Swatch::parse(&Value::deserialize(d)?).map_err(|e| serde::de::Error::custom(format!("{e:#}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn take(v: Value) -> Result<Option<Cycling>> {
        Cycling::take(v.as_object().cloned().as_mut().ok_or_else(|| anyhow!("an object"))?)
    }

    fn refused(v: Value) -> String {
        format!("{:#}", take(v).unwrap_err())
    }

    #[test]
    fn a_palette_with_a_cycle_and_a_paint_is_read_and_removed() {
        let mut o = json!({"copies": 4, "palette": ["#f00", "#00f"], "cycle": {"mode": "ping_pong", "offset": [[0, 0], [1, 2]], "reverse": true}, "paint": "both"}).as_object().cloned().unwrap_or_default();
        let c = Cycling::take(&mut o).unwrap().expect("a palette");
        assert_eq!((c.cycle.mode, c.cycle.reverse, c.paint, c.palette.len().get()), (CycleMode::PingPong, true, Target::Both, 2));
        assert!(c.cycle.offset.is_some());
        assert_eq!(o.keys().collect::<Vec<_>>(), ["copies"]);
        assert!(take(json!({"copies": 4})).unwrap().is_none());
        let plain = take(json!({"palette": ["#fff"]})).unwrap().expect("a palette");
        assert_eq!((plain.cycle.mode, plain.paint, plain.cycle.reverse), (CycleMode::Step, Target::Fill, false));
    }

    #[test]
    fn mistakes_say_what_to_write() {
        assert!(refused(json!({"palette": []})).contains("palette has no colours"));
        assert!(refused(json!({"palette": "#fff"})).contains("list of colours"));
        let bad = refused(json!({"palette": ["#fff", 5]}));
        assert!(bad.contains("palette[1]") && bad.contains("a colour is"), "{bad}");
        assert!(refused(json!({"palette": ["nothex"]})).contains("palette[0]"));
        assert!(refused(json!({"cycle": {"mode": "step"}})).contains("add a `palette`"));
        let mode = refused(json!({"palette": ["#fff"], "cycle": {"mode": "stepp"}}));
        assert!(mode.contains("cycle:") && mode.contains("stepp") && mode.contains("ping_pong"), "{mode}");
        let key = refused(json!({"palette": ["#fff"], "cycle": {"ofset": 1}}));
        assert!(key.contains("Did you mean \"offset\"?"), "{key}");
        assert!(refused(json!({"palette": ["#fff"], "paint": "outline"})).contains("paint:"));
    }

    #[test]
    fn a_swatch_needs_an_index_and_a_palette_and_names_a_stray_key() {
        let parse = |v: Value| de::from_value_at::<Swatch>(v).map_err(|e| format!("{e:#}"));
        let ok = parse(json!({"palette": ["#fff", "#000"], "index": 3, "cycle": {"mode": "gradient"}})).unwrap();
        assert_eq!((ok.index, ok.cycling.cycle.mode), (3, CycleMode::Gradient));
        assert!(parse(json!({"palette": ["#fff"]})).unwrap_err().contains("index"));
        assert!(parse(json!({"index": 0})).unwrap_err().contains("palette"));
        assert!(parse(json!({"palette": ["#fff"], "index": -1})).unwrap_err().contains("index"));
        assert!(parse(json!({"palette": ["#fff"], "index": 0, "palete": 1})).unwrap_err().contains("Did you mean \"palette\"?"));
    }
}
