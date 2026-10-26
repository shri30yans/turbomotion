//! `sdk check` without a TypeScript compiler: run the film, then validate what it emitted against the engine itself
//! (parse, plan, evaluate, lint) and point every complaint at the line of the script that created the layer.
use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result};
use turbomotion_comp::{api, check::Severity, schema::Problems};

use crate::{check::Report, run::{build_full, Options}};

pub const MODE: &str = "schema (no TypeScript compiler: the film was run and every layer checked against the engine)";

pub fn schema(file: &Path, sdk: Option<&Path>) -> Result<Report> {
    let options = Options { sdk_dir: sdk.map(Path::to_path_buf), ..Options::default() };
    let built = match build_full(file, &options, true) {
        Ok(built) => built,
        Err(error) => return Ok(Report { success: false, diagnostics: format!("{error:#}\n"), mode: MODE }),
    };
    let dir = std::fs::canonicalize(file)?.parent().context("film has no folder")?.to_path_buf();
    let mut out = String::new();
    let mut success = true;
    match api::validate(built.film, &dir) {
        Err(error) => {
            success = false;
            match error.downcast_ref::<Problems>() {
                // one line per mistake, each at the script line that made its layer
                Some(problems) => for p in &problems.0 {
                    let site = p.id.as_deref().and_then(|id| built.sites.get(id));
                    out.push_str(&format!("{}{}\n", site.map_or(String::new(), |s| format!("{s}: ")), p.message));
                },
                None => {
                    let message = format!("{error:#}");
                    let at = locate(&message, &built.sites).map_or(String::new(), |(id, site)| format!("{site}: layer \"{id}\": "));
                    out.push_str(&format!("{at}{message}\n"));
                }
            }
        }
        Ok(findings) => for f in findings {
            success &= f.severity != Severity::Error;
            let id = f.layer.as_deref().and_then(|l| l.rsplit('/').next());
            let site = id.and_then(|id| built.sites.get(id)).map_or(String::new(), |s| format!("{s}: "));
            out.push_str(&format!("{site}{f}\n"));
        },
    }
    Ok(Report { success, diagnostics: out, mode: MODE })
}

/// The layer an engine message is about: the longest known layer id that appears in it as a whole word.
fn locate<'a>(message: &str, sites: &'a BTreeMap<String, String>) -> Option<(&'a str, &'a str)> {
    let word = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
    sites.iter().filter(|(id, _)| message.match_indices(id.as_str()).any(|(i, m)| {
        !message[..i].chars().next_back().is_some_and(word) && !message[i + m.len()..].chars().next().is_some_and(word)
    })).max_by_key(|(id, _)| id.len()).map(|(id, site)| (id.as_str(), site.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_match_as_whole_words_and_the_longest_wins() {
        let sites: BTreeMap<String, String> = [("a", "film.ts:1:1"), ("a-body", "film.ts:2:1"), ("card", "film.ts:3:1")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        assert_eq!(locate("layer \"a-body\" has no size", &sites).unwrap().0, "a-body");
        assert_eq!(locate("bad cardinal value", &sites), None);
        assert_eq!(locate("the layer a: nope", &sites).unwrap().1, "film.ts:1:1");
    }
}
