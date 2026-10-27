//! What a kernel is made of, in pixels: how many taps each side of a pixel it reads and how far that reaches. Everything
//! here is plain arithmetic on the parameters, so the planner (a layer's buffer growth) and the shader passes agree by
//! construction: both read these functions and nothing else.

/// A Gaussian is cut at this many standard deviations. Past four sigma 6.3e-5 of its weight is left (3.2e-5 each side: under
/// a hundredth of an 8-bit level on a black and white edge); the weights are renormalised, so no light is lost either way.
pub const REACH: f64 = 4.0;

/// Taps each side of a pixel a separable pass can read. The loop bound of `separable.sksl` is written from this.
pub const MAX_TAPS: u32 = 2048;

/// Taps each side of a pixel the iris pass can read (it reads the square of them). The loop bound of `iris.sksl`.
pub const MAX_IRIS_TAPS: u32 = 32;

/// Samples along a smear a pass can take. The loop bound of `smear.sksl`.
pub const MAX_SMEAR_TAPS: u32 = 2048;

/// A sigma below this has a second tap lighter than `e^-200`: there is nothing to blur.
pub const MIN_SIGMA: f64 = 0.05;

/// Taps each side of the centre of a Gaussian of `sigma`; none when it has nothing to blur.
pub fn gaussian_taps(sigma: f64) -> u32 {
    if sigma < MIN_SIGMA {
        0
    } else {
        (REACH * sigma).ceil() as u32
    }
}

/// `1 / (2 sigma^2)`, the Gaussian's falloff per squared pixel.
pub fn falloff(sigma: f64) -> f64 {
    1.0 / (2.0 * sigma * sigma)
}

/// A box of `radius` pixels covers `2 radius + 1` pixels' worth: whole taps out to `radius.floor()` and the remainder as
/// one lighter tap beyond them, so the radius is continuous and animating it does not step.
pub struct BoxTaps {
    /// Taps each side of the centre, the lighter one included.
    pub count: u32,
    /// The weight of the outermost tap: 1 for a whole radius, else the remainder.
    pub last: f64,
}

pub fn box_taps(radius: f64) -> BoxTaps {
    if radius <= 0.0 {
        return BoxTaps { count: 0, last: 1.0 };
    }
    let count = radius.ceil() as u32;
    BoxTaps { count, last: radius - f64::from(count - 1) }
}

/// Taps each side of an iris of `radius` pixels: its soft edge reaches half a pixel past the radius.
pub fn iris_taps(radius: f64) -> u32 {
    if radius <= 0.0 {
        0
    } else {
        (radius + 0.5).ceil() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gaussian_reads_four_sigma_each_side_and_nothing_when_it_has_no_width() {
        assert_eq!([1.0, 2.0, 16.0, 64.0].map(gaussian_taps), [4, 8, 64, 256]);
        assert_eq!(gaussian_taps(0.0), 0);
        assert_eq!(gaussian_taps(MIN_SIGMA / 2.0), 0);
        assert_eq!(gaussian_taps(2.1), 9, "a fractional reach rounds up so the cut is never nearer than four sigma");
    }

    #[test]
    fn the_cut_leaves_less_than_a_hundredth_of_a_level_on_an_edge() {
        // the tail of a normal beyond REACH sigma on one side, from the complementary error function's series for large x
        let x = REACH / std::f64::consts::SQRT_2;
        let tail = 0.5 * (-x * x).exp() / (x * std::f64::consts::PI.sqrt()) * (1.0 - 1.0 / (2.0 * x * x));
        assert!(tail * 255.0 < 0.01, "{tail}");
    }

    #[test]
    fn a_fractional_box_radius_adds_one_light_tap_so_the_width_is_continuous() {
        let whole = box_taps(3.0);
        assert_eq!((whole.count, whole.last), (3, 1.0));
        let part = box_taps(3.25);
        assert_eq!((part.count, part.last), (4, 0.25));
        // the pixels covered: the centre, whole taps both sides and the light tap both sides
        let width = |b: BoxTaps| 1.0 + 2.0 * f64::from(b.count - 1) + 2.0 * b.last;
        assert_eq!([width(whole), width(part)], [7.0, 7.5]);
        assert_eq!(box_taps(0.0).count, 0);
    }

    #[test]
    fn an_iris_reaches_half_a_pixel_past_its_radius() {
        assert_eq!([0.0, 0.4, 1.0, 10.0, 10.6].map(iris_taps), [0, 1, 2, 11, 12]);
    }
}
