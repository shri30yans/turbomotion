//! How a lit 3D layer answers to light.

use serde::Deserialize;

/// How a lit 3D layer answers to light: strengths of the ambient and diffuse response, the specular highlight and its tightness.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Material {
    pub ambient: f64,
    pub diffuse: f64,
    pub specular: f64,
    pub shininess: f64,
}

impl Default for Material {
    fn default() -> Self {
        Material { ambient: 1.0, diffuse: 1.0, specular: 0.0, shininess: 32.0 }
    }
}
