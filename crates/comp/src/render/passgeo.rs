//! What a plugin pass is told about where it is drawn (`resolution`, `layer_origin`, `buffer_size`, and the shift that
//! makes `xy` a layer pixel), in screen space and in layer space (see `fxlib`'s module docs).

use skia_safe::{Matrix, Shader};
use turbomotion_core::effects::Space;

use crate::eval::fx::PluginState;

pub struct Geometry {
    /// The layer's content size in this pass's pixels: what `xy / resolution` is a fraction of.
    resolution: [f32; 2],
    /// Where the pass's top-left corner is, measured in this pass's pixels from the layer content's top-left corner:
    /// zero or negative, the room effects grew the picture by up and to the left.
    origin: [f32; 2],
    /// The pass's own size.
    buffer: [f32; 2],
    /// Layer space only: the picture is a window onto the layer, so shaders need their coordinates moved.
    windowed: bool,
}

impl Geometry {
    /// A pass `(pw, ph)` pixels big over a picture `(w, h)` pixels big that sits at `space` in the layer (none: it is the
    /// whole layer).
    pub fn of(space: Option<Space>, (w, h): (i32, i32), (pw, ph): (i32, i32)) -> Self {
        let buffer = [pw as f32, ph as f32];
        let Some(space) = space else { return Self { resolution: buffer, origin: [0.0, 0.0], buffer, windowed: false } };
        let (kx, ky) = (f64::from(pw) / f64::from(w), f64::from(ph) / f64::from(h));
        Self {
            resolution: [(space.width() * kx) as f32, (space.height() * ky) as f32],
            // `xy` counts from the content's top-left, so an image, a solid or a precomp (content at 0, 0) sees layer pixels and a
            // shape or text whose content sits elsewhere still sees `0..resolution` across its content
            origin: [((space.origin[0] - space.content[0]) * kx) as f32, ((space.origin[1] - space.content[1]) * ky) as f32],
            buffer,
            windowed: true,
        }
    }

    /// A generator painted into `size` of the canvas, whose matrix the caller has set (its `xy` is already a layer pixel).
    pub fn whole(size: [f64; 2]) -> Self {
        let size = [size[0] as f32, size[1] as f32];
        Self { resolution: size, origin: [0.0, 0.0], buffer: size, windowed: false }
    }

    /// Every value a pass is given: the plugin's parameters, the geometry and the clock.
    pub fn values(&self, p: &PluginState, t: f64) -> Vec<(String, Vec<f32>)> {
        let mut values: Vec<(String, Vec<f32>)> = p.values.clone();
        values.push(("resolution".to_string(), self.resolution.to_vec()));
        values.push(("time".to_string(), vec![t as f32]));
        values.push(("layer_origin".to_string(), self.origin.to_vec()));
        values.push(("buffer_size".to_string(), self.buffer.to_vec()));
        values
    }

    /// `m`, which carries a picture's pixels to the pass's, as a child shader needs it: the shader's coordinates are layer
    /// pixels, so the pass's own pixels are moved by the origin.
    pub fn child(&self, m: Matrix) -> Matrix {
        if self.windowed { Matrix::concat(&Matrix::translate((self.origin[0], self.origin[1])), &m) } else { m }
    }

    /// The pass's shader, drawn so that its `xy` is a layer pixel.
    pub fn shader(&self, s: Shader) -> Shader {
        if self.windowed { s.with_local_matrix(&Matrix::translate((-self.origin[0], -self.origin[1]))) } else { s }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_whole_picture_is_told_its_own_size_and_no_offset() {
        let g = Geometry::of(None, (200, 100), (100, 50));
        assert_eq!((g.resolution, g.origin, g.buffer, g.windowed), ([100.0, 50.0], [0.0, 0.0], [100.0, 50.0], false));
        let m = Matrix::scale((0.5, 0.5));
        assert_eq!(g.child(m).scale_x(), 0.5);
        assert_eq!(g.child(m).translate_x(), 0.0);
    }

    #[test]
    fn a_window_reports_the_content_size_the_origin_and_the_buffer_scaled_to_the_pass() {
        let space = Space { origin: [-30.0, -20.0], content: [0.0, 0.0, 40.0, 20.0] };
        let full = Geometry::of(Some(space), (100, 60), (100, 60));
        assert_eq!((full.resolution, full.origin, full.buffer), ([40.0, 20.0], [-30.0, -20.0], [100.0, 60.0]));
        let half = Geometry::of(Some(space), (100, 60), (50, 30));
        assert_eq!((half.resolution, half.origin, half.buffer), ([20.0, 10.0], [-15.0, -10.0], [50.0, 30.0]), "a half-size pass counts in half-size pixels");
        let m = half.child(Matrix::scale((0.5, 0.5)));
        assert_eq!((m.translate_x(), m.translate_y(), m.scale_x()), (-15.0, -10.0, 0.5), "the picture's pixel p lands at p/2 + origin");
    }

    #[test]
    fn xy_counts_from_the_contents_top_left_corner() {
        // text or a shape: the content box starts at (10, 5) in layer pixels and the picture 12 and 9 px before it
        let space = Space { origin: [-2.0, -4.0], content: [10.0, 5.0, 50.0, 25.0] };
        let g = Geometry::of(Some(space), (64, 38), (64, 38));
        assert_eq!((g.origin, g.resolution), ([-12.0, -9.0], [40.0, 20.0]));
    }

    #[test]
    fn a_generator_sees_its_size_and_the_clock() {
        let g = Geometry::whole([64.0, 32.0]);
        let p = PluginState { name: "x".into(), values: vec![("amount".into(), vec![0.5])] };
        let v = g.values(&p, 2.0);
        let get = |n: &str| v.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
        assert_eq!((get("resolution"), get("time"), get("layer_origin"), get("buffer_size"), get("amount")), (Some(vec![64.0, 32.0]), Some(vec![2.0]), Some(vec![0.0, 0.0]), Some(vec![64.0, 32.0]), Some(vec![0.5])));
    }
}
