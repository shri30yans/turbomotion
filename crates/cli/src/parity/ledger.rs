//! The parity ledger on disk: two tab-separated tables, `MATRIX.tsv` (core-model features) and `effects.tsv` (one row
//! per effect), plus the folders beside them (`specs/ probes/ golden/`). A cell holding `-` is empty.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

pub const MATRIX_COLUMNS: [&str; 7] = ["feature", "category", "status", "test", "doc", "ae_ref", "wp"];
/// `engine` (the catalogue names that implement the row, comma separated) and `test` extend ADR-6's list: an AE name and
/// an engine name are different things, and a `done` row has to point at the test that proves it.
pub const EFFECT_COLUMNS: [&str; 10] = ["name", "category", "recipe_class", "tier", "status", "grows_bounds", "linear", "ae_params", "engine", "test"];

/// `have`/`part`/`none`/`unverified` are the seed vocabulary (what exists today); `todo`, `in_progress`, `done` are the
/// work states; `not_supported` is a decision recorded with its reason in the plan.
pub const STATUSES: [&str; 8] = ["have", "part", "none", "unverified", "todo", "in_progress", "done", "not_supported"];

pub struct Row {
    /// 1-based line in the file, for messages.
    pub line: usize,
    pub cells: Vec<String>,
}

pub struct Table {
    pub path: PathBuf,
    pub columns: &'static [&'static str],
    pub rows: Vec<Row>,
}

impl Table {
    pub fn load(path: &Path, columns: &'static [&'static str]) -> Result<Table> {
        let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut lines = text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#'));
        let Some((n, header)) = lines.next() else { bail!("{}: empty ledger; expected the header {}", path.display(), columns.join("\t")) };
        if header.split('\t').collect::<Vec<_>>() != columns {
            bail!("{}:{}: the header is {:?}; expected the columns {}", path.display(), n + 1, header, columns.join(", "));
        }
        let mut rows = Vec::new();
        for (n, l) in lines {
            let cells: Vec<String> = l.split('\t').map(str::to_string).collect();
            if cells.len() != columns.len() {
                bail!("{}:{}: {} cells; the table has {} columns ({})", path.display(), n + 1, cells.len(), columns.len(), columns.join(", "));
            }
            rows.push(Row { line: n + 1, cells });
        }
        Ok(Table { path: path.to_path_buf(), columns, rows })
    }

    pub fn col(&self, name: &str) -> usize {
        self.columns.iter().position(|c| *c == name).unwrap_or_else(|| panic!("no column {name}"))
    }

    /// The cell of `row` in column `name`; `-` reads as empty.
    pub fn val<'a>(&self, row: &'a Row, name: &str) -> &'a str {
        let v = row.cells[self.col(name)].as_str();
        if v == "-" {
            ""
        } else {
            v
        }
    }

    pub fn save(&self) -> Result<()> {
        let mut out = self.columns.join("\t");
        out.push('\n');
        for r in &self.rows {
            out.push_str(&r.cells.join("\t"));
            out.push('\n');
        }
        fs::write(&self.path, out).with_context(|| format!("writing {}", self.path.display()))
    }

    pub fn push(&mut self, cells: Vec<String>) {
        let line = self.rows.last().map_or(2, |r| r.line + 1);
        self.rows.push(Row { line, cells });
    }
}

pub struct Ledger {
    pub dir: PathBuf,
    pub matrix: Table,
    pub effects: Table,
}

impl Ledger {
    pub fn load(dir: &Path) -> Result<Ledger> {
        Ok(Ledger { dir: dir.to_path_buf(), matrix: Table::load(&dir.join("MATRIX.tsv"), &MATRIX_COLUMNS)?, effects: Table::load(&dir.join("effects.tsv"), &EFFECT_COLUMNS)? })
    }

    /// The folder the ledger sits in: paths in `test`/`doc` cells are relative to it (`../scripts/x.py` is allowed).
    pub fn root(&self) -> PathBuf {
        self.dir.parent().map(Path::to_path_buf).filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| PathBuf::from("."))
    }
}

/// The work package that builds a `todo` effect: the family WPs first, the long tail (WP-33) for the rest.
fn effect_wp(category: &str, name: &str) -> &'static str {
    match category {
        "Blur & Sharpen" => "WP-02",
        "Keying & Matte" => "WP-35",
        "Simulation" => "WP-36",
        "Time" => "WP-13",
        "Expression Controls" => "WP-11",
        "Distort" if matches!(name, "Bezier Warp" | "Mesh Warp" | "Liquify" | "Corner Pin" | "Polar Coordinates" | "Spherize" | "Magnify" | "Offset" | "Reshape" | "Smear" | "Rolling Shutter Repair") => "WP-34",
        "Perspective" if name.starts_with("Bevel") => "WP-31",
        "Text" => "WP-32",
        "Utility" if name == "Grow Bounds" => "WP-01",
        _ => "WP-33",
    }
}

/// One thing the system cannot do yet, or has decided not to do.
pub struct Gap {
    pub name: String,
    pub category: String,
    pub status: String,
    pub wp: String,
    pub ae_ref: String,
}

/// Every row whose status says it does not work: `none`, `todo`, `unverified` (work still to do) and `not_supported`.
pub fn gaps(ledger: &Ledger) -> Vec<Gap> {
    let mut out = Vec::new();
    let m = &ledger.matrix;
    for r in &m.rows {
        if matches!(m.val(r, "status"), "none" | "todo" | "unverified" | "not_supported") {
            out.push(Gap { name: m.val(r, "feature").into(), category: m.val(r, "category").into(), status: m.val(r, "status").into(), wp: m.val(r, "wp").into(), ae_ref: m.val(r, "ae_ref").into() });
        }
    }
    let e = &ledger.effects;
    for r in &e.rows {
        if matches!(e.val(r, "status"), "none" | "todo" | "not_supported") {
            out.push(Gap { name: e.val(r, "name").into(), category: format!("effects/{}", e.val(r, "category")), status: e.val(r, "status").into(), wp: effect_wp(e.val(r, "category"), e.val(r, "name")).into(), ae_ref: String::new() });
        }
    }
    out
}

#[cfg(test)]
pub mod fixture {
    use std::{fs, path::PathBuf};

    use super::{EFFECT_COLUMNS, MATRIX_COLUMNS};

    /// A fresh `<tmp>/<tag>/parity` folder with the two tables written from the rows given.
    pub fn write(tag: &str, matrix: &[&str], effects: &[&str]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("tm-parity-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = root.join("parity");
        for d in ["specs", "probes", "golden"] {
            fs::create_dir_all(dir.join(d)).unwrap();
        }
        let table = |cols: &[&str], rows: &[&str]| format!("{}\n{}\n", cols.join("\t"), rows.join("\n"));
        fs::write(dir.join("MATRIX.tsv"), table(&MATRIX_COLUMNS, matrix)).unwrap();
        fs::write(dir.join("effects.tsv"), table(&EFFECT_COLUMNS, effects)).unwrap();
        dir
    }
}
