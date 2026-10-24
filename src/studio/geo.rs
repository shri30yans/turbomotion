//! Geography: the country and state outlines (`geo.json`, simplified from Natural Earth with India shown inside its
//! official boundary) and the projections that put them on screen. Pure data and arithmetic; drawing lives in `atlas`.

use std::{collections::HashMap, fs, path::Path};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

use super::model::{ViewKey, H, W};

/// A closed outline of (longitude, latitude) degrees.
pub type Ring = Vec<[f32; 2]>;

#[derive(Deserialize)]
pub struct Geo {
    countries: HashMap<String, Vec<Ring>>,
    states: HashMap<String, Vec<Ring>>,
}

impl Geo {
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join("geo.json");
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    /// Every outline of every country in the list.
    pub fn country_rings<'a>(&'a self, names: &[String]) -> Result<Vec<&'a Ring>> {
        Self::rings_of(&self.countries, names, "country")
    }

    /// Every outline of every US state in the list.
    pub fn state_rings<'a>(&'a self, names: &[String]) -> Result<Vec<&'a Ring>> {
        Self::rings_of(&self.states, names, "state")
    }

    /// Every outline of every country in the atlas.
    pub fn all_country_rings(&self) -> impl Iterator<Item = &Ring> {
        self.countries.values().flatten()
    }

    fn rings_of<'a>(table: &'a HashMap<String, Vec<Ring>>, names: &[String], what: &str) -> Result<Vec<&'a Ring>> {
        let mut out = Vec::new();
        for name in names {
            out.extend(table.get(name).ok_or_else(|| anyhow!("unknown {what} {name:?}"))?);
        }
        Ok(out)
    }
}

/// Mercator `y` of a latitude in degrees (clamped short of the poles).
fn mercator_y(lat_deg: f64) -> f64 {
    (std::f64::consts::FRAC_PI_4 + lat_deg.clamp(-84.0, 84.0).to_radians() / 2.0).tan().ln()
}

/// A frame-sized Mercator window: centre longitude/latitude and the horizontal span in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    lon: f64,
    span: f64,
    lat: f64,
    /// Pixels per degree of longitude.
    kx: f64,
    /// Pixels per radian of Mercator `y`.
    k: f64,
    cy: f64,
}

impl View {
    pub fn new([lon, lat, span]: ViewKey) -> Self {
        let kx = f64::from(W) / span;
        Self { lon, lat, span, kx, k: kx.to_degrees(), cy: mercator_y(lat) }
    }

    /// The view `u` of the way from `a` to `b`: centre moves linearly, the span geometrically.
    pub fn between(a: ViewKey, b: ViewKey, u: f64) -> ViewKey {
        [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u, (a[2].ln() + (b[2].ln() - a[2].ln()) * u).exp()]
    }

    pub fn key(&self) -> ViewKey {
        [self.lon, self.lat, self.span]
    }

    /// Screen position of a place.
    pub fn xy(&self, lon: f64, lat: f64) -> (f64, f64) {
        (f64::from(W) / 2.0 + (lon - self.lon) * self.kx, f64::from(H) / 2.0 - (mercator_y(lat) - self.cy) * self.k)
    }

    /// Screen position of an outline vertex (latitude held within +-1.4 rad as the reference does for outlines).
    pub fn vertex(&self, [lon, lat]: [f32; 2]) -> (f32, f32) {
        let y = (std::f64::consts::FRAC_PI_4 + f64::from(lat).to_radians().clamp(-1.4, 1.4) / 2.0).tan().ln();
        ((f64::from(W) / 2.0 + (f64::from(lon) - self.lon) * self.kx) as f32, (f64::from(H) / 2.0 - (y - self.cy) * self.k) as f32)
    }
}

/// A point on a great-circle route: position and the height factor of the lifted arc above the surface.
#[derive(Debug, Clone, Copy)]
pub struct RoutePoint {
    pub lon: f64,
    pub lat: f64,
    pub height: f64,
}

/// `n + 1` points along the great circle from `a` to `b` (longitude, latitude degrees); the height peaks at `lift` halfway.
pub fn great_circle(a: [f64; 2], b: [f64; 2], n: usize, lift: f64) -> Vec<RoutePoint> {
    let unit = |[lon, lat]: [f64; 2]| {
        let (la, lo) = (lat.to_radians(), lon.to_radians());
        [la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()]
    };
    let (v1, v2) = (unit(a), unit(b));
    let omega = (v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2]).clamp(-1.0, 1.0).acos();
    (0..=n)
        .map(|i| {
            let f = i as f64 / n as f64;
            let (w1, w2) = (((1.0 - f) * omega).sin() / omega.sin().max(1e-9), (f * omega).sin() / omega.sin().max(1e-9));
            let v: Vec<f64> = (0..3).map(|k| w1 * v1[k] + w2 * v2[k]).collect();
            RoutePoint { lon: v[1].atan2(v[0]).to_degrees(), lat: v[2].asin().to_degrees(), height: lift * (std::f64::consts::PI * f).sin() }
        })
        .collect()
}

/// An orthographic globe of diameter `d` pixels turned to face (`lon`, `lat`).
#[derive(Debug, Clone, Copy)]
pub struct Globe {
    pub d: f64,
    pub lon: f64,
    pub lat: f64,
}

impl Globe {
    /// Pixel position inside the globe's square layer of a point `height` above the surface, and whether it is on the
    /// visible hemisphere (just over the limb counts for lifted points).
    pub fn project(&self, lon: f64, lat: f64, height: f64) -> (f64, f64, bool) {
        let (la0, lo0) = (self.lat.to_radians(), self.lon.to_radians());
        let (la, lo) = (lat.to_radians(), lon.to_radians() - lo0);
        let (wx, wy, wz) = (la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin());
        let z = wx * la0.cos() + wz * la0.sin();
        let y = -wx * la0.sin() + wz * la0.cos();
        let k = 1.0 + height;
        let visible = (z > 0.02 || (height > 0.0 && z * k > 0.0)) && z > -0.05;
        (self.d / 2.0 + wy * k * self.d / 2.0, self.d / 2.0 - y * k * self.d / 2.0, visible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_centre_is_frame_centre() {
        let v = View::new([77.0, 20.0, 40.0]);
        let (x, y) = v.xy(77.0, 20.0);
        assert!((x - 960.0).abs() < 1e-9 && (y - 540.0).abs() < 1e-9);
        assert!((v.xy(97.0, 20.0).0 - (960.0 + 20.0 * 1920.0 / 40.0)).abs() < 1e-9);
    }

    #[test]
    fn great_circle_runs_between_its_ends_and_lifts_in_the_middle() {
        let route = great_circle([0.0, 0.0], [90.0, 0.0], 8, 0.3);
        assert_eq!(route.len(), 9);
        assert!((route[0].lon).abs() < 1e-9 && (route[8].lon - 90.0).abs() < 1e-9);
        assert!((route[4].lon - 45.0).abs() < 1e-9 && (route[4].height - 0.3).abs() < 1e-12);
    }

    #[test]
    fn globe_centre_faces_the_viewer() {
        let g = Globe { d: 1000.0, lon: 10.0, lat: 20.0 };
        let (x, y, visible) = g.project(10.0, 20.0, 0.0);
        assert!((x - 500.0).abs() < 1e-9 && (y - 500.0).abs() < 1e-9 && visible);
        assert!(!g.project(190.0, -20.0, 0.0).2);
    }
}
