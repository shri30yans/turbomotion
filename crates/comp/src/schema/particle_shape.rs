//! What a particle looks like: a named sprite, or any path you draw (`{"path": {"d": "M0 -1 L1 1 L-1 1 Z"}}`).

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::PathSpec;

#[derive(Debug, Clone, Default)]
pub enum ParticleShape {
    /// A glow: round, soft-edged.
    #[default]
    Soft,
    Circle,
    Square,
    /// A streak along the direction of travel.
    Spark,
    Star,
    Ring,
    /// Your own outline, fitted to the particle's size and turned with its spin. Any `PathSpec` geometry.
    Path(Box<PathSpec>),
}

impl ParticleShape {
    pub const NAMES: &'static [&'static str] = &["soft", "circle", "square", "spark", "star", "ring"];

    pub fn name(&self) -> &'static str {
        match self {
            ParticleShape::Soft => "soft",
            ParticleShape::Circle => "circle",
            ParticleShape::Square => "square",
            ParticleShape::Spark => "spark",
            ParticleShape::Star => "star",
            ParticleShape::Ring => "ring",
            ParticleShape::Path(_) => "path",
        }
    }
}

impl Serialize for ParticleShape {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for ParticleShape {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        match Value::deserialize(d)? {
            Value::String(name) => match name.as_str() {
                "soft" => Ok(ParticleShape::Soft),
                "circle" => Ok(ParticleShape::Circle),
                "square" => Ok(ParticleShape::Square),
                "spark" => Ok(ParticleShape::Spark),
                "star" => Ok(ParticleShape::Star),
                "ring" => Ok(ParticleShape::Ring),
                other => Err(D::Error::custom(format!(
                    "unknown particle shape {other:?}.{} The shapes are {}, or {{\"path\": {{\"d\": \"...\"}}}} for your own outline",
                    turbomotion_core::suggest::hint(other, Self::NAMES),
                    Self::NAMES.join(", ")
                ))),
            },
            Value::Object(o) if o.len() == 1 && o.contains_key("path") => {
                let spec: PathSpec = serde_json::from_value(o["path"].clone()).map_err(|e| D::Error::custom(format!("particle shape path: {e}")))?;
                Ok(ParticleShape::Path(Box::new(spec)))
            }
            other => Err(D::Error::custom(format!("a particle shape is one of {} or {{\"path\": {{...}}}}, not {other}", Self::NAMES.join(", ")))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(v: Value) -> Result<ParticleShape, String> {
        serde_json::from_value(v).map_err(|e| e.to_string())
    }

    #[test]
    fn names_and_paths_parse_and_a_misspelling_is_corrected() {
        assert!(matches!(parse(json!("star")).unwrap(), ParticleShape::Star));
        assert!(matches!(parse(json!({"path": {"d": "M0 0 L1 0 L1 1 Z"}})).unwrap(), ParticleShape::Path(_)));
        let e = parse(json!("strar")).unwrap_err();
        assert!(e.contains("Did you mean \"star\"?") && e.contains("path"), "{e}");
        assert!(parse(json!({"path": {"dd": "x"}})).unwrap_err().contains("dd"));
        assert!(parse(json!(5)).is_err());
    }
}
