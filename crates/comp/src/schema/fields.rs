//! The keys a struct accepts, read from the struct itself.
//!
//! `#[derive(Deserialize)]` hands the field list to `Deserializer::deserialize_struct`; a deserializer that records it
//! and stops therefore reports exactly the keys the parser accepts, so an error message, a "did you mean" and the
//! catalogue never keep a second list that could drift.

use serde::{
    de::{self, Deserialize, Deserializer, Visitor},
    forward_to_deserialize_any,
};

use super::{
    kinds::{Audio, Camera, Image, Light, MeshLayer, Pack, Particles, Precomp, Solid, Text, Texture, Video},
    Common,
};

/// A struct as the parser sees it: its name in the source and the keys it accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fields {
    pub name: &'static str,
    pub keys: &'static [&'static str],
}

#[derive(Debug)]
struct Stop;

impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("stop")
    }
}
impl std::error::Error for Stop {}
impl de::Error for Stop {
    fn custom<T: std::fmt::Display>(_: T) -> Self {
        Stop
    }
}

/// What a derived `Deserialize` told the capture: a struct's keys or an enum's variants.
#[derive(Default)]
struct Capture {
    fields: Option<Fields>,
    variants: Option<Fields>,
}

impl<'de> Deserializer<'de> for &mut Capture {
    type Error = Stop;

    fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Stop> {
        Err(Stop)
    }

    fn deserialize_struct<V: Visitor<'de>>(self, name: &'static str, keys: &'static [&'static str], _: V) -> Result<V::Value, Stop> {
        self.fields = Some(Fields { name, keys });
        Err(Stop)
    }

    fn deserialize_enum<V: Visitor<'de>>(self, name: &'static str, variants: &'static [&'static str], _: V) -> Result<V::Value, Stop> {
        self.variants = Some(Fields { name, keys: variants });
        Err(Stop)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf option unit unit_struct
        newtype_struct seq tuple tuple_struct map identifier ignored_any
    }
}

/// The keys `T` accepts, when `T` is a struct with a derived `Deserialize` (a custom one has no field list).
pub fn of<'de, T: Deserialize<'de>>() -> Option<Fields> {
    let mut capture = Capture::default();
    let _ = T::deserialize(&mut capture);
    capture.fields
}

/// The variants of an enum with a derived `Deserialize`, as serde reads them.
pub fn variants_of<'de, T: Deserialize<'de>>() -> Option<Fields> {
    let mut capture = Capture::default();
    let _ = T::deserialize(&mut capture);
    capture.variants
}

/// Every key of `given` that is not in `valid`, as one message: each stray key with the nearest valid one, then the list
/// of valid keys. `None` when all are valid.
pub fn stray_keys<'a>(scope: &str, given: impl IntoIterator<Item = &'a String>, valid: &[&str]) -> Option<String> {
    let strays: Vec<String> = given.into_iter().filter(|k| !valid.contains(&k.as_str())).map(|k| format!("unknown key {k:?}.{}", turbomotion_core::suggest::hint(k, valid))).collect();
    (!strays.is_empty()).then(|| format!("{scope}: {} Valid keys: {}", strays.join(" "), valid.join(", ")))
}

/// The keys every layer accepts.
pub fn common() -> Fields {
    of::<Common>().expect("Common derives Deserialize")
}

/// The keys a layer of type `ty` accepts besides the common ones (`None` for an unknown type).
pub fn of_kind(ty: &str) -> Option<Fields> {
    const NONE: Fields = Fields { name: "", keys: &[] };
    Some(match ty {
        "solid" => of::<Solid>()?,
        "shape" => Fields { name: "Shape", keys: &["items"] },
        "text" => of::<Text>()?,
        "mesh" => of::<MeshLayer>()?,
        "image" => of::<Image>()?,
        "video" => of::<Video>()?,
        "audio" => of::<Audio>()?,
        "precomp" => of::<Precomp>()?,
        "null" | "adjustment" => NONE,
        "camera" => of::<Camera>()?,
        "light" => of::<Light>()?,
        "particles" => of::<Particles>()?,
        "texture" => of::<Texture>()?,
        "pack" => of::<Pack>()?,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Kind, COMMON_KEYS};

    #[test]
    fn every_layer_type_reports_the_keys_its_parser_accepts() {
        for ty in Kind::NAMES {
            let f = of_kind(ty).unwrap_or_else(|| panic!("{ty} has no field list"));
            let bare = matches!(*ty, "null" | "adjustment");
            assert_eq!(f.keys.is_empty(), bare, "{ty}: {f:?}");
        }
        assert!(of_kind("text").unwrap().keys.contains(&"marquee"));
        assert!(of_kind("nonsense").is_none());
        assert_eq!(variants_of::<turbomotion_core::effects::Blend>().unwrap().keys[..3], ["normal", "add", "screen"]);
    }

    #[test]
    fn every_stray_key_is_reported_with_its_nearest_valid_one() {
        let given: Vec<String> = ["dure", "fx", "zzzzzz"].iter().map(|s| s.to_string()).collect();
        let m = stray_keys("shot", &given, &["dur", "fx"]).unwrap();
        assert!(m.contains("\"dure\". Did you mean \"dur\"?") && m.contains("\"zzzzzz\".") && m.ends_with("Valid keys: dur, fx"), "{m}");
        assert!(stray_keys("shot", &given[1..2], &["dur", "fx"]).is_none());
    }

    #[test]
    fn the_common_keys_are_the_ones_the_parser_accepts() {
        let mut parsed = common().keys.to_vec();
        let mut listed = COMMON_KEYS.to_vec();
        parsed.sort_unstable();
        listed.sort_unstable();
        assert_eq!(parsed, listed);
    }
}
