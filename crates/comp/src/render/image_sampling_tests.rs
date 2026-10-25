//! Numeric regressions at real raster boundaries, including nested pixel-art compositions.
use super::*;
use crate::{
    plan::Plan,
    render::Renderer,
    schema::{Image as ImageDef, Precomp},
};
use serde_json::{json, Value};
use skia_safe::{png_encoder, AlphaType, ColorType, ImageInfo, Pixmap};
use std::path::Path;

#[test]
fn image_sampling_schema_is_optional_and_rejects_unknown_filters() {
    let image: ImageDef = serde_json::from_value(json!({"src":"sprite.png"})).unwrap();
    assert_eq!(image.sampling, None);
    for (name, expected) in [
        ("nearest", RasterSampling::Nearest),
        ("linear", RasterSampling::Linear),
    ] {
        let image: ImageDef =
            serde_json::from_value(json!({"src":"sprite.png","sampling":name})).unwrap();
        let precomp: Precomp =
            serde_json::from_value(json!({"comp":"sprite","sampling":name})).unwrap();
        assert_eq!(image.sampling, Some(expected));
        assert_eq!(precomp.sampling, Some(expected));
    }
    let error = serde_json::from_value::<ImageDef>(json!({"src":"sprite.png","sampling":"cubic"}))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("nearest") && error.contains("linear"),
        "{error}"
    );
    let precomp: Precomp = serde_json::from_value(json!({"comp":"sprite"})).unwrap();
    assert_eq!(precomp.sampling, None);
}

#[test]
fn image_sampling_default_is_adaptive_and_explicit_filters_ignore_scale() {
    let mut surface = skia_safe::surfaces::raster_n32_premul((8, 4)).unwrap();
    let canvas = surface.canvas();
    let view = View {
        src: [0., 0., 2., 1.],
        dest: [0., 0., 8., 4.],
        viewport: [8., 4.],
    };
    assert!(sampling(canvas, &view, None).use_cubic);
    canvas.scale((0.1, 0.1));
    assert_eq!(
        sampling(canvas, &view, None),
        SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear)
    );
    assert_eq!(
        sampling(canvas, &view, Some(RasterSampling::Nearest)),
        SamplingOptions::new(FilterMode::Nearest, MipmapMode::None)
    );
    assert_eq!(
        sampling(canvas, &view, Some(RasterSampling::Linear)),
        SamplingOptions::new(FilterMode::Linear, MipmapMode::None)
    );
}

fn fixture() -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("turbomotion-image-sampling-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let info = ImageInfo::new((2, 1), ColorType::RGBA8888, AlphaType::Premul, None);
    let mut pixels = [255u8, 0, 0, 255, 0, 0, 255, 255];
    let pixmap = Pixmap::new(&info, &mut pixels, 8).unwrap();
    let mut png = vec![];
    assert!(png_encoder::encode(
        &pixmap,
        &mut png,
        &png_encoder::Options::default()
    ));
    std::fs::write(dir.join("sprite.png"), png).unwrap();
    dir
}

fn render(value: Value, dir: &Path) -> Vec<u8> {
    let plan = Plan::from_value(value, dir).unwrap();
    let mut renderer = Renderer::new(&plan).unwrap();
    renderer.draw(0).unwrap();
    let mut pixels = vec![];
    renderer.pixels(&mut pixels).unwrap();
    pixels
}

fn assert_crisp(pixels: &[u8]) {
    for y in 0..4 {
        for x in 0..8 {
            let offset = (y * 8 + x) * 4;
            let expected = if x < 4 {
                [0, 0, 255, 255]
            } else {
                [255, 0, 0, 255]
            };
            assert_eq!(&pixels[offset..offset + 4], &expected, "BGRA at ({x}, {y})");
        }
    }
}

#[test]
fn image_sampling_nearest_preserves_exact_pixels_directly_and_through_scaled_precomp() {
    let dir = fixture();
    let mut direct = json!({"width":8,"height":4,"dur":1,"layers":[
        {"type":"image","src":"sprite.png","size":[8,4],"fit":"fill","pos":[4,2],"sampling":"nearest"}
    ]});
    assert_crisp(&render(direct.clone(), &dir));
    direct["layers"][0]["sampling"] = json!("linear");
    let linear = render(direct, &dir);
    assert!(
        linear[12] > 0 && linear[14] > 0,
        "linear boundary should contain both red and blue"
    );

    let mut nested = json!({"width":8,"height":4,"dur":1,"comps":{
        "sprite":{"width":2,"height":1,"dur":1,"layers":[
            {"type":"image","src":"sprite.png","fit":"none","pos":[1,0.5],"sampling":"nearest"}
        ]}
    },"layers":[{"type":"precomp","comp":"sprite","pos":[4,2],"scale":4,"sampling":"nearest"}]});
    assert_crisp(&render(nested.clone(), &dir));
    nested["layers"][0]
        .as_object_mut()
        .unwrap()
        .remove("sampling");
    let linear = render(nested, &dir);
    assert!(
        linear[12] > 0 && linear[14] > 0,
        "default precomp filtering must stay linear"
    );
}
