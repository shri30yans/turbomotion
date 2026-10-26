//! Reading a list of keys: `[seconds, value, ease?]` or an object with every field of an After Effects keyframe.

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::{Map, Value as Json};
use turbomotion_core::suggest::hint;

use super::{
    key::{Key, Rove, Tangents},
    parse::{num, VECTOR_FORMS},
    parse_side::{join, side, tangents},
    value::Kind,
};
use crate::anim::ease::Ease;

/// The fields a key object accepts.
pub const KEY_FIELDS: &[&str] = &["t", "v", "ease", "ease_in", "ease_out", "continuous", "spatial", "in", "out", "roving"];
const PATH_FIELDS: &[&str] = &["spatial", "in", "out", "roving"];

/// What keys are read as.
pub(super) struct Reader {
    pub kind: Kind,
    /// The handles of a key of a motion path that gives none; none when the property has no motion path.
    pub path: Option<Tangents>,
}

/// One entry of a key list: a fixed key, or a roving point that belongs to the fixed key before it.
enum Entry {
    Fixed(Key),
    Rove(Rove),
}

impl Reader {
    pub fn keys(&self, v: &Json) -> Result<Vec<Key>> {
        let list = v.as_array().ok_or_else(|| anyhow!("keys must be a list like [[0, 0], [1, 100]]"))?;
        ensure!(!list.is_empty(), "keys must not be empty");
        let mut keys: Vec<Key> = Vec::with_capacity(list.len());
        for (i, item) in list.iter().enumerate() {
            match self.entry(item).with_context(|| format!("key {i}"))? {
                Entry::Fixed(key) => {
                    if let Some(prev) = keys.last() {
                        ensure!(key.t >= prev.t, "key {i} is at {}s but the key before it is at {}s: keys must be in time order", key.t, prev.t);
                    }
                    keys.push(key);
                }
                Entry::Rove(r) => keys.last_mut().ok_or_else(|| anyhow!("key {i} is roving, but a roving key follows a key with a time; the first key needs a \"t\""))?.rove.push(r),
            }
        }
        Ok(keys)
    }

    fn entry(&self, item: &Json) -> Result<Entry> {
        match item {
            Json::Array(a) => {
                ensure!((2..=3).contains(&a.len()), "a key is [seconds, value] or [seconds, value, ease]");
                if self.kind == Kind::Scalar {
                    ensure!(!a[1].is_array(), "a key's value here is one number, not {}. A vector animates as {VECTOR_FORMS}", a[1]);
                }
                let ease = a.get(2).map(|e| Ease::parse(e.as_str().ok_or_else(|| anyhow!("the ease of a key is text like \"expo.out\""))?)).transpose()?;
                let mut key = Key::new(num("a key's time", &a[0])?, self.kind.read(&a[1])?);
                key.ease = ease;
                Ok(Entry::Fixed(key))
            }
            Json::Object(o) => self.object(o),
            other => bail!("a key is [seconds, value, ease?], not {other}"),
        }
    }

    fn object(&self, o: &Map<String, Json>) -> Result<Entry> {
        if !o.contains_key("t") && !o.contains_key("roving") && ["from", "to", "dur", "at"].iter().any(|k| o.contains_key(*k)) {
            bail!("keys are [seconds, value, ease?] (or {{\"t\", \"v\"}}); this looks like a tween object, and a list of tweens is not keys. Use one tween per property ({{\"from\": 0, \"to\": 1, \"at\": 0.5, \"dur\": 0.4}}), or keys such as [[0, 0], [0.5, 1, \"expo.out\"]]");
        }
        if let Some(bad) = o.keys().find(|k| !KEY_FIELDS.contains(&k.as_str())) {
            bail!("unknown key field {bad:?}; a key has {}.{}", KEY_FIELDS.join(", "), hint(bad, KEY_FIELDS));
        }
        let path_field = PATH_FIELDS.iter().find(|f| o.contains_key(**f));
        let default = match (&self.path, path_field) {
            (Some(d), _) => d,
            (None, Some(f)) => bail!("{f:?} belongs to the motion path of a position; this property has none. Give a position, or set \"spatial\" on the property"),
            (None, None) => &Tangents::Auto,
        };
        let value = self.kind.read(o.get("v").ok_or_else(|| anyhow!("key needs \"v\""))?).context("\"v\"")?;
        let roving = match o.get("roving") {
            None => false,
            Some(r) => r.as_bool().ok_or_else(|| anyhow!("\"roving\" is true or false, not {r}"))?,
        };
        if roving {
            for f in ["t", "ease", "ease_in", "ease_out", "continuous"] {
                ensure!(!o.contains_key(f), "a roving key has no time, so it has no {f:?}: its timing comes from the keys around it");
            }
            let at = value.parts();
            return Ok(Entry::Rove(Rove { at: [at.v[0], at.v[1], at.v[2]], tangents: tangents(o, self.kind, default)? }));
        }
        let mut key = Key::new(num("\"t\"", o.get("t").ok_or_else(|| anyhow!("key needs \"t\""))?)?, value);
        key.ease = o.get("ease").map(|e| Ease::parse(e.as_str().unwrap_or(""))).transpose()?;
        key.ease_in = o.get("ease_in").map(|s| side("ease_in", s)).transpose()?;
        key.ease_out = o.get("ease_out").map(|s| side("ease_out", s)).transpose()?;
        let continuous = match o.get("continuous") {
            None => false,
            Some(c) => c.as_bool().ok_or_else(|| anyhow!("\"continuous\" is true or false, not {c}"))?,
        };
        join(&mut key.ease_in, &mut key.ease_out, continuous)?;
        if self.path.is_some() {
            key.tangents = tangents(o, self.kind, default)?;
        }
        Ok(Entry::Fixed(key))
    }
}
