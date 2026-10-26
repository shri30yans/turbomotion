//! The table, the names and the shared harness for the numbers of every mode (`tests_modes.rs`). Everything here runs on
//! Skia's CPU raster with 32-bit float surfaces, so what is read back is what the blend produced: nothing is rounded to
//! 8 bits or to half-float. The renderers use the GPU; `comp` checks the modes end to end through films.

use skia_safe::{surfaces, AlphaType, Color4f, ColorType, ISize, ImageInfo, Paint, Rect, Surface};

use super::{closed_form::Px, table, Blend, Group, How, Programs, MODES, NAMES};
use crate::layer::Layer;

pub fn px(c: [f64; 3], a: f64) -> Px {
    Px { c, a }
}

fn float_surface(w: i32) -> Surface {
    let info = ImageInfo::new(ISize::new(w, 1), ColorType::RGBAF32, AlphaType::Premul, None);
    surfaces::raster(&info, None, None).expect("a float surface")
}

/// One pixel per colour, side by side.
fn strip(pixels: &[Px]) -> Surface {
    let mut s = float_surface(pixels.len() as i32);
    for (x, p) in pixels.iter().enumerate() {
        let mut paint = Paint::new(Color4f::new(p.c[0] as f32, p.c[1] as f32, p.c[2] as f32, p.a as f32), None);
        paint.set_anti_alias(false);
        s.canvas().draw_rect(Rect::from_xywh(x as f32, 0.0, 1.0, 1.0), &paint);
    }
    s
}

/// The premultiplied float pixels of a surface.
pub fn read(s: &mut Surface) -> Vec<[f32; 4]> {
    let info = ImageInfo::new(s.image_info().dimensions(), ColorType::RGBAF32, AlphaType::Premul, None);
    let mut bytes = vec![0u8; info.compute_min_byte_size()];
    assert!(s.read_pixels(&info, &mut bytes, info.min_row_bytes(), (0, 0)));
    bytes.chunks_exact(16).map(|px| std::array::from_fn(|c| f32::from_ne_bytes(px[c * 4..c * 4 + 4].try_into().expect("four bytes")))).collect()
}

pub fn programs() -> Programs {
    Programs::compile().expect("the blend programs compile")
}

/// `sources[i]` laid on `backdrops[i]` in `blend` at `opacity`, through [`Blend::stage`], as the renderers do.
pub fn blended(blend: Blend, backdrops: &[Px], sources: &[Px], opacity: f32, frame: u32) -> Vec<[f32; 4]> {
    assert_eq!(backdrops.len(), sources.len());
    let mut below = strip(backdrops);
    let layer = Layer::from_image(strip(sources).image_snapshot());
    let mut paint = Paint::default();
    paint.set_alpha_f(opacity);
    let picture = blend.stage(&programs(), below.canvas(), &layer, &mut paint, frame).expect("the mode stages");
    below.canvas().draw_image(picture.image(), (0, 0), Some(&paint));
    read(&mut below)
}

/// One pair, premultiplied.
pub fn one(blend: Blend, backdrop: Px, source: Px) -> [f32; 4] {
    blended(blend, &[backdrop], &[source], 1.0, 0)[0]
}

/// The three channels as they appear when both pixels are opaque grey-or-colour and the result is opaque.
pub fn rgb(p: [f32; 4]) -> [f64; 3] {
    [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])]
}

#[test]
fn the_table_has_one_row_per_variant_in_the_order_of_the_enum() {
    for (i, m) in MODES.iter().enumerate() {
        assert_eq!(m.blend as usize, i, "{} is out of place", m.name);
        assert_eq!(m.blend.mode().name, m.name);
    }
    assert_eq!(Blend::LuminescentPremul as usize + 1, MODES.len(), "a variant without a row");
}

#[test]
fn names_are_unique_lowercase_words_and_every_group_has_a_mode() {
    let mut seen = std::collections::BTreeSet::new();
    for name in NAMES {
        assert!(seen.insert(name), "{name} is listed twice");
        assert!(name.chars().all(|c| c.is_ascii_lowercase() || c == '_'), "{name}");
    }
    for g in Group::ALL {
        assert!(MODES.iter().any(|m| m.group == g), "no mode in {}", g.name());
    }
    assert!(MODES.iter().all(|m| !m.summary.is_empty() && m.summary.ends_with('.')));
}

#[test]
fn the_original_seventeen_modes_are_drawn_by_the_skia_modes_they_always_were() {
    use skia_safe::BlendMode as S;
    let original = [
        ("normal", S::SrcOver), ("add", S::Plus), ("screen", S::Screen), ("multiply", S::Multiply), ("overlay", S::Overlay), ("soft_light", S::SoftLight),
        ("hard_light", S::HardLight), ("darken", S::Darken), ("lighten", S::Lighten), ("color_dodge", S::ColorDodge), ("color_burn", S::ColorBurn),
        ("difference", S::Difference), ("exclusion", S::Exclusion), ("hue", S::Hue), ("saturation", S::Saturation), ("color", S::Color), ("luminosity", S::Luminosity),
    ];
    for (i, (name, skia)) in original.iter().enumerate() {
        assert_eq!(MODES[i].name, *name, "the original modes keep their places");
        assert_eq!(MODES[i].how, How::Native(*skia), "{name} must still be Skia's {skia:?}");
        assert_eq!(Blend::parse(name), Ok(MODES[i].blend));
    }
}

#[test]
fn the_shared_blender_numbers_exactly_the_program_rows() {
    let programs: Vec<&str> = table::programs().map(|m| m.name).collect();
    assert_eq!(programs.len(), MODES.iter().filter(|m| m.how == How::Program).count());
    assert_eq!(programs[0], "darker_color");
    assert!(!programs.contains(&"stencil_alpha") && !programs.contains(&"dissolve"));
    for m in table::programs() {
        assert_eq!(m.blend.program_code(), programs.iter().position(|n| *n == m.name));
    }
    assert_eq!(Blend::Multiply.program_code(), None);
}

#[test]
fn a_name_or_an_alias_parses_and_a_misspelling_names_the_nearest_mode_and_lists_them_all() {
    assert_eq!(Blend::parse("linear_dodge"), Ok(Blend::Add), "After Effects' second name for Add");
    assert_eq!(Blend::parse("vivid_light"), Ok(Blend::VividLight));
    let e = Blend::parse("vivd_light").unwrap_err();
    assert!(e.contains("\"vivd_light\"") && e.contains("Did you mean \"vivid_light\"?") && e.contains("stencil_luma, silhouette_alpha"), "{e}");
    let e = Blend::parse("Multiply").unwrap_err();
    assert!(e.contains("Did you mean \"multiply\"?"), "names are lowercase: {e}");
    let from_json: Blend = serde_json::from_value(serde_json::json!("pin_light")).expect("a name in JSON");
    assert_eq!(from_json, Blend::PinLight);
    let e = serde_json::from_value::<Blend>(serde_json::json!("pin_ligth")).unwrap_err().to_string();
    assert!(e.contains("Did you mean \"pin_light\"?"), "{e}");
    assert!(serde_json::from_value::<Blend>(serde_json::json!(3)).is_err());
}

#[test]
fn only_the_dissolves_and_the_two_stencils_that_reach_past_the_layer_need_the_frame() {
    let framed: Vec<&str> = MODES.iter().filter(|m| m.whole_frame).map(|m| m.name).collect();
    assert_eq!(framed, ["dissolve", "dancing_dissolve", "stencil_alpha", "stencil_luma"]);
    assert!(Blend::StencilAlpha.needs_frame() && !Blend::Multiply.needs_frame());
}
