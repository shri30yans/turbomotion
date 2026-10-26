//! `turbomotion parity report`: coverage per category, and the programme's own yardstick (T0+T1 effects done).

use std::{collections::BTreeMap, fmt::Write as _};

use super::ledger::{Ledger, Table};

/// How many rows have each status, for one category.
#[derive(Default, Clone)]
struct Counts {
    by_status: BTreeMap<String, usize>,
}

impl Counts {
    fn add(&mut self, status: &str) {
        *self.by_status.entry(status.to_string()).or_default() += 1;
    }
    fn n(&self, status: &str) -> usize {
        self.by_status.get(status).copied().unwrap_or(0)
    }
    fn total(&self) -> usize {
        self.by_status.values().sum()
    }
    /// Rows that count toward "done": everything except decisions not to build.
    fn in_scope(&self) -> usize {
        self.total() - self.n("not_supported")
    }
    fn percent(&self) -> f64 {
        if self.in_scope() == 0 {
            100.0
        } else {
            100.0 * self.n("done") as f64 / self.in_scope() as f64
        }
    }
}

fn tally(t: &Table, category_of: impl Fn(&super::ledger::Row) -> String) -> BTreeMap<String, Counts> {
    let mut m: BTreeMap<String, Counts> = BTreeMap::new();
    for r in &t.rows {
        m.entry(category_of(r)).or_default().add(t.val(r, "status"));
    }
    m
}

pub fn render(ledger: &Ledger) -> String {
    let mut s = String::new();
    let mut section = |title: &str, rows: BTreeMap<String, Counts>| {
        let _ = writeln!(s, "{title}\n{:<28} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5} {:>7}", "category", "rows", "done", "have", "part", "todo", "n/s", "% done");
        let mut all = Counts::default();
        for (cat, c) in &rows {
            let todo = c.n("todo") + c.n("none") + c.n("unverified") + c.n("in_progress");
            let _ = writeln!(s, "{cat:<28} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5} {:>6.1}%", c.total(), c.n("done"), c.n("have"), c.n("part"), todo, c.n("not_supported"), c.percent());
            for (k, v) in &c.by_status {
                *all.by_status.entry(k.clone()).or_default() += v;
            }
        }
        let todo = all.n("todo") + all.n("none") + all.n("unverified") + all.n("in_progress");
        let _ = writeln!(s, "{:<28} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5} {:>6.1}%\n", "ALL", all.total(), all.n("done"), all.n("have"), all.n("part"), todo, all.n("not_supported"), all.percent());
    };
    let m = &ledger.matrix;
    section("Core model (MATRIX.tsv)", tally(m, |r| m.val(r, "category").to_string()));
    let e = &ledger.effects;
    section("Effects (effects.tsv)", tally(e, |r| e.val(r, "category").to_string()));
    let (mut done, mut scope) = (0, 0);
    for r in &e.rows {
        if matches!(e.val(r, "tier"), "T0" | "T1") && e.val(r, "status") != "not_supported" {
            scope += 1;
            done += usize::from(e.val(r, "status") == "done");
        }
    }
    let pct = if scope == 0 { 100.0 } else { 100.0 * done as f64 / scope as f64 };
    let _ = writeln!(s, "T0+T1 effects done: {done} of {scope} ({pct:.1}%); the programme target is 95%");
    s
}

#[cfg(test)]
mod tests {
    use super::{super::ledger::fixture, *};

    #[test]
    fn the_report_counts_each_category_and_the_tier_yardstick() {
        let dir = fixture::write(
            "report",
            &["a\tmodel\tdone\t-\t-\t-\t-", "b\tmodel\tnone\t-\t-\t-\t-", "c\tmodel\tnot_supported\t-\t-\t-\t-"],
            &["X\tBlur\tA\tT0\tdone\tno\tno\t-\tx\t-", "Y\tBlur\tA\tT1\ttodo\t?\t?\t-\t-\t-", "Z\tBlur\tA\tT2\ttodo\t?\t?\t-\t-\t-"],
        );
        let text = render(&Ledger::load(&dir).unwrap());
        assert!(text.contains("model") && text.contains("50.0%"), "{text}");
        assert!(text.contains("T0+T1 effects done: 1 of 2 (50.0%)"), "{text}");
    }
}
