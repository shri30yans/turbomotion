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

/// What the check found: the problems that make the ledger unsound, and the proofs it could not check here.
#[derive(Default)]
pub struct Outcome {
    /// Lines that make the ledger unsound.
    pub problems: Vec<String>,
    /// Proofs that live outside this checkout: not failures, but the check says so out loud.
    pub unverified: Vec<String>,
}

/// Every problem in `ledger`, given the effect names the catalogue really has. Empty means the ledger is sound.
pub fn check(ledger: &Ledger, catalogue: &BTreeSet<String>) -> Outcome {
    let mut out = Outcome::default();
    matrix(ledger, &mut out);
    effects(ledger, catalogue, &mut out);
    out
}

/// What the engine says about whether an effect reaches past its input (`catalogue effects[].bounds`: `none`, `grows`,
/// `expand=...`, or `undeclared`), against the ledger's `grows_bounds` for every row that names engine effects (a row still
/// `todo` or `in_progress` may say `?`).
pub fn bounds(ledger: &Ledger, bounds: &BTreeMap<String, String>) -> Vec<String> {
    let t = &ledger.effects;
    let mut problems = Vec::new();
    for r in &t.rows {
        let engine: Vec<&str> = t.val(r, "engine").split(',').filter(|s| !s.is_empty() && bounds.contains_key(*s)).collect();
        if engine.is_empty() {
            continue;
        }
        let tags: Vec<&str> = engine.iter().map(|e| bounds[*e].as_str()).collect();
        let truth = if tags.iter().any(|b| !matches!(*b, "none" | "undeclared")) {
            "yes"
        } else if tags.contains(&"undeclared") {
            "?"
        } else {
            "no"
        };
        let said = t.val(r, "grows_bounds");
        // a row being worked on (`init-effect` scaffolds `?`) may still say so; one that claims to exist may not
        let started = matches!(t.val(r, "status"), "todo" | "in_progress");
        if said != truth && !(said == "?" && started) {
            problems.push(at(t, r.line, format!("{:?} says grows_bounds {said:?} but the engine says {truth:?} ({})", t.val(r, "name"), engine.iter().zip(&tags).map(|(e, b)| format!("{e}: {b}")).collect::<Vec<_>>().join(", "))));
        }
    }
    problems
}

fn at(t: &Table, line: usize, msg: String) -> String {
    format!("{}:{line}: {msg}", t.path.display())
}

fn status_ok(t: &Table, status: &str, line: usize, out: &mut Outcome) {
    if !STATUSES.contains(&status) {
        out.problems.push(at(t, line, format!("status {status:?} is not one of {}", STATUSES.join(", "))));
    }
}

fn matrix(ledger: &Ledger, out: &mut Outcome) {
    let t = &ledger.matrix;
    let root = ledger.root();
    let mut seen = BTreeMap::new();
    for r in &t.rows {
        let feature = t.val(r, "feature");
        if feature.is_empty() {
            out.problems.push(at(t, r.line, "empty feature".into()));
        }
        if let Some(first) = seen.insert(feature.to_string(), r.line) {
            out.problems.push(at(t, r.line, format!("feature {feature:?} repeats line {first}")));
        }
        let status = t.val(r, "status");
        status_ok(t, status, r.line, out);
        let wp = t.val(r, "wp");
        if !(wp.is_empty() || (wp.starts_with("WP-") && wp.len() == 5 && wp[3..].chars().all(|c| c.is_ascii_digit()))) {
            out.problems.push(at(t, r.line, format!("wp {wp:?} is not WP-NN")));
        }
        if status == "done" {
            proof(t, r.line, feature, t.val(r, "test"), Some(t.val(r, "doc")), &root, out);
        }
    }
}

/// A `done` row names a test that exists and a document that exists.
///
/// The test cell is either a bare test name (looked for under `crates/` and `../scripts/`) or `path::name`, which points
/// at one file. A `path::name` whose file is not in this checkout is not a failure: the proof lives in another tree,
/// so it is reported as unverified instead.
fn proof(t: &Table, line: usize, what: &str, test: &str, doc: Option<&str>, root: &Path, out: &mut Outcome) {
    if test.is_empty() {
        out.problems.push(at(t, line, format!("{what:?} is done but names no test")));
    } else if let Some((path, name)) = test.split_once("::") {
        let file = root.join(path);
        if file.exists() {
            if !names_test(&file, name) {
                out.problems.push(at(t, line, format!("{what:?} is done but {path} has no test named {name:?}")));
            }
        } else {
            out.unverified.push(at(t, line, format!("{what:?} is done but its test {path} is not in this checkout, so the proof could not be checked here")));
        }
    } else if !test_exists(root, test) {
        out.problems.push(at(t, line, format!("{what:?} is done but no test named {test:?} exists under crates/ or ../scripts/")));
    }
    if let Some(doc) = doc {
        if doc.is_empty() {
            out.problems.push(at(t, line, format!("{what:?} is done but names no doc")));
        } else if !root.join(doc).exists() {
            out.problems.push(at(t, line, format!("{what:?} is done but its doc {doc:?} does not exist")));
        }
    }
}

fn effects(ledger: &Ledger, catalogue: &BTreeSet<String>, out: &mut Outcome) {
    let t = &ledger.effects;
    let root = ledger.root();
    let mut seen = BTreeMap::new();
    let mut covered: BTreeSet<String> = BTreeSet::new();
    for r in &t.rows {
        let name = t.val(r, "name");
        if let Some(first) = seen.insert(name.to_string(), r.line) {
            out.problems.push(at(t, r.line, format!("effect {name:?} repeats line {first}")));
        }
        let status = t.val(r, "status");
        status_ok(t, status, r.line, out);
        for (col, allowed) in [("recipe_class", &CLASSES[..]), ("tier", &TIERS[..]), ("grows_bounds", &TRISTATE[..]), ("linear", &TRISTATE[..])] {
            let v = r.cells[t.col(col)].as_str();
            if !allowed.contains(&v) {
                out.problems.push(at(t, r.line, format!("{col} {v:?} is not one of {}", allowed.join(", "))));
            }
        }
        let engine: Vec<&str> = t.val(r, "engine").split(',').filter(|s| !s.is_empty()).collect();
        for e in &engine {
            if !catalogue.contains(*e) {
                out.problems.push(at(t, r.line, format!("{name:?} says the engine effect {e:?} implements it, but the catalogue has no effect {e:?}")));
            }
            covered.insert(e.to_string());
        }
        match status {
            "have" | "part" | "done" if engine.is_empty() => out.problems.push(at(t, r.line, format!("{name:?} is {status} but names no engine effect in the catalogue"))),
            "todo" | "not_supported" if !engine.is_empty() => out.problems.push(at(t, r.line, format!("{name:?} is {status} but names engine effects ({}); set it to part, have or in_progress", engine.join(",")))),
            _ => {}
        }
        if status == "done" {
            proof(t, r.line, name, t.val(r, "test"), None, &root, out);
            for (folder, ext) in [("specs", "md"), ("probes", "json")] {
                if !ledger.dir.join(folder).join(format!("{}.{ext}", slug(name))).exists() {
                    out.problems.push(at(t, r.line, format!("{name:?} is done but parity/{folder}/{}.{ext} does not exist", slug(name))));
                }
            }
        }
    }
    for name in catalogue {
        if !covered.contains(name) {
            out.problems.push(format!("{}: the catalogue has the effect {name:?} and no row lists it in `engine`; add a row", t.path.display()));
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

/// Is there a `fn <test>(` (Rust) or `def <test>(` (Python) in this file?
fn names_test(file: &Path, test: &str) -> bool {
    let needles = [format!("fn {test}("), format!("def {test}(")];
    fs::read_to_string(file).is_ok_and(|s| needles.iter().any(|n| s.contains(n.as_str())))
}

/// Is there a `fn <test>(` (Rust) or `def <test>(` (Python) under `<root>/crates` or `<root>/../scripts`?
fn test_exists(root: &Path, test: &str) -> bool {
    let mut stack: Vec<PathBuf> = vec![root.join("crates"), root.join("../scripts")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs" || x == "py") && names_test(&p, test) {
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
        check(&Ledger::load(&dir).unwrap(), &names(catalogue)).problems
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
        let p = check(&Ledger::load(&dir).unwrap(), &names(&["blur"])).problems;
        assert!(p.iter().any(|m| m.contains("parity/specs/blur_two.md does not exist")), "{p:?}");
        assert!(p.iter().any(|m| m.contains("parity/probes/blur_two.json does not exist")), "{p:?}");
    }

    #[test]
    fn a_proof_in_a_file_outside_this_checkout_is_a_note_and_not_a_failure() {
        let row = "Fonts\tAssets\tdone\toutside.py::some_test\tREADME.md\t-\tWP-52";
        let dir = fixture::write("external", &[row], &[]);
        let root = dir.parent().unwrap().to_path_buf();
        fs::write(root.join("README.md"), "the doc").unwrap();
        fs::write(root.join("outside.py"), "def some_test():\n    pass\n").unwrap();
        let here = check(&Ledger::load(&dir).unwrap(), &BTreeSet::new());
        assert!(here.problems.is_empty(), "{:?}", here.problems);
        assert!(here.unverified.is_empty(), "{:?}", here.unverified);
        fs::remove_file(root.join("outside.py")).unwrap();
        let away = check(&Ledger::load(&dir).unwrap(), &BTreeSet::new());
        assert!(away.problems.is_empty(), "{:?}", away.problems);
        assert_eq!(away.unverified.len(), 1, "{:?}", away.unverified);
        assert!(away.unverified[0].contains("outside.py"), "{:?}", away.unverified);
    }

    fn bound_map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(n, b)| (n.to_string(), b.to_string())).collect()
    }

    #[test]
    fn grows_bounds_must_say_what_the_engine_says() {
        let rows = ["Blur\tBlur\tB\tT0\thave\tno\t?\t-\tblur\t-", "Wave\tDistort\tA\tT1\thave\tyes\t?\t-\tflat\t-", "Mix\tX\tA\tT1\thave\t?\t?\t-\tplain,odd\t-", "Todo\tX\tA\tT1\ttodo\t?\t?\t-\t-\t-"];
        let dir = fixture::write("bounds", &[], &rows);
        let p = bounds(&Ledger::load(&dir).unwrap(), &bound_map(&[("blur", "grows"), ("flat", "none"), ("plain", "none"), ("odd", "undeclared")]));
        assert_eq!(p.len(), 2, "{p:?}");
        let started = fixture::write("bounds_started", &[], &["Blur\tBlur\tB\tT0\tin_progress\t?\t?\t-\tblur\t-"]);
        assert!(bounds(&Ledger::load(&started).unwrap(), &bound_map(&[("blur", "grows")])).is_empty(), "a row in progress may still say ?");
        assert!(p[0].contains("\"Blur\" says grows_bounds \"no\" but the engine says \"yes\" (blur: grows)"), "{p:?}");
        assert!(p[1].contains("\"Wave\"") && p[1].contains("\"no\""), "{p:?}");
        let p = bounds(&Ledger::load(&dir).unwrap(), &bound_map(&[("blur", "expand=amount*2"), ("flat", "yes-it-grows"), ("plain", "none"), ("odd", "undeclared")]));
        assert!(p.len() == 1 && p[0].contains("\"Blur\"") && p[0].contains("blur: expand=amount*2"), "a plugin's expansion counts as growing, so only the Blur row is wrong: {p:?}");
    }

    #[test]
    fn slugs_are_file_names() {
        assert_eq!(slug("Gaussian Blur"), "gaussian_blur");
        assert_eq!(slug("CC Light Sweep (2)"), "cc_light_sweep_2");
    }
}
