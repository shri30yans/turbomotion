//! `sdk watch`: re-render a contact sheet whenever the script, anything it imports or reads, or the chosen SDK tree
//! changes. Plain polling of modification times: no extra dependencies, and the same edits always trigger the same renders.
use std::{collections::BTreeMap, fs, path::{Path, PathBuf}, thread::sleep, time::{Duration, SystemTime}};

use anyhow::Result;

use crate::cli::{build_written, write_sheet, Script};

type Stamp = Option<(SystemTime, u64)>;
/// Modification time and size of each file (`None` when it is missing).
pub type Snapshot = BTreeMap<PathBuf, Stamp>;

pub fn snapshot(files: &[PathBuf]) -> Snapshot {
    files.iter().map(|f| (f.clone(), fs::metadata(f).ok().and_then(|m| Some((m.modified().ok()?, m.len()))))).collect()
}

pub struct Watch<'a> {
    pub script: &'a Script,
    pub sheet: &'a Path,
    pub times: &'a [f64],
    pub interval: Duration,
    /// Stop after this many renders (tests, `--once`); `None` watches until interrupted.
    pub max_renders: Option<u32>,
}

/// Render now and after every change. A failed build prints its error and keeps watching the files it knew about.
pub fn run(w: &Watch) -> Result<()> {
    let mut files = vec![fs::canonicalize(&w.script.file)?];
    let mut renders = 0;
    loop {
        let before = snapshot(&files);
        match build_written(w.script, None).and_then(|(comp, read)| write_sheet(&comp, w.sheet, w.times).map(|()| read)) {
            Ok(read) => { files = read; let film = fs::canonicalize(&w.script.file)?; if !files.contains(&film) { files.push(film); } }
            Err(error) => eprintln!("{error:#}"),
        }
        renders += 1;
        if w.max_renders.is_some_and(|max| renders >= max) { return Ok(()); }
        // what was watched, as it is now; a file edited while we were building has already changed against `before`
        let seen = snapshot(&files);
        let edited_meanwhile = before.iter().any(|(file, stamp)| seen.get(file).is_some_and(|now| now != stamp));
        if !edited_meanwhile {
            // wait for a change, then for the edit to settle (an editor may write several files)
            let mut now = snapshot(&files);
            while now == seen { sleep(w.interval); now = snapshot(&files); }
            loop { sleep(w.interval); let again = snapshot(&files); if again == now { break; } now = again; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_snapshot_changes_when_a_file_is_edited_created_or_removed() {
        let dir = std::env::temp_dir().join(format!("tm-watch-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (a, b) = (dir.join("a.ts"), dir.join("b.ts"));
        fs::write(&a, "1").unwrap();
        let files = [a.clone(), b.clone()];
        let first = snapshot(&files);
        assert_eq!(first, snapshot(&files));
        fs::write(&a, "22").unwrap();
        assert_ne!(first, snapshot(&files));
        let second = snapshot(&files);
        fs::write(&b, "x").unwrap();
        assert_ne!(second, snapshot(&files));
        fs::remove_file(&b).unwrap();
        assert_eq!(second, snapshot(&files));
    }
}
