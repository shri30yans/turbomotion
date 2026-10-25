//! Built-in effects take plain numbers or `[seconds, value]` keys, but a plan is written with the same property
//! forms everywhere: `{"from": 7, "to": 0, "at": 1, "dur": 0.5, "ease": "expo.out"}`, or keys with an ease name.
//! This turns those forms into dense plain keys (120 per second), so the curve the author asked for is the curve drawn.

use anyhow::Result;
use serde_json::{json, Value};

use crate::anim::{
    expr::{Arg, Env},
    prop::Prop,
};

struct NoEnv;

impl Env for NoEnv {
    fn var(&self, _: &str) -> Option<f64> {
        None
    }
    fn call(&self, _: &str, _: &[Arg]) -> Option<Result<f64>> {
        None
    }
    fn salt(&self) -> u32 {
        0
    }
}

const STEP: f64 = 1.0 / 120.0;

/// The span of time a property form covers, when it is one of the animated forms this module bakes.
fn span(v: &Value) -> Option<(f64, f64)> {
    match v {
        Value::Object(o) if o.contains_key("from") || o.contains_key("to") => {
            let at = o.get("at").and_then(Value::as_f64).unwrap_or(0.0);
            let dur = o.get("dur").and_then(Value::as_f64).unwrap_or(1.0);
            Some((at, at + dur.max(0.0)))
        }
        Value::Object(o) => {
            let keys = o.get("keys")?.as_array()?;
            Some((keys.first()?.get(0)?.as_f64()?, keys.last()?.get(0)?.as_f64()?))
        }
        Value::Array(keys) if !keys.is_empty() && keys.iter().all(Value::is_array) && keys.iter().any(|k| k.as_array().is_some_and(|k| k.len() > 2)) => {
            Some((keys.first()?.get(0)?.as_f64()?, keys.last()?.get(0)?.as_f64()?))
        }
        _ => None,
    }
}

/// Replace every eased or tweened parameter of effect `fx` with dense `[seconds, value]` keys, in place.
pub fn bake(fx: &mut Value) -> Result<()> {
    let Some(o) = fx.as_object_mut() else { return Ok(()) };
    for (k, v) in o.iter_mut() {
        if k == "effect" {
            continue;
        }
        let Some((t0, t1)) = span(v) else { continue };
        let prop = Prop::parse(v)?;
        let n = (((t1 - t0) / STEP).ceil() as usize).max(1);
        let keys: Vec<Value> = (0..=n)
            .map(|i| {
                let t = (t0 + i as f64 * STEP).min(t1);
                prop.at(t, &NoEnv).map(|y| json!([t, y]))
            })
            .collect::<Result<_>>()?;
        *v = Value::Array(keys);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tween_becomes_dense_keys_that_follow_its_ease() {
        let mut fx = json!({"effect": "blur", "radius": {"from": 8, "to": 0, "at": 1, "dur": 0.5, "ease": "expo.out"}});
        bake(&mut fx).unwrap();
        let keys = fx["radius"].as_array().unwrap();
        assert_eq!(keys.first().unwrap(), &json!([1.0, 8.0]));
        assert!(keys.last().unwrap()[1].as_f64().unwrap().abs() < 1e-9);
        let mid = keys[keys.len() / 4][1].as_f64().unwrap();
        assert!(mid < 4.0, "expo.out is past halfway a quarter of the way in: {mid}");
    }

    #[test]
    fn plain_numbers_and_unkeyed_arrays_are_left_alone() {
        let mut fx = json!({"effect": "blur", "radius": 8});
        bake(&mut fx).unwrap();
        assert_eq!(fx["radius"], json!(8));
        let mut fx = json!({"effect": "blur", "radius": [[0, 1], [2, 3]]});
        bake(&mut fx).unwrap();
        assert_eq!(fx["radius"], json!([[0, 1], [2, 3]]));
    }

    #[test]
    fn eased_keys_are_baked() {
        let mut fx = json!({"effect": "blur", "radius": [[0, 7], [1, 7], [1.5, 0, "expo.out"]]});
        bake(&mut fx).unwrap();
        assert!(fx["radius"].as_array().unwrap().len() > 30);
    }
}
