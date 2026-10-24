//! Runtime shaders: SkSL programs compiled once, with uniforms packed in declaration order and checked against the
//! program's own uniform size so a mismatched call fails loudly instead of drawing garbage.

use anyhow::{anyhow, ensure, Result};
use skia_safe::{runtime_effect::ChildPtr, Blender, Data, Matrix, RuntimeEffect, Shader};

/// Uniform values in declaration order.
#[derive(Default)]
pub struct Uniforms(Vec<u8>);

impl Uniforms {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn f(mut self, v: f64) -> Self {
        self.0.extend_from_slice(&(v as f32).to_ne_bytes());
        self
    }

    pub fn v2(self, a: f64, b: f64) -> Self {
        self.f(a).f(b)
    }

    pub fn v3(self, a: f64, b: f64, c: f64) -> Self {
        self.f(a).f(b).f(c)
    }

    pub fn v4(self, a: f64, b: f64, c: f64, d: f64) -> Self {
        self.f(a).f(b).f(c).f(d)
    }

    /// An sRGB colour as `float3` in 0..1.
    pub fn rgb(self, c: [u8; 3]) -> Self {
        self.v3(f64::from(c[0]) / 255.0, f64::from(c[1]) / 255.0, f64::from(c[2]) / 255.0)
    }

    /// Several `float4` rows (a uniform array).
    pub fn rows4(mut self, rows: &[[f64; 4]]) -> Self {
        for r in rows {
            self = self.v4(r[0], r[1], r[2], r[3]);
        }
        self
    }
}

pub struct Program {
    name: &'static str,
    effect: RuntimeEffect,
}

impl Program {
    pub fn shader(name: &'static str, sksl: &str) -> Result<Self> {
        let effect = RuntimeEffect::make_for_shader(sksl, None).map_err(|e| anyhow!("shader {name}: {e}"))?;
        Ok(Self { name, effect })
    }

    pub fn blender(name: &'static str, sksl: &str) -> Result<Self> {
        let effect = RuntimeEffect::make_for_blender(sksl, None).map_err(|e| anyhow!("blender {name}: {e}"))?;
        Ok(Self { name, effect })
    }

    fn data(&self, u: Uniforms) -> Result<Data> {
        ensure!(u.0.len() == self.effect.uniform_size(), "{}: {} uniform bytes given, the program declares {}", self.name, u.0.len(), self.effect.uniform_size());
        Ok(Data::new_copy(&u.0))
    }

    /// An instance of the shader bound to `uniforms` and its child shaders (in declaration order).
    pub fn instance(&self, u: Uniforms, children: &[Shader]) -> Result<Shader> {
        let kids: Vec<ChildPtr> = children.iter().cloned().map(ChildPtr::Shader).collect();
        self.effect.make_shader(self.data(u)?, &kids, None::<&Matrix>).ok_or_else(|| anyhow!("{}: could not make a shader instance", self.name))
    }

    pub fn blend(&self, u: Uniforms) -> Result<Blender> {
        self.effect.make_blender(self.data(u)?, None).ok_or_else(|| anyhow!("{}: could not make a blender instance", self.name))
    }
}
