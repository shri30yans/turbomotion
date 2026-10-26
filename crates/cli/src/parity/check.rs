//! `turbomotion parity check`: the ledger and the engine must agree. Every problem is one line, `file:line: message`.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use super::ledger::{Ledger, Table, STATUSES};

const CLASSES: [&str; 6] = ["A", "B", "C", "D", "E", "-"];
const TIERS: [&str; 4] = ["T0", "T1", "T2", "-"];
const TRISTATE: [&str; 3] = ["yes", "no", "?"];

/// Every problem in `ledger`, given the effect names the catalogue really has. Empty means the ledger is sound.
pub fn check(ledger: &Ledger, catalogue: &BTreeSet<String>) -> Vec<String> {
    let mut out = Vec::new();
    matrix(ledger, &mut out);
    effects(ledger, catalogue, &mut out);
    out
}

fn at(t: &Table, line: usize, msg: String) -> String {
    format!("{}:{line}: {msg}", t.path.display())
}

fn status_ok(t: &Table, status: &str, line: usize, out: &mut Vec<String>) {
    if !STATUSES.contains(&status) {
        out.push(at(t, line, format!("status {status:?} is not one of {}", STATUSES.join(", "))));
    }
}

fn matrix(ledger: &Ledger, out: &mut Vec<String>) {
    let t = &ledger.matrix;
    let root = ledger.root();
    let mut seen = BTreeMap::new();
    for r in &t.rows {
        let feature = t.val(r, "feature");
        if feature.is_empty() {
            out.push(at(t, r.line, "empty feature".into()));
        }
        if let Some(first) = seen.insert(feature.to_string(), r.line) {
            out.push(at(t, r.line, format!("feature {feature:?} repeats line {first}")));
        }
        let status = t.val(r, "status");
        status_ok(t, status, r.line, out);
        let wp = t.val(r, "wp");
        if !wp.is_empty() && !(wp.starts_with("WP-") && wp.len() == 5 && wp[3..].chars().all(|c| c.is_ascii_digit())) {
            out.push(at(t, r.line, format!("wp {wp:?} is not WP-NN")));
        }
        if status == "done" {
            proof(t, r.line, feature, t.val(r, "test"), Some(t.val(r, "doc")), &root, out);
        }
    }
}

/// A `done` row names a test that exists and a document that exists.
fn proof(t: &Table, line: usize, what: &str, test: &str, doc: Option<&str>, root: &Path, out: &mut Vec<String>) {
    if test.is_empty() {
        out.push(at(t, line, format!("{what:?} is done but names no test")));
    } else if !test_exists(root, test) {
        out.push(at(t, line, format!("{what:?} is done but no test named {test:?} exists under crates/ or ../scripts/")));
    }
    if let Some(doc) = doc {
        if doc.is_empty() {
            out.push(at(t, line, format!("{what:?} is done but names no doc")));
        } else if !root.join(doc).exists() {
            out.push(at(t, line, format!("{what:?} is done but its doc {doc:?} does not exist")));
        }
    }
}

fn effects(ledger: &Ledger, catalogue: &BTreeSet<String>, out: &mut Vec<String>) {
    let t = &ledger.effects;
    let root = ledger.root();
    let mut seen = BTreeMap::new();
    let mut covered: BTreeSet<String> = BTreeSet::new();
    for r in &t.rows {
        let name = t.val(r, "name");
        if let Some(first) = seen.insert(name.to_string(), r.line) {
            out.push(at(t, r.line, format!("effect {name:?} repeats line {first}")));
        }
        let status = t.val(r, "status");
        status_ok(t, status, r.line, out);
        for (col, allowed) in [("recipe_class", &CLASSES[..]), ("tier", &TIERS[..]), ("grows_bounds", &TRISTATE[..]), ("linear", &TRISTATE[..])] {
            let v = r.cells[t.col(col)].as_str();
            if !allowed.contains(&v) {
                out.push(at(t, r.line, format!("{col} {v:?} is not one of {}", allowed.join(", "))));
            }
        }
        let engine: Vec<&str> = t.val(r, "engine").split(',').filter(|s| !s.is_empty()).collect();
        for e in &engine {
            if !catalogue.contains(*e) {
                out.push(at(t, r.line, format!("{name:?} says the engine effect {e:?} implements it, but the catalogue has no effect {e:?}")));
            }
            covered.insert(e.to_string());
        }
        match status {
            "have" | "part" | "done" if engine.is_empty() => out.push(at(t, r.line, format!("{name:?} is {status} but names no engine effect in the catalogue"))),
            "todo" | "not_supported" if !engine.is_empty() => out.push(at(t, r.line, format!("{name:?} is {status} but names engine effects ({}); set it to part, have or in_progress", engine.join(",")))),
            _ => {}
        }
        if status == "done" {
            proof(t, r.line, name, t.val(r, "test"), None, &root, out);
            for (folder, ext) in [("specs", "md"), ("probes", "json")] {
                if !ledger.dir.join(folder).join(format!("{}.{ext}", slug(name))).exists() {
                    out.push(at(t, r.line, format!("{name:?} is done but parity/{folder}/{}.{ext} does not exist", slug(name))));
                }
            }
        }
    }
    for name in catalogue {
        if !covered.contains(name) {
            out.push(format!("{}: the catalogue has the effect {name:?} and no row lists it in `engine`; add a row", t.path.display()));
        }
    }
}

/// File-name form of an effect name: `Gaussian Blur` -> `gaussian_blur`.
pub fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
        } else if !s.ends_with('_') && !s.is_empty() {
            s.push('_');
        }
    }
    s.trim_end_matches('_').to_string()
}

/// Is there a `fn <test>(` (Rust) or `def <test>(` (Python) under `<root>/crates` or `<root>/../scripts`?
fn test_exists(root: &Path, test: &str) -> bool {
    let needles = [format!("fn {test}("), format!("def {test}(")];
    let mut stack: Vec<PathBuf> = vec![root.join("crates"), root.join("../scripts")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs" || x == "py") && fs::read_to_string(&p).is_ok_and(|s| needles.iter().any(|n| s.contains(n.as_str()))) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{super::ledger::fixture, *};

    const MATRIX_HEADER_ROW_DONE_NO_TEST: &str = "Hold keys\tanimation\tdone\t-\t-\t-\t-";

    fn names(v: &[&str]) -> BTreeSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn problems(tag: &str, matrix: &[&str], effects: &[&str], catalogue: &[&str]) -> Vec<String> {
        let dir = fixture::write(tag, matrix, effects);
        check(&Ledger::load(&dir).unwrap(), &names(catalogue))
    }

    #[test]
    fn a_sound_ledger_has_no_problems() {
        let p = problems("sound", &["Hold keys\tanimation\thave\t-\t-\t-\t-"], &["Blur\tBlur\t-\tT1\thave\t?\t?\t-\tblur\t-", "Bloom\tx\tB\tT1\ttodo\t?\t?\t-\t-\t-"], &["blur"]);
        assert!(p.is_empty(), "{p:?}");
    }

    #[test]
    fn a_done_core_row_without_a_test_or_a_doc_fails() {
        let p = problems("matrix-done", &[MATRIX_HEADER_ROW_DONE_NO_TEST], &[], &[]);
        assert!(p.iter().any(|m| m.contains("done but names no test")), "{p:?}");
        assert!(p.iter().any(|m| m.contains("done but names no doc")), "{p:?}");
    }

    #[test]
    fn a_done_row_must_name_a_test_that_exists() {
        let p = problems("matrix-ghost", &["Hold keys\tanimation\tdone\tno_such_test_anywhere\tREADME.md\t-\t-"], &[], &[]);
        assert!(p.iter().any(|m| m.contains("no test named")), "{p:?}");
    }

    #[test]
    fn an_effect_marked_done_that_is_not_in_the_catalogue_fails() {
        let p = problems("done-missing", &[], &["Blur\tBlur\tB\tT0\tdone\tno\tno\t-\tghost_blur\t-"], &[]);
        assert!(p.iter().any(|m| m.contains("the catalogue has no effect \"ghost_blur\"")), "{p:?}");
        let q = problems("done-empty", &[], &["Blur\tBlur\tB\tT0\tdone\tno\tno\t-\t-\t-"], &[]);
        assert!(q.iter().any(|m| m.contains("done but names no engine effect")), "{q:?}");
    }

    #[test]
    fn a_catalogue_effect_with_no_row_fails() {
        let p = problems("uncovered", &[], &["Blur\tBlur\t-\tT1\thave\t?\t?\t-\tblur\t-"], &["blur", "twirl"]);
        assert_eq!(p.len(), 1, "{p:?}");
        assert!(p[0].contains("\"twirl\""), "{p:?}");
    }

    #[test]
    fn bad_vocabulary_and_duplicates_are_named_with_their_line() {
        let p = problems("vocab", &["A\tc\tmaybe\t-\t-\t-\tWP-1", "A\tc\thave\t-\t-\t-\t-"], &["X\tc\tZ\tT9\tdone?\tmaybe\t?\t-\t-\t-"], &[]);
        for want in ["status \"maybe\"", "wp \"WP-1\"", "repeats line 2", "recipe_class \"Z\"", "tier \"T9\"", "grows_bounds \"maybe\""] {
            assert!(p.iter().any(|m| m.contains(want)), "{want}: {p:?}");
        }
    }

    #[test]
    fn a_done_effect_needs_its_spec_and_probe_files() {
        let dir = fixture::write("files", &[], &["Blur Two\tBlur\tB\tT0\tdone\tno\tno\t-\tblur\tn_a_test"]);
        let p = check(&Ledger::load(&dir).unwrap(), &names(&["blur"]));
        assert!(p.iter().any(|m| m.contains("parity/specs/blur_two.md does not exist")), "{p:?}");
        assert!(p.iter().any(|m| m.contains("parity/probes/blur_two.json does not exist")), "{p:?}");
    }

    #[test]
    fn slugs_are_file_names() {
        assert_eq!(slug("Gaussian Blur"), "gaussian_blur");
        assert_eq!(slug("CC Light Sweep (2)"), "cc_light_sweep_2");
    }
}
