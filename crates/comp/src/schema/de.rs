//! Deserialising a layer's fields with the path of the field that failed.
//!
//! `serde_json::from_value` reports "expected a number" without saying which property. Every layer goes through
//! `from_value_at`, so one place adds `opacity:` or `fx.0.radius:` to the message, whatever type the field has.

use anyhow::{anyhow, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// `value` as a `T`; a failure names the property it happened in (`scale: ...`), or the whole object when no field is to blame.
pub fn from_value_at<T: DeserializeOwned>(value: Value) -> Result<T> {
    serde_path_to_error::deserialize(value).map_err(|e| {
        let path = e.path().to_string();
        let message = with_suggestion(&e.inner().to_string());
        if path == "." { anyhow!("{message}") } else { anyhow!("{path}: {message}") }
    })
}

/// serde words a misspelt key as "unknown field `colour`, expected one of `color`, `fill`"; add the nearest valid name.
fn with_suggestion(message: &str) -> String {
    let names: Vec<&str> = message.split('`').skip(1).step_by(2).collect();
    match (message.strip_prefix("unknown field `"), names.split_first()) {
        (Some(_), Some((wrong, valid))) => format!("{message}.{}", turbomotion_core::suggest::hint(wrong, valid)),
        _ => message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::wrap::P;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Probe {
        #[allow(dead_code)]
        opacity: Option<P>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Strict {
        #[allow(dead_code)]
        radius: f64,
        #[allow(dead_code)]
        height: f64,
    }

    #[test]
    fn a_misspelt_key_gets_the_nearest_valid_one() {
        let err = from_value_at::<Strict>(serde_json::json!({"radius": 1, "heigth": 2})).unwrap_err().to_string();
        assert!(err.contains("Did you mean \"height\"?"), "{err}");
        let far = from_value_at::<Strict>(serde_json::json!({"radius": 1, "height": 1, "zzzzzz": 2})).unwrap_err().to_string();
        assert!(far.contains("unknown field") && !far.contains("Did you mean"), "{far}");
    }

    #[test]
    fn a_failing_property_is_named() {
        let err = from_value_at::<Probe>(serde_json::json!({"opacity": {"from": [1], "to": 0}})).unwrap_err().to_string();
        assert!(err.starts_with("opacity: "), "{err}");
    }
}
