//! The definition table: every blend mode, once. Its name, the group After Effects lists it in, how it is drawn and a
//! sentence of what it does. Nothing else in the engine says anything about one mode in particular: the parser, the
//! schema's list of names, the catalogue, the documentation and the choice of how to draw all read this table.

use skia_safe::BlendMode;

use super::Blend;

/// The groups of After Effects' blending-mode menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Normal,
    Subtractive,
    Additive,
    Complex,
    Difference,
    Hsl,
    Matte,
    Utility,
}

impl Group {
    pub const ALL: [Group; 8] = [Group::Normal, Group::Subtractive, Group::Additive, Group::Complex, Group::Difference, Group::Hsl, Group::Matte, Group::Utility];

    pub fn name(self) -> &'static str {
        match self {
            Group::Normal => "normal",
            Group::Subtractive => "subtractive",
            Group::Additive => "additive",
            Group::Complex => "complex",
            Group::Difference => "difference",
            Group::Hsl => "hsl",
            Group::Matte => "matte",
            Group::Utility => "utility",
        }
    }
}

/// How a mode reaches the pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    /// Skia has the mode: the paint carries it.
    Native(BlendMode),
    /// One of the modes of the shared blender (`shaders/blend.sksl`); its number is its place among these rows.
    Program,
    /// A pattern of whole pixels kept or dropped (`shaders/dissolve.sksl`); `dancing` re-rolls the pattern every frame.
    Dissolve { dancing: bool },
}

/// One row of the table.
#[derive(Debug, Clone, Copy)]
pub struct Mode {
    pub blend: Blend,
    pub name: &'static str,
    pub group: Group,
    pub how: How,
    /// The layer must be a picture of the frame to be drawn: the mode reads the pixel grid, or changes pixels the layer
    /// does not cover (a stencil clears everything the layer leaves transparent).
    pub whole_frame: bool,
    pub summary: &'static str,
}

const fn mode(blend: Blend, name: &'static str, group: Group, how: How, summary: &'static str) -> Mode {
    Mode { blend, name, group, how, whole_frame: false, summary }
}

const fn framed(m: Mode) -> Mode {
    Mode { whole_frame: true, ..m }
}

/// Other spellings that name a mode in the table: After Effects calls Add "Linear Dodge" too.
pub const ALIASES: [(&str, Blend); 1] = [("linear_dodge", Blend::Add)];

use Blend as B;
use Group as G;
use How::{Dissolve, Native, Program};

/// In the order of [`Blend`]'s variants (a test holds it): the original seventeen first, then the rest.
pub const MODES: [Mode; 37] = [
    mode(B::Normal, "normal", G::Normal, Native(BlendMode::SrcOver), "The layer over what is below."),
    mode(B::Add, "add", G::Additive, Native(BlendMode::Plus), "Adds the channels and clips at white. `linear_dodge` is the same mode."),
    mode(B::Screen, "screen", G::Additive, Native(BlendMode::Screen), "Inverts, multiplies and inverts again: never darker, white stays white."),
    mode(B::Multiply, "multiply", G::Subtractive, Native(BlendMode::Multiply), "Multiplies the channels: black stays black, white changes nothing."),
    mode(B::Overlay, "overlay", G::Complex, Native(BlendMode::Overlay), "Multiplies where the base is dark and screens where it is light: the base's contrast is kept."),
    mode(B::SoftLight, "soft_light", G::Complex, Native(BlendMode::SoftLight), "A gentle overlay: lightens or darkens the base by the layer, as a diffuse light would."),
    mode(B::HardLight, "hard_light", G::Complex, Native(BlendMode::HardLight), "Multiplies where the layer is dark and screens where it is light: a harsh spotlight."),
    mode(B::Darken, "darken", G::Subtractive, Native(BlendMode::Darken), "The darker of the two, channel by channel."),
    mode(B::Lighten, "lighten", G::Additive, Native(BlendMode::Lighten), "The lighter of the two, channel by channel."),
    mode(B::ColorDodge, "color_dodge", G::Additive, Native(BlendMode::ColorDodge), "Brightens the base to reflect the layer by lowering contrast; black changes nothing."),
    mode(B::ColorBurn, "color_burn", G::Subtractive, Native(BlendMode::ColorBurn), "Darkens the base to reflect the layer by raising contrast; white changes nothing."),
    mode(B::Difference, "difference", G::Difference, Native(BlendMode::Difference), "The distance between the two, channel by channel: black changes nothing, white inverts."),
    mode(B::Exclusion, "exclusion", G::Difference, Native(BlendMode::Exclusion), "Like difference with less contrast: mid-greys fade to grey."),
    mode(B::Hue, "hue", G::Hsl, Native(BlendMode::Hue), "The layer's hue with the base's saturation and luminosity."),
    mode(B::Saturation, "saturation", G::Hsl, Native(BlendMode::Saturation), "The layer's saturation with the base's hue and luminosity."),
    mode(B::Color, "color", G::Hsl, Native(BlendMode::Color), "The layer's hue and saturation with the base's luminosity: tints without changing the light."),
    mode(B::Luminosity, "luminosity", G::Hsl, Native(BlendMode::Luminosity), "The layer's luminosity with the base's hue and saturation."),
    framed(mode(B::Dissolve, "dissolve", G::Normal, Dissolve { dancing: false }, "Each pixel is either the layer or what is below, chosen by a fixed random pattern: the layer's alpha is the chance a pixel is kept.")),
    framed(mode(B::DancingDissolve, "dancing_dissolve", G::Normal, Dissolve { dancing: true }, "Dissolve with a new pattern every frame.")),
    mode(B::DarkerColor, "darker_color", G::Subtractive, Program, "The whole colour, layer or base, whose channels add up to less."),
    mode(B::LighterColor, "lighter_color", G::Additive, Program, "The whole colour, layer or base, whose channels add up to more."),
    mode(B::LinearBurn, "linear_burn", G::Subtractive, Program, "Adds the channels and subtracts one: a straight-line darkening."),
    mode(B::ClassicColorBurn, "classic_color_burn", G::Subtractive, Program, "The older Color Burn: black burns to black, and opacity fades the layer toward white before it burns."),
    mode(B::ClassicColorDodge, "classic_color_dodge", G::Additive, Program, "The older Color Dodge: white dodges to white, and opacity fades the layer toward black before it dodges."),
    mode(B::LinearLight, "linear_light", G::Complex, Program, "Linear burn where the layer is dark, linear dodge where it is light."),
    mode(B::VividLight, "vivid_light", G::Complex, Program, "Color burn where the layer is dark, color dodge where it is light."),
    mode(B::PinLight, "pin_light", G::Complex, Program, "Replaces the base by the layer where that is darker (layer dark) or lighter (layer light) than the base."),
    mode(B::HardMix, "hard_mix", G::Complex, Program, "Each channel becomes black or white: white where the channels of the two add up to one or more."),
    mode(B::ClassicDifference, "classic_difference", G::Difference, Program, "The older Difference: opacity fades the layer toward black before the distance is taken."),
    mode(B::Subtract, "subtract", G::Difference, Program, "The base minus the layer, never below black."),
    mode(B::Divide, "divide", G::Difference, Program, "The base divided by the layer, never above white; black in the layer gives white where the base is lit."),
    framed(mode(B::StencilAlpha, "stencil_alpha", G::Matte, Native(BlendMode::DstIn), "Everything below shows only where the layer is opaque, and is cleared where the layer is transparent.")),
    framed(mode(B::StencilLuma, "stencil_luma", G::Matte, Program, "Everything below shows in proportion to the layer's luma (its brightness times its alpha), and is cleared where the layer is black or transparent.")),
    mode(B::SilhouetteAlpha, "silhouette_alpha", G::Matte, Native(BlendMode::DstOut), "Everything below is cut away where the layer is opaque and left alone where it is transparent."),
    mode(B::SilhouetteLuma, "silhouette_luma", G::Matte, Program, "Everything below is cut away in proportion to the layer's luma, and left alone where the layer is black or transparent."),
    mode(B::AlphaAdd, "alpha_add", G::Utility, Program, "Adds the two alphas instead of uniting them, so abutting shapes of one colour leave no seam."),
    mode(B::LuminescentPremul, "luminescent_premul", G::Utility, Native(BlendMode::SrcOver), "A premultiplied layer whose colour may exceed its alpha (light) is laid over the base without being clipped to it."),
];

/// The modes drawn by the shared blender, in the order that gives each its number.
pub fn programs() -> impl Iterator<Item = &'static Mode> {
    MODES.iter().filter(|m| matches!(m.how, Program))
}

/// Every name that parses: the modes' own, then the aliases.
pub const NAMES: [&str; MODES.len() + ALIASES.len()] = {
    let mut names = [""; MODES.len() + ALIASES.len()];
    let mut i = 0;
    while i < MODES.len() {
        names[i] = MODES[i].name;
        i += 1;
    }
    let mut j = 0;
    while j < ALIASES.len() {
        names[i + j] = ALIASES[j].0;
        j += 1;
    }
    names
};
