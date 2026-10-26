//! The numbers of the modes. Every mode but the dissolves is compared with its closed-form value (`closed_form.rs`) on
//! every pair of a small set of colours that includes the edges of each formula (0, 1, mid values) at seven pairs of
//! alphas, and a few results are checked against values worked out by hand so the closed forms themselves are held.

use super::{
    closed_form::{expected, lum, sat, Px},
    tests::{blended, one, px, rgb},
    Blend, How, MODES,
};

/// What the CPU float pipeline may differ from the closed form by. The surfaces and the blender work in 32-bit floats
/// (24-bit significands, a relative step of 6e-8) through a handful of operations per channel. The worst of the 567
/// pairs in any mode measures 3.8e-6 (Skia's colour dodge and burn, which divide by a small difference); the bound is
/// two and a half times that and still a four-hundredth of one 8-bit display level (0.0039).
const FLOAT: f64 = 1e-5;

/// Multiples of 1/16, which a 32-bit float and a 64-bit one hold exactly: a mode that compares (a sum against one, one sum
/// against another) then breaks every tie the same way in the closed form and in the shader.
const COLOURS: [[f64; 3]; 9] = [
    [0.0, 0.0, 0.0],
    [1.0, 1.0, 1.0],
    [0.5, 0.5, 0.5],
    [0.25, 0.5, 0.75],
    [0.875, 0.1875, 0.5],
    [0.125, 0.8125, 0.3125],
    [0.875, 0.0, 0.0],
    [0.0, 0.375, 0.375],
    [1.0, 0.5, 0.0],
];

/// (backdrop alpha, source alpha): both opaque, one translucent, both translucent, a transparent source, a transparent
/// backdrop.
const ALPHAS: [(f64, f64); 7] = [(1.0, 1.0), (1.0, 0.5), (0.5, 1.0), (0.5, 0.5), (0.25, 0.75), (1.0, 0.0), (0.0, 1.0)];

fn every_pair() -> (Vec<Px>, Vec<Px>) {
    let mut pairs = vec![];
    for cb in COLOURS {
        for cs in COLOURS {
            for (da, sa) in ALPHAS {
                pairs.push((px(cb, da), px(cs, sa)));
            }
        }
    }
    pairs.into_iter().unzip()
}

#[test]
fn every_mode_but_the_dissolves_equals_its_closed_form_on_every_pair() {
    let (backdrops, sources) = every_pair();
    assert_eq!(backdrops.len(), 567);
    for m in MODES.iter().filter(|m| !matches!(m.how, How::Dissolve { .. })) {
        let got = blended(m.blend, &backdrops, &sources, 1.0, 0);
        let mut worst = (0.0f64, 0);
        for (i, (g, (b, s))) in got.iter().zip(backdrops.iter().zip(&sources)).enumerate() {
            let want = expected(m.blend, *b, *s);
            let err = (0..4).map(|c| (f64::from(g[c]) - want[c]).abs()).fold(0.0, f64::max);
            if err > worst.0 {
                worst = (err, i);
            }
        }
        let (err, i) = worst;
        assert!(err <= FLOAT, "{}: off by {err:.2e} (bound {FLOAT:.0e}) on backdrop {:?} source {:?}: got {:?}, closed form {:?}", m.name, backdrops[i], sources[i], got[i], expected(m.blend, backdrops[i], sources[i]));
    }
}

fn close(got: [f32; 4], want: [f64; 4], what: &str) {
    for c in 0..4 {
        assert!((f64::from(got[c]) - want[c]).abs() <= FLOAT, "{what}: channel {c} is {} and should be {}", got[c], want[c]);
    }
}

fn grey(v: f64, a: f64) -> Px {
    px([v; 3], a)
}

#[test]
fn values_worked_out_by_hand() {
    let opaque = |b: Blend, base: f64, layer: f64| one(b, grey(base, 1.0), grey(layer, 1.0))[0];
    let eq = |got: f32, want: f64, what: &str| assert!((f64::from(got) - want).abs() <= FLOAT, "{what}: {got} should be {want}");
    eq(opaque(Blend::Multiply, 0.6, 0.3), 0.18, "multiply: 0.6 * 0.3");
    eq(opaque(Blend::Screen, 0.6, 0.3), 0.72, "screen: 0.6 + 0.3 - 0.18");
    eq(opaque(Blend::Overlay, 0.6, 0.3), 0.44, "overlay of a light base: screen(0.3, 2 * 0.6 - 1) = 0.3 + 0.2 - 0.06");
    eq(opaque(Blend::LinearBurn, 0.8, 0.5), 0.3, "linear burn: 0.8 + 0.5 - 1");
    eq(opaque(Blend::LinearBurn, 0.6, 0.3), 0.0, "linear burn below black");
    eq(opaque(Blend::LinearLight, 0.4, 0.75), 0.9, "linear light: 0.4 + 2 * 0.75 - 1");
    eq(opaque(Blend::VividLight, 0.4, 0.75), 0.8, "vivid light, light layer: dodge(0.4, 0.5) = 0.4 / 0.5");
    eq(opaque(Blend::VividLight, 0.4, 0.25), 0.0, "vivid light, dark layer: burn(0.4, 0.5) = 1 - 0.6 / 0.5, held at 0");
    eq(opaque(Blend::PinLight, 0.4, 0.25), 0.4, "pin light, dark layer: min(0.4, 0.5)");
    eq(opaque(Blend::PinLight, 0.4, 0.1), 0.2, "pin light, darker layer: min(0.4, 0.2)");
    eq(opaque(Blend::PinLight, 0.4, 0.9), 0.8, "pin light, light layer: max(0.4, 0.8)");
    eq(opaque(Blend::HardMix, 0.4, 0.6), 1.0, "hard mix: the channels add up to 1");
    eq(opaque(Blend::HardMix, 0.4, 0.5), 0.0, "hard mix: they add up to less");
    eq(opaque(Blend::Subtract, 0.7, 0.2), 0.5, "subtract: 0.7 - 0.2");
    eq(opaque(Blend::Subtract, 0.2, 0.7), 0.0, "subtract below black");
    eq(opaque(Blend::Divide, 0.3, 0.6), 0.5, "divide: 0.3 / 0.6");
    eq(opaque(Blend::Divide, 0.6, 0.3), 1.0, "divide above white");
    eq(opaque(Blend::Divide, 0.5, 0.0), 1.0, "divide by black over a lit base");
    eq(opaque(Blend::Divide, 0.0, 0.0), 0.0, "divide black by black");
}

#[test]
fn the_classic_modes_differ_from_the_new_ones_at_the_edges_and_under_opacity() {
    let at = |b: Blend, base: f64, layer: f64, alpha: f64| one(b, grey(base, 1.0), grey(layer, alpha));
    assert_eq!(at(Blend::ColorDodge, 0.0, 1.0, 1.0)[0], 0.0, "color dodge leaves black base black");
    assert_eq!(at(Blend::ClassicColorDodge, 0.0, 1.0, 1.0)[0], 1.0, "the classic mode dodges white to white even over black");
    assert_eq!(at(Blend::ColorBurn, 1.0, 0.0, 1.0)[0], 1.0, "color burn leaves a white base white");
    assert_eq!(at(Blend::ClassicColorBurn, 1.0, 0.0, 1.0)[0], 0.0, "the classic mode burns black to black even over white");
    // half-transparent grey 0.9 over an opaque 0.6: classic fades it to 0.45 then takes |0.6 - 0.45|; the new mode mixes
    let classic = at(Blend::ClassicDifference, 0.6, 0.9, 0.5);
    close(classic, [0.15, 0.15, 0.15, 1.0], "classic difference at half opacity");
    let new = at(Blend::Difference, 0.6, 0.9, 0.5);
    close(new, [0.45, 0.45, 0.45, 1.0], "difference at half opacity: 0.5 * 0.6 + 0.5 * 0.3");
    assert_eq!(at(Blend::ClassicDifference, 0.6, 0.9, 1.0)[0], at(Blend::Difference, 0.6, 0.9, 1.0)[0], "opaque, they agree");
}

#[test]
fn darker_and_lighter_colour_compare_the_sum_of_the_channels_not_the_luma() {
    // (0.9, 0, 0) sums to 0.9 and has luma 0.27; (0, 0.4, 0.4) sums to 0.8 and has luma 0.28: the orders differ
    let (red, teal) = (px([0.9, 0.0, 0.0], 1.0), px([0.0, 0.4, 0.4], 1.0));
    close(one(Blend::DarkerColor, red, teal), [0.0, 0.4, 0.4, 1.0], "the layer's sum is lower, so the layer stays");
    close(one(Blend::LighterColor, red, teal), [0.9, 0.0, 0.0, 1.0], "the base's sum is higher, so the base stays");
    close(one(Blend::DarkerColor, teal, red), [0.0, 0.4, 0.4, 1.0], "swapped, the lower sum still wins");
    let tie = one(Blend::DarkerColor, px([0.5, 0.25, 0.25], 1.0), px([0.25, 0.5, 0.25], 1.0));
    close(tie, [0.5, 0.25, 0.25, 1.0], "equal sums keep the base");
}

#[test]
fn alpha_add_adds_alphas_and_closes_a_seam_that_over_leaves() {
    let half = px([0.8, 0.4, 0.2], 0.5);
    let over = one(Blend::Normal, half, half);
    let added = one(Blend::AlphaAdd, half, half);
    assert!((over[3] - 0.75).abs() < 1e-6, "over unites the alphas: 0.75");
    assert!((added[3] - 1.0).abs() < 1e-6, "alpha add sums them: 1");
    for (c, v) in [0.8, 0.4, 0.2].into_iter().enumerate() {
        assert!((added[c] - v).abs() < 1e-6, "the colour stays {v}: seamless, got {}", added[c]);
    }
    let opaque = one(Blend::AlphaAdd, px([0.25; 3], 1.0), px([0.5; 3], 1.0));
    close(opaque, [0.5, 0.5, 0.5, 1.0], "an opaque layer covers: alpha add is over when the sum passes 1");
}

#[test]
fn luminescent_premul_keeps_light_above_alpha_that_normal_would_clip_to_it() {
    // premultiplied colour 0.8 with alpha 0.5: a straight colour of 1.6, which is light
    let (below, layer) = (px([0.2; 3], 1.0), px([1.6; 3], 0.5));
    let lit = one(Blend::LuminescentPremul, below, layer);
    close(lit, [0.8 + 0.1, 0.8 + 0.1, 0.8 + 0.1, 1.0], "premultiplied light 0.8 over 0.2 * (1 - 0.5)");
    assert_eq!(one(Blend::Normal, below, layer), lit, "in a half-float engine nothing clips Normal either, so the two agree");
}

#[test]
fn stencils_and_silhouettes_act_on_what_is_below_and_a_transparent_layer_is_the_edge_case() {
    let below = px([0.8, 0.4, 0.2], 1.0);
    let clear = px([0.0; 3], 0.0);
    close(one(Blend::StencilAlpha, below, clear), [0.0; 4], "stencil alpha clears everything under a transparent layer");
    close(one(Blend::SilhouetteAlpha, below, clear), below.premul(), "silhouette alpha leaves everything under a transparent layer");
    close(one(Blend::StencilLuma, below, clear), [0.0; 4], "stencil luma clears under a transparent layer: its luma is 0");
    close(one(Blend::SilhouetteLuma, below, clear), below.premul(), "silhouette luma leaves everything under a transparent layer");
    let half_white = px([1.0; 3], 0.5);
    close(one(Blend::StencilAlpha, below, half_white), [0.4, 0.2, 0.1, 0.5], "stencil alpha: everything below times the alpha");
    close(one(Blend::SilhouetteAlpha, below, half_white), [0.4, 0.2, 0.1, 0.5], "silhouette alpha: everything below times one minus the alpha");
    close(one(Blend::StencilLuma, below, half_white), [0.4, 0.2, 0.1, 0.5], "stencil luma: white at half alpha has luma 0.5");
    close(one(Blend::StencilLuma, below, px([0.0; 3], 1.0)), [0.0; 4], "stencil luma: opaque black has luma 0");
    close(one(Blend::SilhouetteLuma, below, px([0.0; 3], 1.0)), below.premul(), "silhouette luma: opaque black cuts nothing");
    close(one(Blend::SilhouetteLuma, below, px([1.0; 3], 1.0)), [0.0; 4], "silhouette luma: opaque white cuts everything");
    close(one(Blend::StencilLuma, px([1.0; 3], 1.0), px([0.0, 1.0, 0.0], 1.0)), [0.7152; 4], "stencil luma of pure green: its Rec. 709 luma, 0.7152, of an opaque white base");
}

#[test]
fn the_non_separable_modes_keep_the_luminosity_and_hue_they_are_defined_by() {
    // pairs whose result needs no clipping, so each invariant holds exactly
    let pairs = [([0.2, 0.4, 0.6], [0.5, 0.3, 0.4]), ([0.7, 0.5, 0.3], [0.2, 0.6, 0.4]), ([0.3, 0.3, 0.6], [0.6, 0.5, 0.5]), ([0.5, 0.4, 0.6], [0.3, 0.7, 0.5])];
    let tol = 1e-5;
    for (cb, cs) in pairs {
        let (base, layer) = (px(cb, 1.0), px(cs, 1.0));
        let l = rgb(one(Blend::Luminosity, base, layer));
        assert!((lum(l) - lum(cs)).abs() < tol, "luminosity: the result has the layer's luma: {} against {}", lum(l), lum(cs));
        let shift = [l[0] - cb[0], l[1] - cb[1], l[2] - cb[2]];
        assert!((shift[0] - shift[1]).abs() < tol && (shift[1] - shift[2]).abs() < tol, "luminosity moves the base's colour along the grey axis: {shift:?}");
        let c = rgb(one(Blend::Color, base, layer));
        assert!((lum(c) - lum(cb)).abs() < tol, "color keeps the base's luma");
        let h = rgb(one(Blend::Hue, base, layer));
        assert!((lum(h) - lum(cb)).abs() < tol && (sat(h) - sat(cb)).abs() < tol, "hue keeps the base's luma and saturation");
        let s = rgb(one(Blend::Saturation, base, layer));
        assert!((lum(s) - lum(cb)).abs() < tol && (sat(s) - sat(cs)).abs() < tol, "saturation takes the layer's saturation and keeps the base's luma");
    }
}

#[test]
fn a_transparent_layer_changes_nothing_except_in_the_stencils_and_a_transparent_base_shows_the_layer() {
    let (backdrops, sources): (Vec<Px>, Vec<Px>) = COLOURS.iter().map(|c| (px(*c, 0.75), px([0.3, 0.6, 0.9], 0.0))).unzip();
    for m in MODES.iter().filter(|m| !matches!(m.how, How::Dissolve { .. }) && !m.whole_frame) {
        for (got, b) in blended(m.blend, &backdrops, &sources, 1.0, 0).iter().zip(&backdrops) {
            close(*got, b.premul(), &format!("{}: a transparent layer", m.name));
        }
    }
    let (backdrops, sources): (Vec<Px>, Vec<Px>) = COLOURS.iter().map(|c| (px([0.3, 0.6, 0.9], 0.0), px(*c, 0.75))).unzip();
    for m in MODES.iter().filter(|m| !matches!(m.how, How::Dissolve { .. }) && m.group != super::Group::Matte) {
        for (got, s) in blended(m.blend, &backdrops, &sources, 1.0, 0).iter().zip(&sources) {
            close(*got, s.premul(), &format!("{}: over nothing", m.name));
        }
    }
}

#[test]
fn opacity_scales_the_layer_before_the_mode_sees_it() {
    let (below, layer) = (px([0.6; 3], 1.0), px([0.9; 3], 1.0));
    for b in [Blend::Multiply, Blend::LinearBurn, Blend::VividLight, Blend::ClassicDifference, Blend::StencilLuma] {
        let faded = blended(b, &[below], &[layer], 0.5, 0)[0];
        let by_alpha = blended(b, &[below], &[px([0.9; 3], 0.5)], 1.0, 0)[0];
        close(faded, by_alpha.map(f64::from), &format!("{b:?}: opacity 0.5 is the same as alpha 0.5"));
    }
}
