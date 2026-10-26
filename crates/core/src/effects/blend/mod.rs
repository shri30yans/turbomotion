//! How an element's layer combines with what is already on the scene: the layer blend modes of After Effects.
//!
//! One mechanism. A mode is a [`Blend`], a plain enum; everything about one mode is a row of the table in
//! [`table`], and [`Blend::stage`] is the single way a mode is drawn, for a layer on the composite and for an effect's
//! result on its input alike. The rows say how: Skia has the mode (the paint carries it), the shared blender
//! (`shaders/blend.sksl`) computes it, or it is a dissolve (`shaders/dissolve.sksl`).
//!
//! A mode is a function of the numbers the pictures hold. The default film holds display-encoded sRGB, so it blends
//! encoded numbers, as After Effects does in an 8 or 16-bit project; a film with `color.linear_light` holds light, so
//! the same mode blends light (`parity/specs/blend_modes.md`).

mod programs;
mod table;
#[cfg(any(test, feature = "reference"))]
pub mod closed_form;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_dissolve;
#[cfg(test)]
mod tests_modes;

use serde::{de, Deserialize, Deserializer};

pub use programs::Programs;
pub use table::{Group, How, Mode, ALIASES, MODES, NAMES};

use crate::suggest::hint;

/// A layer blend mode. Declared in the order of [`MODES`], so a variant is its own row's index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    Normal,
    Add,
    Screen,
    Multiply,
    Overlay,
    SoftLight,
    HardLight,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
    Dissolve,
    DancingDissolve,
    DarkerColor,
    LighterColor,
    LinearBurn,
    ClassicColorBurn,
    ClassicColorDodge,
    LinearLight,
    VividLight,
    PinLight,
    HardMix,
    ClassicDifference,
    Subtract,
    Divide,
    StencilAlpha,
    StencilLuma,
    SilhouetteAlpha,
    SilhouetteLuma,
    AlphaAdd,
    LuminescentPremul,
}

impl Blend {
    /// This mode's row.
    pub fn mode(self) -> &'static Mode {
        &MODES[self as usize]
    }

    pub fn name(self) -> &'static str {
        self.mode().name
    }

    /// The mode a name or an alias spells, or an error that names the nearest valid spelling and lists them all.
    pub fn parse(name: &str) -> Result<Blend, String> {
        let found = MODES.iter().map(|m| (m.name, m.blend)).chain(ALIASES).find(|(n, _)| *n == name);
        found.map(|(_, b)| b).ok_or_else(|| format!("unknown blend mode {name:?}.{} The modes are {}", hint(name, &NAMES), NAMES.join(", ")))
    }

    /// Whether the layer has to be a picture of the whole frame to be drawn in this mode (see [`Mode::whole_frame`]).
    pub fn needs_frame(self) -> bool {
        self.mode().whole_frame
    }

    /// The number of this mode among those of the shared blender, if it is one of them.
    fn program_code(self) -> Option<usize> {
        table::programs().position(|m| m.blend == self)
    }
}

/// Read as the name or alias a table row spells. Written by hand, not derived, so that a misspelling gets a "did you
/// mean" and the names a parser accepts (which `schema::fields` reads back from `deserialize_enum`) are the table's.
impl<'de> Deserialize<'de> for Blend {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Name;
        impl<'de> de::Visitor<'de> for Name {
            type Value = Blend;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("the name of a blend mode")
            }

            fn visit_enum<A: de::EnumAccess<'de>>(self, data: A) -> Result<Blend, A::Error> {
                let (name, variant): (String, _) = data.variant()?;
                de::VariantAccess::unit_variant(variant)?;
                Blend::parse(&name).map_err(de::Error::custom)
            }
        }
        d.deserialize_enum("Blend", &NAMES, Name)
    }
}
