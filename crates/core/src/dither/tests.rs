//! The dither in numbers, on the GPU path the renderer uses: the patterns' balance and blueness, then a smooth dark
//! gradient drawn into an 8-bit frame with and without each dither and measured against the ideal ramp.

use skia_safe::{gpu::{self, Budgeted, SurfaceOrigin}, AlphaType, Canvas, ColorType, ISize, ImageInfo, Paint, Rect, Surface};

use super::{blue_noise, patterns, Dither, Dithering};
use crate::{gpu::Gpu, layer::Layer, shader::{Program, Uniforms}};

const RAMP_W: i32 = 2048;
/// One whole tile of the largest pattern tall, and a slab (below) is a whole tile of the smaller one wide.
const RAMP_H: i32 = 64;
/// Banding is judged on slabs of this many columns over the whole height: 1024 pixels, about the size of the patches the
/// eye averages, and a whole number of tiles of either pattern.
const SLAB: i32 = 16;
/// The ramp runs from this to this, as display-encoded values: a dark stretch, 5.1 to 15.3 levels, where 8 bits band most.
const RAMP: [f64; 2] = [0.02, 0.06];

fn half_float_target(gpu: &mut Gpu, w: i32, h: i32) -> Surface {
    let info = ImageInfo::new(ISize::new(w, h), ColorType::RGBAF16, AlphaType::Premul, None);
    gpu::surfaces::render_target(&mut gpu.context, Budgeted::Yes, &info, None, SurfaceOrigin::TopLeft, None, false, false).expect("a render target")
}

/// `picture` drawn into a new `w` x `h` 8-bit frame the way the renderer draws its finished picture; the red channel.
fn frame(gpu: &mut Gpu, kind: Dither, w: i32, h: i32, picture: &Layer) -> Vec<u8> {
    let mut surface = gpu.frame_surface(ISize::new(w, h), None).expect("a frame");
    let canvas: &Canvas = surface.canvas();
    canvas.clear(skia_safe::Color::BLACK);
    match Dithering::new(kind).expect("the dither builds") {
        Some(dither) => dither.draw(canvas, picture.image()).expect("the dither draws"),
        None => {
            canvas.draw_image(picture.image(), (0, 0), None);
        }
    }
    let info = ImageInfo::new(ISize::new(w, h), ColorType::BGRA8888, AlphaType::Premul, None);
    let mut bytes = vec![0u8; info.compute_min_byte_size()];
    assert!(surface.read_pixels(&info, &mut bytes, info.min_row_bytes(), (0, 0)));
    bytes.chunks_exact(4).map(|px| px[2]).collect()
}

/// A half-float picture whose value is `a + (b - a) * x / w` across, grey.
fn ramp_picture(gpu: &mut Gpu, w: i32, h: i32, a: f64, b: f64) -> Layer {
    let mut like = half_float_target(gpu, w, h);
    let program = Program::shader("ramp", "uniform float3 span; half4 main(float2 p) { float v = span.x + (span.y - span.x) * p.x / span.z; return half4(half3(v), 1.0); }").expect("the ramp compiles");
    let mut paint = Paint::default();
    paint.set_shader(program.instance(Uniforms::new().v3(a, b, f64::from(w)), &[]).expect("the ramp"));
    Layer::render(like.canvas(), w, h, |c| {
        c.draw_rect(Rect::from_wh(w as f32, h as f32), &paint);
    })
    .expect("a picture")
}

/// What the ramp's 8-bit frame is, measured against the exact ramp `255 * (a + (b - a) * (x + 0.5) / w)`.
struct Banding {
    /// Distinct pixel values in the frame.
    levels: usize,
    /// Distinct values among the slab means.
    distinct_means: usize,
    /// The largest jump between neighbouring slabs' means, in levels (the ramp itself climbs 0.08 per slab).
    max_step: f64,
    /// The largest gap between a slab's mean and the exact ramp at its middle, in levels.
    max_mean_error: f64,
    /// The mean and the largest magnitude of the pixel errors, in levels.
    mean_error: f64,
    max_error: f64,
}

fn band(kind: Dither) -> Banding {
    let mut gpu = Gpu::new().expect("a GPU");
    let picture = ramp_picture(&mut gpu, RAMP_W, RAMP_H, RAMP[0], RAMP[1]);
    let px = frame(&mut gpu, kind, RAMP_W, RAMP_H, &picture);
    let ideal = |x: f64| 255.0 * (RAMP[0] + (RAMP[1] - RAMP[0]) * (x + 0.5) / f64::from(RAMP_W));
    let (mut means, mut total, mut max_error) = (Vec::new(), 0.0, 0.0f64);
    for slab in 0..RAMP_W / SLAB {
        let mut sum = 0.0;
        for x in slab * SLAB..(slab + 1) * SLAB {
            for y in 0..RAMP_H {
                let v = f64::from(px[(y * RAMP_W + x) as usize]);
                total += v - ideal(f64::from(x));
                max_error = max_error.max((v - ideal(f64::from(x))).abs());
                sum += v;
            }
        }
        means.push(sum / f64::from(SLAB * RAMP_H));
    }
    let mut distinct: Vec<i64> = means.iter().map(|m| (m * 1e6).round() as i64).collect();
    distinct.sort_unstable();
    distinct.dedup();
    let mut levels: Vec<u8> = px.clone();
    levels.sort_unstable();
    levels.dedup();
    Banding {
        levels: levels.len(),
        distinct_means: distinct.len(),
        max_step: means.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f64::max),
        max_mean_error: means.iter().enumerate().map(|(i, m)| (m - ideal(f64::from(i as i32 * SLAB) + f64::from(SLAB - 1) / 2.0)).abs()).fold(0.0, f64::max),
        mean_error: total / f64::from(RAMP_W * RAMP_H),
        max_error,
    }
}

#[test]
fn every_rank_appears_equally_so_the_mean_threshold_is_exactly_one_half() {
    for tile in [patterns::ordered(), blue_noise::tile().expect("the tile builds")] {
        assert!(patterns::is_balanced(&tile), "side {}", tile.side);
        let total: f64 = tile.ranks.iter().map(|&r| (f64::from(r) + 0.5) / 256.0).sum();
        assert_eq!(total, 0.5 * tile.ranks.len() as f64, "side {}: the thresholds average one half", tile.side);
    }
}

/// The variance of the mean threshold over `block` x `block` pixels of the tile, which for white noise would be one
/// twelfth over the pixel count.
fn block_variance(tile: &patterns::Tile, block: usize) -> f64 {
    let blocks = tile.side / block;
    let means: Vec<f64> = (0..blocks * blocks)
        .map(|i| {
            let (bx, by) = ((i % blocks) * block, (i / blocks) * block);
            (0..block * block).map(|j| (f64::from(tile.ranks[(by + j / block) * tile.side + bx + j % block]) + 0.5) / 256.0).sum::<f64>() / (block * block) as f64
        })
        .collect();
    means.iter().map(|m| (m - 0.5).powi(2)).sum::<f64>() / means.len() as f64
}

#[test]
fn blue_noise_keeps_the_low_frequencies_out_where_white_noise_would_not() {
    let white = 1.0 / 12.0 / 64.0;
    let blue = block_variance(&blue_noise::tile().expect("the tile builds"), 8);
    assert!(blue < white / 10.0, "an 8 x 8 block's mean threshold varies by {blue:.2e} against white noise's {white:.2e}");
}

#[test]
fn the_blue_noise_tile_is_the_same_on_every_build() {
    let (a, b) = (blue_noise::tile().expect("the tile builds"), blue_noise::tile().expect("the tile builds"));
    assert_eq!(a.ranks, b.ranks);
    let checksum = a.ranks.iter().enumerate().fold(0u64, |h, (i, &r)| h.wrapping_mul(1_099_511_628_211).wrapping_add(u64::from(r) * (i as u64 + 1)));
    assert_eq!(checksum, 1_680_497_426_194_267_903, "pinned: the tile's checksum");
}

#[test]
fn no_dither_bands_a_dark_ramp_and_each_dither_removes_the_bands() {
    let none = band(Dither::None);
    assert_eq!(none.levels, 11, "levels 5 to 15 and no other value");
    assert!(none.max_mean_error > 0.45, "and sit up to half a level off the ramp: {}", none.max_mean_error);
    for kind in [Dither::Ordered, Dither::BlueNoise] {
        let b = band(kind);
        assert!(b.distinct_means > 120, "{kind:?}: {} distinct slab means", b.distinct_means);
        assert!(b.max_step < 0.15, "{kind:?}: the biggest step between slabs is {} levels", b.max_step);
        assert!(b.max_mean_error < 0.025, "{kind:?}: a slab's mean is at most {} levels off the ramp", b.max_mean_error);
        assert!(b.mean_error.abs() < 0.01, "{kind:?}: the error averages {} levels", b.mean_error);
        assert!(b.max_error < 1.0 + 0.02, "{kind:?}: no pixel is more than {} levels off", b.max_error);
    }
}

#[test]
fn a_flat_colour_keeps_its_level_except_for_the_half_float_it_is_held_in() {
    // 255 * (k / 255) held in half-float is off by up to k / 2048 levels, so a threshold within that gap moves a pixel one
    // level, up or down: a fraction of the pixels no larger than the gap (plus a rank's width, 1/256). Black and white are exact.
    let mut gpu = Gpu::new().expect("a GPU");
    for kind in [Dither::Ordered, Dither::BlueNoise] {
        for k in [0u8, 1, 17, 64, 128, 200, 254, 255] {
            let v = f64::from(k) / 255.0;
            let picture = ramp_picture(&mut gpu, 64, 64, v, v);
            let px = frame(&mut gpu, kind, 64, 64, &picture);
            assert!(px.iter().all(|&p| p.abs_diff(k) <= 1), "{kind:?} level {k}: a pixel is more than one level away");
            let moved = px.iter().filter(|&&p| p != k).count() as f64 / px.len() as f64;
            let allowed = f64::from(k) / 2048.0 + 1.0 / 256.0;
            assert!(moved <= allowed, "{kind:?} level {k}: {moved} of the pixels moved a level, at most {allowed} allowed");
            if k == 0 || k == 255 {
                assert_eq!(moved, 0.0, "{kind:?} level {k} is exact");
            }
        }
    }
}
