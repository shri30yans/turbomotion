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
        if path == "." { anyhow!("{}", e.inner()) } else { anyhow!("{path}: {}", e.inner()) }
    })
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

    #[test]
    fn a_failing_property_is_named() {
        let err = from_value_at::<Probe>(serde_json::json!({"opacity": {"from": [1], "to": 0}})).unwrap_err().to_string();
        assert!(err.starts_with("opacity: "), "{err}");
    }
}
