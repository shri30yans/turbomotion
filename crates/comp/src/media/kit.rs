//! The built-in sound kit: families of procedurally synthesised sounds (`assets/sound/kit/<family>/<name>.wav`), their
//! index (compiled in), and how a cue such as `kit:whoosh`, `kit:whoosh/airy-03` or `kit:pluck/C#4` becomes one file.
//!
//! * `kit:<family>` is any variant of the family. Which one is decided by [`Kit::pick`] from a running order (the
//!   n-th use of that cue, by time) and a seed: the variants are put in a seeded shuffle and used round-robin, so two
//!   neighbouring hits never get the same file when the family has two or more. A spread pick also gets a small
//!   deterministic pitch and gain variation (per family; none for pitched voices).
//! * A family may restrict what a bare `kit:<family>` draws from (a plain `riser` is a short build, so a hit placed
//!   ahead of a moment still lands on it); every variant stays reachable by name or style.
//! * `kit:<family>/<name>` is exactly that variant. `kit:<family>/<style>` (e.g. `whoosh/airy`) is the pool of
//!   variants named `<style>-NN`. `kit:<voice>/<note>` (e.g. `pluck/C4`) is a pitched voice at that note, played from
//!   the nearest rendered root at the exact rate that tunes it.
//! * The 15 original cue names and a few curated pools (`fast-pass`, `soft-pass`) are aliases for pools.
//!
//! The index is embedded; the WAV files are read from the compiled path (the way the other built-in assets are).

use std::{collections::BTreeMap, path::PathBuf, sync::OnceLock};

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};

const INDEX: &str = include_str!("../../assets/sound/kit/index.json");

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub dur: f64,
    /// Seconds to the loudest moment: where it should land on a transition's midpoint.
    pub peak: f64,
    pub desc: String,
    pub tags: Vec<String>,
    /// For pitched voices: the MIDI note this file is tuned to.
    pub midi: Option<i32>,
}

#[derive(Debug)]
pub struct Family {
    pub name: String,
    pub desc: String,
    /// Spread variation: pitch (+/- semitones) and gain (+/- dB). Zero for tonal sounds.
    pub jitter: (f64, f64),
    /// Pitched voices: the lowest and highest MIDI note they can play.
    pub range: Option<(i32, i32)>,
    /// Names or styles (`quick` means `quick-NN`) that a bare `kit:<family>` draws from; empty means every variant.
    pub default_pool: Vec<String>,
    pub variants: Vec<Variant>,
}

#[derive(Debug)]
pub struct Kit {
    pub families: BTreeMap<String, Family>,
    pub aliases: BTreeMap<String, Vec<String>>,
}

/// One resolved sound: the file, when it is loudest, and the playback rate and gain it needs.
#[derive(Debug, Clone, PartialEq)]
pub struct Pick {
    /// `family/name`: the exact cue that reproduces this pick.
    pub cue: String,
    pub path: PathBuf,
    pub dur: f64,
    pub peak: f64,
    /// Playback speed (1 = as rendered); changes pitch and length together.
    pub rate: f64,
    /// Linear gain applied on top of the layer's volume.
    pub gain: f64,
}

struct Cand<'a> {
    fam: &'a Family,
    var: &'a Variant,
    /// Set when the cue named this variant (or a note) outright: no spread variation.
    exact: bool,
    rate: f64,
}

pub fn kit() -> &'static Kit {
    static K: OnceLock<Kit> = OnceLock::new();
    K.get_or_init(|| Kit::parse(INDEX).expect("assets/sound/kit/index.json is valid (rebuild it with `python3 -m sfx.build`)"))
}

pub fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/sound/kit")
}

fn hash(parts: &[&str], nums: &[u64]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in parts {
        for b in p.bytes().chain([0xff]) {
            h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
        }
    }
    for n in nums {
        h = (h ^ n).wrapping_mul(0x100_0000_01b3);
        h ^= h >> 29;
    }
    // splitmix finaliser so nearby inputs give unrelated outputs
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    h ^ (h >> 31)
}

fn unit(h: u64) -> f64 {
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// `"C4"`, `"f#3"`, `"Bb2"` to a MIDI number (C4 = 60). `None` if it is not a note name.
pub fn parse_note(s: &str) -> Option<i32> {
    let mut ch = s.chars();
    let base = match ch.next()?.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest: String = ch.collect();
    let digits_at = rest.find(|c: char| c.is_ascii_digit() || c == '-')?;
    let (acc, oct) = rest.split_at(digits_at);
    let mut pc = base;
    for a in acc.chars() {
        pc += match a {
            '#' => 1,
            'b' => -1,
            _ => return None,
        };
    }
    Some(pc + 12 * (oct.parse::<i32>().ok()? + 1))
}

pub fn note_name(m: i32) -> String {
    format!("{}{}", ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"][m.rem_euclid(12) as usize], m.div_euclid(12) - 1)
}

impl Kit {
    pub fn parse(text: &str) -> Result<Kit> {
        let v: Value = serde_json::from_str(text)?;
        let mut families = BTreeMap::new();
        for (name, f) in v["families"].as_object().ok_or_else(|| anyhow!("index has no families"))? {
            let range = f.get("pitched").map(|p| (p["lo"].as_i64().unwrap_or(0) as i32, p["hi"].as_i64().unwrap_or(0) as i32));
            let variants = f["variants"]
                .as_object()
                .ok_or_else(|| anyhow!("family {name} has no variants"))?
                .iter()
                .map(|(n, e)| Variant {
                    name: n.clone(),
                    dur: e["dur"].as_f64().unwrap_or(0.0),
                    peak: e["peak"].as_f64().unwrap_or(0.0),
                    desc: e["desc"].as_str().unwrap_or("").to_string(),
                    tags: e["tags"].as_array().map(|t| t.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default(),
                    midi: if range.is_some() { parse_note(n) } else { None },
                })
                .collect();
            families.insert(name.clone(), Family { name: name.clone(), desc: f["desc"].as_str().unwrap_or("").to_string(), jitter: (f["jitter"]["pitch"].as_f64().unwrap_or(0.0), f["jitter"]["gain"].as_f64().unwrap_or(0.0)), range, default_pool: f["default"].as_array().map(|d| d.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default(), variants });
        }
        let aliases = v["aliases"]
            .as_object()
            .map(|a| a.iter().map(|(k, l)| (k.clone(), l.as_array().map(|l| l.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default())).collect())
            .unwrap_or_default();
        Ok(Kit { families, aliases })
    }

    /// The names a film can write after `kit:` without a slash.
    pub fn cues(&self) -> Vec<String> {
        self.families.keys().chain(self.aliases.keys()).cloned().collect()
    }

    fn unknown(&self, cue: &str) -> anyhow::Error {
        anyhow!(
            "{cue:?} is not a sound-kit cue. Families: {}. Pools and older names: {}. Use `kit:<family>` (varies), `kit:<family>/<variant>` (exact), `kit:<voice>/<note>` (e.g. kit:pluck/C4)",
            self.families.keys().cloned().collect::<Vec<_>>().join(", "),
            self.aliases.keys().cloned().collect::<Vec<_>>().join(", ")
        )
    }

    fn candidates(&self, cue: &str, depth: usize) -> Result<Vec<Cand<'_>>> {
        let (head, tail) = cue.split_once('/').map_or((cue, None), |(a, b)| (a, Some(b)));
        let Some(fam) = self.families.get(head) else {
            let refs = self.aliases.get(cue).filter(|_| depth == 0).ok_or_else(|| self.unknown(cue))?;
            return refs.iter().map(|r| self.candidates(r, depth + 1)).collect::<Result<Vec<_>>>().map(|v| v.into_iter().flatten().collect());
        };
        let Some(tail) = tail else {
            let pool: Vec<Cand> = fam.variants.iter().filter(|v| fam.default_pool.is_empty() || fam.default_pool.iter().any(|d| v.name == *d || v.name.starts_with(&format!("{d}-")))).map(|var| Cand { fam, var, exact: false, rate: 1.0 }).collect();
            return Ok(pool);
        };
        if let Some(var) = fam.variants.iter().find(|v| v.name == tail) {
            return Ok(vec![Cand { fam, var, exact: true, rate: 1.0 }]);
        }
        if let (Some((lo, hi)), Some(m)) = (fam.range, parse_note(tail)) {
            if m < lo || m > hi {
                bail!("{cue:?}: {head} plays {} to {}", note_name(lo), note_name(hi));
            }
            let root = fam.variants.iter().filter(|v| v.midi.is_some()).min_by_key(|v| (v.midi.unwrap_or(0) - m).abs()).ok_or_else(|| anyhow!("{head} has no pitched variants"))?;
            return Ok(vec![Cand { fam, var: root, exact: true, rate: 2f64.powf(f64::from(m - root.midi.unwrap_or(m)) / 12.0) }]);
        }
        let pool: Vec<Cand> = fam.variants.iter().filter(|v| v.name.starts_with(&format!("{tail}-"))).map(|var| Cand { fam, var, exact: false, rate: 1.0 }).collect();
        if pool.is_empty() {
            let names: Vec<&str> = fam.variants.iter().map(|v| v.name.as_str()).collect();
            bail!("{cue:?}: {head} has no variant or style {tail:?}; variants are {}{}", names.join(", "), fam.range.map_or(String::new(), |(a, b)| format!(" (or any note from {} to {})", note_name(a), note_name(b))));
        }
        Ok(pool)
    }

    /// Resolve `cue` (the text after `kit:`) for the `order`-th use of that cue; `seed` separates compositions.
    ///
    /// Deterministic: the same cue, order and seed always give the same file, rate and gain. For a pool, uses
    /// `order`, `order + 1`, ... walk a seeded shuffle of the pool, so consecutive picks differ.
    pub fn pick(&self, cue: &str, order: usize, seed: u64) -> Result<Pick> {
        let pool = self.candidates(cue, 0)?;
        let n = pool.len();
        anyhow::ensure!(n > 0, "{cue:?} has no sounds");
        let mut perm: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            perm.swap(i, (hash(&[cue], &[seed, i as u64]) % (i as u64 + 1)) as usize);
        }
        let c = &pool[perm[order % n]];
        let h = hash(&[cue], &[seed, order as u64, 77]);
        let (semis, gain_db) = if c.exact || n == 1 { (0.0, 0.0) } else { ((unit(h) * 2.0 - 1.0) * c.fam.jitter.0, (unit(h.rotate_left(21)) * 2.0 - 1.0) * c.fam.jitter.1) };
        let rate = c.rate * 2f64.powf(semis / 12.0);
        Ok(Pick { cue: format!("{}/{}", c.fam.name, c.var.name), path: dir().join(&c.fam.name).join(format!("{}.wav", c.var.name)), dur: c.var.dur / rate, peak: c.var.peak / rate, rate, gain: 10f64.powf(gain_db / 20.0) })
    }

    /// For the catalogue: every family with its variants, and the aliases.
    pub fn catalogue(&self) -> Value {
        let cues: Vec<Value> = self.families.values().map(|f| json!({"cue": f.name, "timbre": f.desc})).chain(self.aliases.iter().map(|(a, r)| json!({"cue": a, "timbre": format!("a pool: {}", r.join(", "))}))).collect();
        let fams: BTreeMap<&String, Value> = self
            .families
            .iter()
            .map(|(n, f)| {
                let v: Vec<Value> = f.variants.iter().map(|v| json!({"name": v.name, "dur": v.dur, "peak": v.peak, "desc": v.desc})).collect();
                (n, json!({"summary": f.desc, "range": f.range.map(|(a, b)| format!("{} to {}", note_name(a), note_name(b))), "variants": v}))
            })
            .collect();
        json!({
            "src": "kit:<cue>",
            "summary": format!("{} procedurally synthesised sounds in {} families. `kit:whoosh` varies by itself (a different file for each use, with a touch of pitch and gain); `kit:whoosh/airy-01` is exactly that file; `kit:whoosh/airy` is the airy ones; `kit:pluck/C4` is a note.", self.families.values().map(|f| f.variants.len()).sum::<usize>(), self.families.len()),
            "cues": cues,
            "families": fams,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families_variants_pools_and_notes_resolve() {
        let k = kit();
        assert!(k.families.len() >= 20 && k.families.values().map(|f| f.variants.len()).sum::<usize>() >= 150);
        let exact = k.pick("whoosh/airy-01", 0, 1).unwrap();
        assert_eq!((exact.cue.as_str(), exact.rate, exact.gain), ("whoosh/airy-01", 1.0, 1.0));
        assert!(exact.path.exists(), "{}", exact.path.display());
        for i in 0..8 {
            assert!(k.pick("whoosh/airy", i, 3).unwrap().cue.starts_with("whoosh/airy-"));
        }
        let c4 = k.pick("pluck/C4", 0, 9).unwrap();
        assert_eq!((c4.cue.as_str(), c4.rate), ("pluck/C4", 1.0));
        let d4 = k.pick("pluck/D4", 0, 9).unwrap();
        assert_eq!(d4.cue, "pluck/D#4");
        assert!((d4.rate - 2f64.powf(-1.0 / 12.0)).abs() < 1e-9);
        assert!(k.pick("kit-less", 0, 0).unwrap_err().to_string().contains("whoosh"));
        assert!(k.pick("pluck/C9", 0, 0).unwrap_err().to_string().contains("C3 to C5"));
        assert!(k.pick("whoosh/nothing", 0, 0).unwrap_err().to_string().contains("airy-01"));
        assert!(k.pick("boom", 0, 0).unwrap().path.exists(), "old names still resolve");
    }

    #[test]
    fn spread_never_repeats_a_neighbour_and_is_deterministic() {
        let k = kit();
        for cue in ["whoosh", "ui", "kick", "soft-pass", "swish"] {
            let run: Vec<Pick> = (0..40).map(|i| k.pick(cue, i, 5).unwrap()).collect();
            for w in run.windows(2) {
                assert_ne!(w[0].cue, w[1].cue, "{cue}: neighbouring picks match");
            }
            assert_eq!(run, (0..40).map(|i| k.pick(cue, i, 5).unwrap()).collect::<Vec<_>>());
            let distinct: std::collections::BTreeSet<&str> = run.iter().map(|p| p.cue.as_str()).collect();
            assert!(distinct.len() >= 3, "{cue} used only {distinct:?}");
            assert!(run.iter().all(|p| (p.rate - 1.0).abs() < 0.08 && (p.gain - 1.0).abs() < 0.3));
        }
        assert_ne!((0..6).map(|i| k.pick("whoosh", i, 1).unwrap().cue).collect::<Vec<_>>(), (0..6).map(|i| k.pick("whoosh", i, 2).unwrap().cue).collect::<Vec<_>>());
        // tonal voices are never detuned
        assert!((0..20).all(|i| k.pick("pluck", i, 1).unwrap().rate == 1.0));
    }

    #[test]
    fn every_indexed_file_exists_and_every_alias_resolves() {
        let k = kit();
        for f in k.families.values() {
            for v in &f.variants {
                assert!(dir().join(&f.name).join(format!("{}.wav", v.name)).exists(), "{}/{}", f.name, v.name);
            }
        }
        for a in k.aliases.keys() {
            k.pick(a, 0, 0).unwrap_or_else(|e| panic!("alias {a}: {e}"));
        }
        for old in ["card-in", "stamp", "counter", "loss", "reveal", "whoosh", "swish", "sweep", "riser", "pop", "click", "tick", "glitch", "boom", "type"] {
            k.pick(old, 0, 0).unwrap_or_else(|e| panic!("old cue {old}: {e}"));
        }
    }

    #[test]
    fn note_names_parse() {
        assert_eq!((parse_note("C4"), parse_note("a#3"), parse_note("Bb2"), parse_note("whoosh")), (Some(60), Some(58), Some(46), None));
        assert_eq!(note_name(63), "D#4");
    }
}
