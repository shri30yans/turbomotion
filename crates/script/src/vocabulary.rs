//! The engine's layer vocabulary for the SDK: which keys a layer of a type accepts. Asked of the schema itself, so the
//! SDK keeps no second list that could drift from what the renderer reads.
use rquickjs::{Ctx, Function, Result};

pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    // `__tm_layer_problems(type, label, keysJson)`: a JSON list of complaints, empty when every key belongs to the type
    let problems = Function::new(ctx.clone(), |ty: String, label: String, keys: String| -> String {
        let keys: Vec<String> = serde_json::from_str(&keys).unwrap_or_default();
        let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
        serde_json::to_string(&turbomotion_comp::api::layer_key_problems(&label, &ty, &keys)).unwrap_or_else(|_| "[]".into())
    })?;
    ctx.globals().set("__tm_layer_problems", problems)
}
