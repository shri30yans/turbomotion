//! Natural Earth outlines (`assets/maps/geo.json`): countries and states as polygons, rivers as lines.

use std::{fmt, fs, marker::PhantomData, path::Path};

use anyhow::{Context, Result};
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Deserializer,
};

pub type Ring = Vec<[f64; 2]>;

/// A JSON object read as a list, keeping the file's key order (it is the paint order).
#[derive(Debug)]
pub struct Named<T>(pub Vec<(String, T)>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Named<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Entries<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for Entries<T> {
            type Value = Named<T>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Named<T>, A::Error> {
                let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
                while let Some(entry) = map.next_entry()? {
                    entries.push(entry);
                }
                Ok(Named(entries))
            }
        }
        deserializer.deserialize_map(Entries(PhantomData))
    }
}

#[derive(Debug, Deserialize)]
pub struct Geo {
    pub countries: Named<Vec<Ring>>,
    pub states: Named<Vec<Ring>>,
    pub rivers: Named<Vec<Ring>>,
}

impl Geo {
    pub fn load(assets: &Path) -> Result<Self> {
        let path = assets.join("maps/geo.json");
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_file_order() {
        let named: Named<u8> = serde_json::from_str(r#"{"b": 1, "a": 2, "c": 3}"#).unwrap();
        assert_eq!(named.0.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(), ["b", "a", "c"]);
    }

    #[test]
    fn the_bundled_geometry_parses() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let geo = Geo::load(&assets).unwrap();
        assert!(geo.countries.0.iter().any(|(n, _)| n == "India"));
    }
}
