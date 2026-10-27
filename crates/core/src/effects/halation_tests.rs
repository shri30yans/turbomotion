//! Halation in numbers, on the GPU the renderers use and read back as floats: the energy it adds, the shape of its
//! falloff against the closed-form Gaussian, what counts as "above the threshold" and where its colour comes from.

use serde_json::{json, Value};
use skia_safe::{gpu::{self, Budgeted, SurfaceOrigin}, AlphaType, Color4f, ColorType, ISize, ImageInfo, Paint, Rect, Surface};

use super::{apply, Effect};
use crate::{gpu::Gpu, layer::Layer, shaders::Shaders};

const SIDE: i32 = 256;
const LUMA: [f64; 3] = [0.2126, 0.7152, 0.0722];

fn target(gpu: &mut Gpu) -> Surface {
    let info = ImageInfo::new(ISize::new(SIDE, SIDE), ColorType::RGBAF16, AlphaType::Premul, None);
    gpu::surfaces::render_target(&mut gpu.context, Budgeted::Yes, &info, None, SurfaceOrigin::TopLeft, None, false, false).expect("a render target")
}

/// A picture on `background` (opaque black or transparent) with white squares of `side` px at the given corners, each
/// `[x, y, level, alpha]`, and the pixels of the picture after `effect`: premultiplied floats, row by row.
fn halated(background: Color4f, squares: &[[f32; 4]], side: f32, effect: Value) -> Vec<[f32; 4]> {
    let mut gpu = Gpu::new().expect("a GPU");
    let mut like = target(&mut gpu);
    let shaders = Shaders::compile().expect("the shaders compile");
    let picture = Layer::render(like.canvas(), SIDE, SIDE, |c| {
        c.clear(background);
        for [x, y, level, alpha] in squares {
            let mut paint = Paint::new(Color4f::new(*level, *level, *level, *alpha), None);
            paint.set_anti_alias(false);
            c.draw_rect(Rect::from_xywh(*x, *y, side, side), &paint);
        }
    })
    .expect("a picture");
    let mut params = effect.as_object().expect("an object").clone();
    params.remove("effect");
    let chain = [Effect::build("film_halation", &params).expect("the effect builds")];
    let out = apply(&shaders, like.canvas(), picture, &chain, 0.0).expect("the effect runs");
    like.canvas().clear(Color4f::new(0.0, 0.0, 0.0, 0.0));
    like.canvas().draw_image(out.image(), (0, 0), None);
    let info = ImageInfo::new(like.image_info().dimensions(), ColorType::RGBAF32, AlphaType::Premul, None);
    let mut bytes = vec![0u8; info.compute_min_byte_size()];
    assert!(like.read_pixels(&info, &mut bytes, info.min_row_bytes(), (0, 0)));
    bytes.chunks_exact(16).map(|px| std::array::from_fn(|c| f32::from_ne_bytes(px[c * 4..c * 4 + 4].try_into().expect("four bytes")))).collect()
}

const BLACK: Color4f = Color4f::new(0.0, 0.0, 0.0, 1.0);
const CLEAR: Color4f = Color4f::new(0.0, 0.0, 0.0, 0.0);
const CENTRE: f32 = (SIDE / 2) as f32;

fn halation(threshold: f64, radius: f64, strength: f64, color: [u8; 3]) -> Value {
    json!({"effect": "film_halation", "threshold": threshold, "radius": radius, "strength": strength, "color": color})
}

/// The sum over the picture of channel `c` (0 red .. 2 blue) of what the effect added to `before`.
fn added(after: &[[f32; 4]], before: &[[f32; 4]], c: usize) -> f64 {
    after.iter().zip(before).map(|(a, b)| f64::from(a[c]) - f64::from(b[c])).sum()
}

fn unchanged(squares: &[[f32; 4]], side: f32) -> Vec<[f32; 4]> {
    halated(BLACK, squares, side, halation(0.5, 12.0, 0.0, [255, 255, 255]))
}

fn near(what: &str, got: f64, want: f64, tolerance: f64) {
    assert!((got - want).abs() <= tolerance * want.abs(), "{what}: {got} against {want} (tolerance {tolerance} of it)");
}

/// The sigma up to which Skia's Gaussian keeps the light it blurs (see `halation.rs`).
const EXACT: f64 = 4.0;

#[test]
fn the_luma_it_adds_is_strength_times_the_light_above_the_threshold_summed_over_the_picture() {
    // a 4x4 white square: 16 px, each holding 1 - 0.5 of light above the threshold
    let square = [[CENTRE - 2.0, CENTRE - 2.0, 1.0, 1.0]];
    let (before, after) = (unchanged(&square, 4.0), halated(BLACK, &square, 4.0, halation(0.5, EXACT, 0.5, [255, 90, 40])));
    let excess = 16.0 * (1.0 - 0.5);
    let tint = [1.0, 90.0 / 255.0, 40.0 / 255.0];
    let tint_luma: f64 = tint.iter().zip(LUMA).map(|(t, l)| t * l).sum();
    let mut luma = 0.0;
    for c in 0..3 {
        let channel = added(&after, &before, c);
        near(&format!("channel {c}"), channel, 0.5 * excess * tint[c] / tint_luma, 0.005);
        luma += LUMA[c] * channel;
    }
    near("added luma = 0.5 x 8", luma, 4.0, 0.005);
    assert!(after.iter().all(|p| p[3] == 1.0), "the opaque picture stays opaque");
}

/// The kernel Skia blurs with: a Gaussian sampled at whole pixels, cut at `ceil(3 sigma)` and made to sum to 1.
fn kernel(sigma: f64) -> impl Fn(i32) -> f64 {
    let radius = (3.0 * sigma).ceil() as i32;
    let g = move |x: i32| (-f64::from(x * x) / (2.0 * sigma * sigma)).exp();
    let total: f64 = (-radius..=radius).map(g).sum();
    move |x| if x.abs() <= radius { g(x) / total } else { 0.0 }
}

/// The light a `side` px square of 1.0 puts on the pixel `d` from its centre pixel along one axis: the kernel summed
/// over the pixels of the square.
fn across(side: i32, sigma: f64, d: i32) -> f64 {
    let k = kernel(sigma);
    (0..side).map(|i| k(d - (i - side / 2))).sum()
}

fn profile(after: &[[f32; 4]], before: &[[f32; 4]], dx: i32, dy: i32) -> f64 {
    let i = ((SIDE / 2 + dy) * SIDE + SIDE / 2 + dx) as usize;
    f64::from(after[i][1]) - f64::from(before[i][1])
}

#[test]
fn the_falloff_of_a_point_is_the_gaussian_the_radius_names() {
    // one white pixel, no threshold, white tint, strength 1: the picture's added light is the blur's point spread
    let point = [[CENTRE, CENTRE, 1.0, 1.0]];
    let (before, after) = (unchanged(&point, 1.0), halated(BLACK, &point, 1.0, halation(0.0, EXACT, 1.0, [255, 255, 255])));
    near("the sum is the source's light", added(&after, &before, 1), 1.0, 0.005);
    for d in [0, 2, 4, 8, 12] {
        let want = across(1, EXACT, d) * across(1, EXACT, 0);
        near(&format!("{d} px out, across"), profile(&after, &before, d, 0), want, 0.03);
        near(&format!("{d} px out, down"), profile(&after, &before, 0, d), want, 0.03);
    }
    // the second moment along a row is the cut kernel's: a little under sigma squared, as the cut at 3 sigma takes the tails
    let k = kernel(EXACT);
    let want = (-12..=12).map(|x| k(x) * f64::from(x * x)).sum::<f64>().sqrt();
    let (mut mass, mut moment) = (0.0, 0.0);
    for dx in -64..=64 {
        let v = profile(&after, &before, dx, 0);
        mass += v;
        moment += v * f64::from(dx * dx);
    }
    near("the standard deviation along a row", (moment / mass).sqrt(), want, 0.005);
}

#[test]
fn a_wide_halo_follows_the_closed_form_to_four_percent_of_its_peak() {
    // a 64 px square blurred by sigma 24 is Skia's reduced-resolution path; the closed form is the square summed
    // against the sampled Gaussian, and Skia's own filtering is held to 4% of the peak (it measures 3.0% low at the centre, falling to 0.3% at 64 px out: the reduced-resolution
    // path blurs slightly wider than its sigma)
    let (side, sigma) = (64, 24.0);
    let square = [[CENTRE - 32.0, CENTRE - 32.0, 1.0, 1.0]];
    let (before, after) = (unchanged(&square, 64.0), halated(BLACK, &square, 64.0, halation(0.0, sigma, 1.0, [255, 255, 255])));
    let peak = across(side, sigma, 0) * across(side, sigma, 0);
    for d in [0, 16, 32, 48, 64] {
        let want = across(side, sigma, d) * across(side, sigma, 0);
        let got = profile(&after, &before, d, 0);
        assert!((got - want).abs() <= 0.04 * peak, "{d} px out: {got} against {want}; the peak is {peak}");
    }
    let sum = added(&after, &before, 1) / f64::from(side * side);
    assert!((sum - 1.0).abs() < 0.05, "light kept at sigma 24 from a 64 px square: {sum}");
}

#[test]
fn only_light_above_the_threshold_halates_and_a_covered_pixel_counts_by_its_coverage() {
    let (dim, bright) = ([[40.0, 100.0, 0.4, 1.0]], [[40.0, 100.0, 0.6, 1.0]]);
    let effect = halation(0.5, EXACT, 1.0, [255, 255, 255]);
    let below = halated(BLACK, &dim, 16.0, effect.clone());
    assert!(below.iter().zip(unchanged(&dim, 16.0)).all(|(a, b)| *a == b), "light of 0.4 under a threshold of 0.5 adds nothing, not a little");
    let above = halated(BLACK, &bright, 16.0, effect);
    near("0.6 over 0.5 on 256 px", added(&above, &unchanged(&bright, 16.0), 1), 256.0 * 0.1, 0.01);
    // half-covered white (a paint's alpha is held in 8 bits, so 128/255): straight luma 1 and coverage a leave
    // (1 - 0.5) x a of light above the threshold
    let a = 128.0 / 255.0;
    let half = [[CENTRE, CENTRE, 1.0, a as f32]];
    let after = halated(CLEAR, &half, 1.0, halation(0.5, 0.0, 1.0, [255, 255, 255]));
    near("a half-covered white pixel", f64::from(after[(CENTRE as i32 * SIDE + CENTRE as i32) as usize][1]), a + 0.5 * a, 0.001);
}

#[test]
fn the_halo_has_the_tints_hue_and_lights_a_transparent_picture_to_hold_it() {
    let out = halated(CLEAR, &[[CENTRE - 4.0, CENTRE - 4.0, 1.0, 1.0]], 8.0, halation(0.5, EXACT, 0.4, [255, 90, 40]));
    let p = out[(CENTRE as i32 * SIDE + CENTRE as i32 + 8) as usize];
    assert!(p[3] > 0.0, "the halo has coverage {}", p[3]);
    assert!((f64::from(p[1] / p[0]) - 90.0 / 255.0).abs() < 2e-3 && (f64::from(p[2] / p[0]) - 40.0 / 255.0).abs() < 2e-3, "green/red and blue/red {} {}", p[1] / p[0], p[2] / p[0]);
    assert!((p[3] - p[0]).abs() < 1e-6, "coverage is the strongest channel, here red: {} against {}", p[3], p[0]);
}

#[test]
fn no_strength_changes_nothing_and_no_radius_adds_the_excess_where_it_is() {
    let square = [[CENTRE, CENTRE, 1.0, 1.0]];
    let still = halated(BLACK, &square, 1.0, halation(0.5, 12.0, 0.0, [255, 90, 40]));
    let plain = halated(BLACK, &square, 1.0, halation(1.0, 12.0, 1.0, [255, 90, 40]));
    assert!(still.iter().zip(&plain).all(|(a, b)| a == b), "a strength of 0 and a threshold of 1 both leave the picture as it was");
    let sharp = halated(BLACK, &square, 1.0, halation(0.5, 0.0, 1.0, [255, 255, 255]));
    let i = (CENTRE as i32 * SIDE + CENTRE as i32) as usize;
    near("an unblurred halo adds the part above the threshold", f64::from(sharp[i][0]), 1.0 + 0.5, 0.002);
    assert_eq!(sharp[i + 1][0], 0.0, "and reaches no further");
}
