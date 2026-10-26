//! The ledger's `linear` column against what the engine declares: does the effect compute on light (`yes`) or on the
//! numbers the eye reads (`no`)? The engine's answer is the effect's `space` (`// @space` in a plugin, the registry's
//! encoding column for a platform effect), so the ledger cannot claim a colour behaviour the code does not have.

use std::collections::BTreeMap;

use super::ledger::Ledger;

/// Every problem with a `linear` cell, given each catalogue effect's `space` (`display` or `linear`) by name. A row that
/// names no engine effect, or whose engine effects disagree with each other, may say `?`; so may one still being worked on.
pub fn linear(ledger: &Ledger, spaces: &BTreeMap<String, String>) -> Vec<String> {
    let t = &ledger.effects;
    let mut out = Vec::new();
    for r in &t.rows {
        let engine: Vec<&str> = t.val(r, "engine").split(',').filter(|s| spaces.contains_key(*s)).collect();
        if engine.is_empty() {
            continue;
        }
        let truth = match engine.iter().map(|e| spaces[*e].as_str()).collect::<Vec<_>>() {
            s if s.iter().all(|x| *x == "linear") => "yes",
            s if s.iter().all(|x| *x == "display") => "no",
            _ => "?",
        };
        let said = t.val(r, "linear");
        let started = matches!(t.val(r, "status"), "todo" | "in_progress");
        if said != truth && !(said == "?" && started) {
            let detail = engine.iter().map(|e| format!("{e}: {}", spaces[*e])).collect::<Vec<_>>().join(", ");
            out.push(format!("{}:{}: {:?} says linear {said:?} but the engine says {truth:?} ({detail})", t.path.display(), r.line, t.val(r, "name")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{super::ledger::fixture, *};

    fn spaces(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(n, s)| (n.to_string(), s.to_string())).collect()
    }

    #[test]
    fn linear_must_say_what_the_engine_declares() {
        let rows = ["Blur\tBlur\tB\tT0\thave\t?\t?\t-\tblur\t-", "Contrast\tColour\tA\tT1\thave\tno\tyes\t-\tcontrast\t-", "Mixed\tX\tA\tT1\thave\t?\t?\t-\tblur,contrast\t-", "Right\tX\tA\tT1\thave\tno\tyes\t-\tblur\t-", "Todo\tX\tA\tT1\ttodo\t?\t?\t-\t-\t-"];
        let dir = fixture::write("linear", &[], &rows);
        let p = linear(&Ledger::load(&dir).unwrap(), &spaces(&[("blur", "linear"), ("contrast", "display")]));
        assert_eq!(p.len(), 2, "{p:?}");
        assert!(p[0].contains("\"Blur\" says linear \"?\" but the engine says \"yes\" (blur: linear)"), "{p:?}");
        assert!(p[1].contains("\"Contrast\"") && p[1].contains("\"yes\"") && p[1].contains("\"no\""), "{p:?}");
        let started = fixture::write("linear_started", &[], &["Blur\tBlur\tB\tT0\tin_progress\t?\t?\t-\tblur\t-"]);
        assert!(linear(&Ledger::load(&started).unwrap(), &spaces(&[("blur", "linear")])).is_empty(), "a row in progress may still say ?");
    }
}
