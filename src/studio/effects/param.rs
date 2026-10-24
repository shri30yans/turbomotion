//! Effect parameters: a plain number, or keys `[[seconds, value], ...]` it is animated through (held before the
//! first key and after the last, smooth between).

use std::fmt;

use serde::{de, Deserialize, Deserializer};

use crate::studio::ease::keyed;

#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    Fixed(f64),
    Keys(Vec<(f64, f64)>),
}

impl Param {
    pub fn at(&self, t: f64) -> f64 {
        match self {
            Self::Fixed(v) => *v,
            Self::Keys(keys) => keyed(keys, t),
        }
    }
}

/// The value of an optional parameter at `t`, or `default` when the plan leaves it out.
pub fn value(param: &Option<Param>, t: f64, default: f64) -> f64 {
    param.as_ref().map_or(default, |p| p.at(t))
}

impl<'de> Deserialize<'de> for Param {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;
        const EXPECTED: &str = "a number, or a list of [seconds, value] keys";

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Param;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(EXPECTED)
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Param, E> {
                Ok(Param::Fixed(v))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Param, E> {
                Ok(Param::Fixed(v as f64))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Param, E> {
                Ok(Param::Fixed(v as f64))
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Param, A::Error> {
                let mut keys: Vec<(f64, f64)> = Vec::new();
                while let Some(key) = seq.next_element::<(f64, f64)>()? {
                    if keys.last().is_some_and(|last| key.0 < last.0) {
                        return Err(de::Error::custom("animation keys must be in time order"));
                    }
                    keys.push(key);
                }
                if keys.is_empty() {
                    return Err(de::Error::invalid_length(0, &EXPECTED));
                }
                Ok(Param::Keys(keys))
            }
        }
        de.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_keys_parse_and_animate() {
        let fixed: Param = serde_json::from_str("12").unwrap();
        assert_eq!(fixed.at(5.0), 12.0);
        let keys: Param = serde_json::from_str("[[1, 0], [3, 10]]").unwrap();
        assert_eq!((keys.at(0.0), keys.at(2.0), keys.at(9.0)), (0.0, 5.0, 10.0));
    }

    #[test]
    fn bad_parameters_say_what_is_expected() {
        for bad in ["\"big\"", "[]", "[[2, 1], [1, 2]]", "[1, 2]"] {
            assert!(serde_json::from_str::<Param>(bad).is_err(), "{bad}");
        }
    }
}
