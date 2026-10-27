//! Blending two colours the way the working profile blends light.
//!
//! A colour in a composition is an sRGB colour: the numbers a literal such as `#808080` names. Skia converts a gradient's
//! stops into the working profile and interpolates there, so a gradient in a linear-light film is a gradient of light.
//! A colour computed on the CPU (the colour a repeater's copy takes from its palette) must blend in the same numbers to
//! agree with that gradient, and [`Profile::mix`] does: the curve is Skia's, read from its sRGB colour space, so there
//! is no second definition of it.
//!
//! Only the transfer curve decides the result. A change of gamut is a matrix on light, and a matrix commutes with a
//! blend of light, so the primaries of the working profile do not change a blend in linear light.

use skia_safe::{ColorSpace, ColorSpaceTransferFn};

use super::profile::{Profile, Transfer};

/// Skia's parametric curve (`skcms_TransferFunction`) at `x`, odd about zero as Skia's is.
fn curve(f: &ColorSpaceTransferFn, x: f32) -> f32 {
    let magnitude = x.abs();
    let y = if magnitude < f.d { f.c * magnitude + f.f } else { (f.a * magnitude + f.b).powf(f.g) + f.e };
    y.copysign(x)
}

impl Profile {
    /// The colour `t` of the way from `a` to `b`, both sRGB colours (channels 0 to 1), blended in this profile's numbers:
    /// the colours themselves when the profile is display-encoded, their light when it is linear. The result is an sRGB
    /// colour again.
    pub fn mix(self, a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
        let lerp = |x: f32, y: f32| x + (y - x) * t;
        match self.transfer {
            Transfer::Srgb => std::array::from_fn(|c| lerp(a[c], b[c])),
            Transfer::Linear => {
                let srgb = ColorSpace::new_srgb();
                let (to_light, from_light) = (srgb.transfer_fn(), srgb.inv_transfer_fn());
                std::array::from_fn(|c| curve(&from_light, lerp(curve(&to_light, a[c]), curve(&to_light, b[c]))))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{Primaries, tests::srgb_to_linear};

    /// One 8-bit level. Skia's curve is its own single-precision fit of the standard one (up to a hundredth of a level
    /// apart), so the comparison with the standard formula allows a quarter of a level.
    const LEVEL: f64 = 1.0 / 255.0;
    const LINEAR: Profile = Profile { primaries: Primaries::Srgb, transfer: Transfer::Linear };

    fn from_light(v: f64) -> f64 {
        if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
    }

    #[test]
    fn a_display_profile_blends_the_numbers_as_written() {
        let m = Profile::SRGB.mix([0.0, 0.2, 1.0], [1.0, 0.6, 1.0], 0.25);
        for (got, want) in m.into_iter().zip([0.25, 0.3, 1.0]) {
            assert!((got - want).abs() < 1e-6, "{m:?}");
        }
    }

    #[test]
    fn a_linear_profile_blends_light_and_returns_a_colour_on_the_published_curve() {
        // the IEC 61966-2-1 definition written out here, not Skia's own parameters
        for (a, b, t) in [(0.0, 1.0, 0.5), (0.1, 0.9, 0.3), (0.5, 0.5, 0.7), (0.02, 0.04, 0.5), (1.0, 0.0, 0.75)] {
            let want = from_light(srgb_to_linear(a) * f64::from(1.0 - t) + srgb_to_linear(b) * f64::from(t));
            let got = f64::from(LINEAR.mix([a as f32; 3], [b as f32; 3], t)[1]);
            assert!((got - want).abs() < LEVEL / 4.0, "{a} {b} {t}: {got} against {want}");
        }
        // black and white halfway is half the light, which an sRGB display shows as level 188
        let half = LINEAR.mix([0.0; 3], [1.0; 3], 0.5)[0];
        assert_eq!((half * 255.0).round() as u8, 188);
    }

    #[test]
    fn the_ends_of_a_blend_are_the_colours_and_the_gamut_makes_no_difference_in_light() {
        let (a, b) = ([0.9, 0.1, 0.4], [0.2, 0.8, 0.0]);
        for (t, end) in [(0.0, a), (1.0, b)] {
            for (got, want) in LINEAR.mix(a, b, t).into_iter().zip(end) {
                assert!(f64::from((got - want).abs()) < LEVEL / 4.0, "{t}: {got} {want}");
            }
        }
        let p3 = Profile { primaries: Primaries::DisplayP3, transfer: Transfer::Linear };
        assert_eq!(p3.mix(a, b, 0.4), LINEAR.mix(a, b, 0.4));
    }
}
