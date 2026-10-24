//! Packs: domain graphics that live outside this crate and still sit on a composition's timeline. A globe, a map or a
//! chart is not a composition feature, so `comp` knows only this interface; the crate that owns the graphic implements
//! [`Pack`] and the binary registers it once at startup with [`register`]. A layer `{"type": "pack", "name":
//! "studio.globe", "params": {...}}` then asks the pack to paint a layer-sized picture for the layer's own time.
//!
//! Packs must be a pure function of `(params, t)`, like everything else here, so stills equal renders.

use std::sync::OnceLock;

use anyhow::Result;
use serde_json::Value;
use skia_safe::Canvas;
use turbomotion_core::layer::Layer;

/// What a pack gets to paint with.
pub struct PackCtx<'a> {
    /// A canvas on the GPU context to create compatible surfaces from (use `Layer::try_render`).
    pub like: &'a Canvas,
    pub width: i32,
    pub height: i32,
    /// Where the composition file lives, for relative asset paths.
    pub dir: &'a std::path::Path,
}

pub trait Pack {
    /// The name layers use, like `studio.globe`.
    fn name(&self) -> &'static str;
    /// A short description with the parameters it takes, shown by `comp catalogue`.
    fn summary(&self) -> &'static str;
    /// Check `params` once at plan time so mistakes surface with a path, not at frame 900.
    fn validate(&self, _params: &Value) -> Result<()> {
        Ok(())
    }
    /// Paint the picture for `t` seconds on the layer's own clock.
    fn paint(&self, ctx: &PackCtx, params: &Value, t: f64) -> Result<Layer>;
}

pub type Factory = fn() -> Vec<Box<dyn Pack>>;

static FACTORY: OnceLock<Factory> = OnceLock::new();

/// Register the packs of this build. Called once by the binary; later calls are ignored. Each renderer (one per worker
/// thread) calls the factory for its own instances, because Skia objects stay on their thread.
pub fn register(factory: Factory) {
    let _ = FACTORY.set(factory);
}

pub fn instances() -> Vec<Box<dyn Pack>> {
    FACTORY.get().map_or_else(Vec::new, |f| f())
}

/// `(name, summary)` of every registered pack.
pub fn catalogue() -> Vec<(String, String)> {
    instances().iter().map(|p| (p.name().to_string(), p.summary().to_string())).collect()
}

/// Plan-time check of a pack layer: the pack exists and likes its parameters.
pub fn check(name: &str, params: &Value) -> Result<()> {
    let packs = instances();
    let found = packs.iter().find(|p| p.name() == name).ok_or_else(|| {
        let names: Vec<&str> = packs.iter().map(|p| p.name()).collect();
        anyhow::anyhow!("no pack called {name:?}; this build has {}", if names.is_empty() { "none".to_string() } else { names.join(", ") })
    })?;
    found.validate(params)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A pack that fills the layer with a colour taken from its parameters.
    pub struct Fill;

    impl Pack for Fill {
        fn name(&self) -> &'static str {
            "test.fill"
        }
        fn summary(&self) -> &'static str {
            "fills with params.color (a grey level 0..1)"
        }
        fn validate(&self, params: &Value) -> Result<()> {
            params.get("level").and_then(Value::as_f64).map(|_| ()).ok_or_else(|| anyhow::anyhow!("test.fill needs params.level"))
        }
        fn paint(&self, ctx: &PackCtx, params: &Value, t: f64) -> Result<Layer> {
            let g = (params["level"].as_f64().unwrap_or(0.0) * (1.0 + t)).clamp(0.0, 1.0) as f32;
            Layer::try_render(ctx.like, ctx.width, ctx.height, |c| {
                c.clear(skia_safe::Color4f::new(g, g, g, 1.0));
                Ok(())
            })
        }
    }

    #[test]
    fn a_registered_pack_is_validated_at_plan_time() {
        register(|| vec![Box::new(Fill)]);
        let ok = crate::plan::Plan::from_value(serde_json::json!({"layers": [{"type": "pack", "name": "test.fill", "params": {"level": 0.3}}]}), std::path::Path::new(".")).unwrap();
        assert_eq!(ok.root().layers.len(), 1);
        let e = format!("{:#}", crate::plan::Plan::from_value(serde_json::json!({"layers": [{"type": "pack", "name": "test.fil"}]}), std::path::Path::new(".")).err().unwrap());
        assert!(e.contains("test.fill"), "{e}");
        let e = format!("{:#}", crate::plan::Plan::from_value(serde_json::json!({"layers": [{"type": "pack", "name": "test.fill", "params": {}}]}), std::path::Path::new(".")).err().unwrap());
        assert!(e.contains("params.level"), "{e}");
    }
}
