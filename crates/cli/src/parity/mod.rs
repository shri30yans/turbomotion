//! `turbomotion parity`: the ledger of what the engine can do against what After Effects can do (BUILD_PLAN ADR-6).
//!
//! * `check`       fail when the ledger and the catalogue disagree (run by every work package's gate)
//! * `report`      coverage per category
//! * `gaps`        what is missing or decided against, per feature (the page `docs/reference/parity.md` is this list)
//! * `init-effect` scaffold the files for a new effect and its ledger row

use std::{collections::BTreeSet, fmt::Write as _, path::{Path, PathBuf}};

use anyhow::{bail, Result};
use clap::{Args, Subcommand};
use serde_json::json;

pub mod check;
pub mod init_effect;
pub mod ledger;
pub mod report;

use ledger::{gaps, Gap, Ledger};

#[derive(Args)]
pub struct Where {
    /// The ledger folder (MATRIX.tsv, effects.tsv, specs/, probes/, golden/)
    #[arg(long, default_value = "parity")]
    dir: PathBuf,
}

#[derive(Subcommand)]
pub enum Command {
    /// Fail (exit 1) when the ledger and the engine disagree: a done row without test or doc, an unknown engine effect, a catalogue effect with no row
    Check(Where),
    /// Coverage per category, and the T0+T1 effects yardstick
    Report(Where),
    /// What cannot be done yet or is decided against, per feature
    Gaps {
        #[command(flatten)]
        at: Where,
        /// JSON instead of text
        #[arg(long)]
        json: bool,
    },
    /// Scaffold a new effect: shader header, probe film, spec template, test stub and the ledger row
    InitEffect {
        /// The engine name (snake_case)
        name: String,
        /// The AE name of an existing `todo` row to take over (exact, as in effects.tsv)
        #[arg(long)]
        ae: Option<String>,
        /// Category for a new row
        #[arg(long, default_value = "Engine-only")]
        category: String,
        #[command(flatten)]
        at: Where,
    },
}

/// The effect names the catalogue really contains.
pub fn catalogue_effects() -> BTreeSet<String> {
    let c = turbomotion_comp::catalogue::all();
    c["effects"].as_array().into_iter().flatten().filter_map(|e| e["name"].as_str().map(str::to_string)).collect()
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Check(at) => {
            let ledger = Ledger::load(&at.dir)?;
            let problems = check::check(&ledger, &catalogue_effects());
            if !problems.is_empty() {
                for p in &problems {
                    eprintln!("{p}");
                }
                bail!("parity check: {} problem(s)", problems.len());
            }
            println!("parity check: ok ({} core rows, {} effect rows, {} catalogue effects)", ledger.matrix.rows.len(), ledger.effects.rows.len(), catalogue_effects().len());
        }
        Command::Report(at) => print!("{}", report::render(&Ledger::load(&at.dir)?)),
        Command::Gaps { at, json } => {
            let list = gaps(&Ledger::load(&at.dir)?);
            if json {
                let rows: Vec<_> = list.iter().map(|g| json!({"name": g.name, "category": g.category, "status": g.status, "wp": g.wp, "ae_ref": g.ae_ref})).collect();
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else {
                for g in &list {
                    println!("{}", line(g));
                }
            }
        }
        Command::InitEffect { name, ae, category, at } => {
            for p in init_effect::scaffold(&at.dir, &name, ae.as_deref(), &category)? {
                println!("wrote {}", p.display());
            }
            println!("next: fill in parity/specs/{name}.md, write the failing tests, then implement (BUILD_PLAN section 4, steps 2-4)");
        }
    }
    Ok(())
}

fn line(g: &Gap) -> String {
    let what = if g.status == "not_supported" { "not supported (decided)" } else { "not built yet" };
    format!("{} | {} | {what}{}", g.category, g.name, if g.wp.is_empty() { String::new() } else { format!(" | {}", g.wp) })
}

/// The reference page `parity.md`: every gap, grouped, so an author can see "not built" before writing a film that needs it.
/// `None` when the ledger folder is not there (the docs are then written without it).
pub fn page(dir: &Path) -> Option<String> {
    if !dir.join("MATRIX.tsv").exists() {
        return None;
    }
    let ledger = Ledger::load(dir).ok()?;
    let mut s = String::new();
    let _ = writeln!(s, "# Not supported and not built\n\nGenerated from `parity/MATRIX.tsv` and `parity/effects.tsv`. `turbomotion parity gaps` prints the same list; `turbomotion parity report` counts coverage.\n");
    for (status, title) in [("not_supported", "Decided against (with reasons in docs/BUILD_PLAN.md section 2)"), ("rest", "Not built yet")] {
        let _ = writeln!(s, "## {title}\n");
        let mut by_cat: std::collections::BTreeMap<String, Vec<&Gap>> = Default::default();
        let all = gaps(&ledger);
        for g in &all {
            if (g.status == "not_supported") == (status == "not_supported") {
                by_cat.entry(g.category.clone()).or_default().push(g);
            }
        }
        let list = by_cat;
        for (cat, items) in list {
            let names: Vec<String> = items.iter().map(|g| if g.wp.is_empty() { g.name.clone() } else { format!("{} ({})", g.name, g.wp) }).collect();
            let _ = writeln!(s, "* **{cat}**: {}", names.join(", "));
        }
        s.push('\n');
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_lists_gaps_by_category_and_separates_decisions() {
        let dir = ledger::fixture::write("page", &["Roto Brush\tmodel\tnot_supported\t-\t-\t-\t-", "Spatial bezier\tmotion\tnone\t-\t-\t-\tWP-10"], &["Box Blur\tBlur & Sharpen\tB\tT0\ttodo\t?\t?\t-\t-\t-"]);
        let text = page(&dir).unwrap();
        assert!(text.contains("## Decided against"), "{text}");
        assert!(text.contains("* **model**: Roto Brush"), "{text}");
        assert!(text.contains("* **motion**: Spatial bezier (WP-10)"), "{text}");
        assert!(text.contains("* **effects/Blur & Sharpen**: Box Blur (WP-02)"), "{text}");
        assert!(page(&dir.join("missing")).is_none());
    }
}
