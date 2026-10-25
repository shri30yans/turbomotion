//! Named text animations. A preset expands into animators sized to the text, so `"rise"` means "rise by about a
//! line" whatever the font size. Entrances follow the measured rules: long-tailed arrivals, words 60-120 ms apart,
//! letters a frame or two apart, one idea per preset.

use anyhow::{anyhow, bail, ensure, Result};
use serde_json::Value;

use crate::schema::{Animator, Direction, Stagger, StaggerFrom, Unit, UnitProps};

pub const PRESETS: &[(&str, &str)] = &[
    ("rise", "lines rise from behind a mask, one after another (the masked line reveal)"),
    ("reveal", "alias of rise: a line-by-line masked reveal"),
    ("rise_words", "words rise from behind a mask"),
    ("fade_up", "words fade in while rising a little"),
    ("blur_in", "words resolve from a blur, 80 ms apart, without moving"),
    ("pop", "characters pop in with a small overshoot"),
    ("slam", "words slam in large and settle"),
    ("drop", "characters fall in and bounce"),
    ("typewriter", "characters appear one by one"),
    ("scramble", "characters scramble through random symbols and resolve in order"),
    ("tracking_in", "the whole line eases in from wide letter-spacing"),
    ("wave_in", "characters rise in a wave from the left"),
    ("spin_in", "characters turn into place"),
    ("flip_chars", "characters flip up from flat, like a SplitText rotateX"),
    ("elastic", "characters spring up from nothing"),
    ("fade_out", "words fade out (an exit)"),
    ("rise_out", "lines rise away behind a mask (an exit)"),
];

/// How much text a preset is sized to: its type size and how many characters it will stagger over.
#[derive(Debug, Clone, Copy)]
pub struct TextFacts {
    pub size: f64,
    /// Characters that are not whitespace.
    pub visible: usize,
    /// Every character, spaces included.
    pub all: usize,
}

impl TextFacts {
    pub fn of(text: &str, size: f64) -> TextFacts {
        TextFacts { size, visible: text.chars().filter(|c| !c.is_whitespace()).count(), all: text.chars().count() }
    }
}

/// Overrides a preset accepts: `{"name": "rise", "by": "words", "each": 0.1, "at": 0.4, "dur": 0.8, "ease": "expo.out", "dist": 80, "blur": 10, "from": "center", "seed": 3}`.
///
/// `typewriter` reads two options differently: `dur` is the time to type the whole text (the first character shows at
/// `at`, the last at `at + dur`; it replaces `each`), and `spaces: true` gives spaces a slot of their own.
pub fn expand(spec: &Value, facts: TextFacts) -> Result<Vec<Animator>> {
    let size = facts.size;
    let (name, o) = match spec {
        Value::String(n) => (n.as_str(), None),
        Value::Object(o) => (o.get("name").and_then(Value::as_str).ok_or_else(|| anyhow!("a preset needs \"name\""))?, Some(o)),
        other => bail!("a preset is a name like \"rise\" or an object {{\"name\": \"rise\", ...}}, not {other}"),
    };
    if let Some(o) = o {
        if let Some(bad) = o.keys().find(|k| !["name", "by", "each", "at", "dur", "ease", "dist", "blur", "from", "seed", "mask", "spaces"].contains(&k.as_str())) {
            bail!("unknown preset option {bad:?}; options: by, each, at, dur, ease, dist, blur, from, seed, mask, spaces");
        }
    }
    let num = |k: &str| o.and_then(|o| o.get(k)).and_then(Value::as_f64);
    let text = |k: &str| o.and_then(|o| o.get(k)).and_then(Value::as_str);
    let by = |default: Unit| -> Result<Unit> {
        Ok(match text("by") {
            None => default,
            Some("chars") => Unit::Chars,
            Some("words") => Unit::Words,
            Some("lines") => Unit::Lines,
            Some(other) => bail!("\"by\" is chars, words or lines, not {other:?}"),
        })
    };
    let from = match text("from") {
        None => StaggerFrom::Start,
        Some("start") => StaggerFrom::Start,
        Some("end") => StaggerFrom::End,
        Some("center") => StaggerFrom::Center,
        Some("edges") => StaggerFrom::Edges,
        Some("random") => StaggerFrom::Random,
        Some(other) => bail!("\"from\" is start, end, center, edges or random, not {other:?}"),
    };
    let dist = num("dist");
    let blur = num("blur");
    let make = |by_default: Unit, props: UnitProps, each: f64, dur: f64, ease: &str, mask: bool| -> Result<Animator> {
        Ok(Animator {
            by: by(by_default)?,
            props,
            at: num("at").unwrap_or(0.0),
            dur: num("dur").unwrap_or(dur),
            ease: Some(text("ease").unwrap_or(ease).to_string()),
            direction: Direction::In,
            stagger: Some(Stagger { each: num("each").unwrap_or(each), from, seed: num("seed").unwrap_or(1.0) as u32, ease: None }),
            selector: None,
            mask: o.and_then(|o| o.get("mask")).and_then(Value::as_bool).unwrap_or(mask),
            scramble: None,
            count_spaces: false,
            cipher: None,
        })
    };
    let up = |k: f64| UnitProps { y: Some(dist.unwrap_or(size * k)), ..UnitProps::default() };
    Ok(vec![match name {
        "rise" | "reveal" => make(Unit::Lines, up(1.25), 0.12, 0.8, "expo.out", true)?,
        "rise_words" => make(Unit::Words, up(1.25), 0.07, 0.7, "expo.out", true)?,
        "fade_up" => make(Unit::Words, UnitProps { y: Some(dist.unwrap_or(size * 0.45)), opacity: Some(-1.0), ..UnitProps::default() }, 0.08, 0.7, "power3.out", false)?,
        "blur_in" => make(Unit::Words, UnitProps { blur: Some(blur.unwrap_or(size * 0.2)), opacity: Some(-1.0), ..UnitProps::default() }, 0.08, 0.5, "power2.out", false)?,
        "pop" => make(Unit::Chars, UnitProps { scale: Some(-1.0), opacity: Some(-1.0), ..UnitProps::default() }, 0.03, 0.5, "back.out(2)", false)?,
        "slam" => make(Unit::Words, UnitProps { scale: Some(2.2), opacity: Some(-1.0), blur: Some(blur.unwrap_or(size * 0.15)), ..UnitProps::default() }, 0.12, 0.35, "expo.out", false)?,
        "drop" => make(Unit::Chars, UnitProps { y: Some(-dist.unwrap_or(size * 2.2)), opacity: Some(-1.0), ..UnitProps::default() }, 0.04, 0.8, "bounce.out", false)?,
        "typewriter" => {
            let mut a = make(Unit::Chars, UnitProps { opacity: Some(-1.0), ..UnitProps::default() }, 0.045, 0.0, "none", false)?;
            a.dur = 0.0; // a character is either there or not; `dur` below is the whole sentence
            a.count_spaces = o.and_then(|o| o.get("spaces")).and_then(Value::as_bool).unwrap_or(false);
            if let Some(total) = num("dur") {
                ensure!(total >= 0.0, "typewriter \"dur\" is the seconds to type the whole text, 0 or more, got {total}");
                let slots = if a.count_spaces { facts.all } else { facts.visible };
                if let Some(s) = a.stagger.as_mut() {
                    s.each = if slots > 1 { total / (slots - 1) as f64 } else { 0.0 };
                }
            }
            a
        }
        "scramble" => {
            let mut a = make(Unit::Chars, UnitProps::default(), 0.04, 0.6, "none", false)?;
            a.scramble = Some("01#%&@$X/\\".into());
            a
        }
        "tracking_in" => make(Unit::Chars, UnitProps { tracking: Some(dist.unwrap_or(size * 0.35)), opacity: Some(-1.0), ..UnitProps::default() }, 0.0, 1.1, "expo.out", false)?,
        "wave_in" => make(Unit::Chars, UnitProps { y: Some(dist.unwrap_or(size * 0.8)), opacity: Some(-1.0), ..UnitProps::default() }, 0.035, 0.7, "back.out(1.6)", false)?,
        "spin_in" => make(Unit::Chars, UnitProps { rotation: Some(-90.0), scale: Some(-0.6), opacity: Some(-1.0), ..UnitProps::default() }, 0.04, 0.6, "back.out(1.4)", false)?,
        "flip_chars" => make(Unit::Chars, UnitProps { scale: Some(-1.0), rotation: Some(75.0), opacity: Some(-1.0), skew: Some(12.0), ..UnitProps::default() }, 0.035, 0.55, "back.out(1.6)", false)?,
        "elastic" => make(Unit::Chars, UnitProps { scale: Some(-1.0), ..UnitProps::default() }, 0.04, 1.1, "elastic.out(1, 0.45)", false)?,
        "fade_out" => {
            let mut a = make(Unit::Words, UnitProps { opacity: Some(-1.0), ..UnitProps::default() }, 0.05, 0.35, "power2.in", false)?;
            a.direction = Direction::Out;
            a
        }
        "rise_out" => {
            let mut a = make(Unit::Lines, UnitProps { y: Some(-dist.unwrap_or(size * 1.25)), ..UnitProps::default() }, 0.08, 0.4, "power3.in", true)?;
            a.direction = Direction::Out;
            a
        }
        other => bail!("unknown text preset {other:?}; presets are {}", PRESETS.iter().map(|p| p.0).collect::<Vec<_>>().join(", ")),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_preset_expands_and_overrides_apply() {
        for (name, _) in PRESETS {
            assert_eq!(expand(&json!(name), TextFacts::of("Hi there", 80.0)).unwrap().len(), 1, "{name}");
        }
        let a = expand(&json!({"name": "rise", "by": "words", "each": 0.2, "at": 1.0}), TextFacts::of("a b", 100.0)).unwrap().remove(0);
        assert_eq!((a.by, a.at, a.stagger.unwrap().each), (Unit::Words, 1.0, 0.2));
        assert!(expand(&json!("nope"), TextFacts::of("x", 80.0)).unwrap_err().to_string().contains("blur_in"));
        assert!(expand(&json!({"name": "rise", "speed": 2}), TextFacts::of("x", 80.0)).is_err());
    }

    #[test]
    fn typewriter_dur_is_the_whole_sentence_and_spaces_can_take_a_slot() {
        let facts = TextFacts::of("ab cd", 40.0);
        let each = |spec: Value| expand(&spec, facts).unwrap().remove(0).stagger.unwrap().each;
        assert!((each(json!({"name": "typewriter", "dur": 3.0})) - 1.0).abs() < 1e-9, "4 visible characters: 3 gaps in 3 s");
        assert!((each(json!({"name": "typewriter", "dur": 4.0, "spaces": true})) - 1.0).abs() < 1e-9, "5 characters: 4 gaps in 4 s");
        assert_eq!(each(json!("typewriter")), 0.045);
        let a = expand(&json!({"name": "typewriter", "spaces": true}), facts).unwrap().remove(0);
        assert!(a.count_spaces && a.dur == 0.0);
    }
}
