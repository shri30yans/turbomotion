//! Where shader plugins live besides the project's own `effects/` folder, so a library of shared effects is available
//! to every film without copying files. Folders are named explicitly; nothing searches parent folders.
//!
//! Folders are searched in this order; a later folder replaces an earlier plugin of the same name:
//!
//! 1. `TURBOMOTION_FX_PATH`, a `:`-separated list of folders;
//! 2. the composition's own `"fx_path": ["../shared/effects"]`, relative to the film's folder;
//! 3. the project's `effects/` (handled by the caller: it is strict, the others are not).
//!
//! Every folder is read recursively; a plugin is any `*.sksl` file in it.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

/// Environment variable holding extra plugin folders.
pub const ENV_PATH: &str = "TURBOMOTION_FX_PATH";

/// The folders to search for `dir` (the film's folder), lowest priority first, without the project's `effects/`.
pub fn include_dirs(dir: &Path, declared: &[String]) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = vec![];
    let mut push = |p: PathBuf| {
        if !out.contains(&p) {
            out.push(p);
        }
    };
    if let Ok(list) = env::var(ENV_PATH) {
        for entry in list.split(':').filter(|e| !e.is_empty()) {
            let p = PathBuf::from(entry);
            if !p.is_dir() {
                bail!("{ENV_PATH} lists {entry:?}, which is not a folder; fix or unset the variable");
            }
            push(p);
        }
    }
    for entry in declared {
        let p = if Path::new(entry).is_absolute() { PathBuf::from(entry) } else { dir.join(entry) };
        if !p.is_dir() {
            bail!("\"fx_path\" lists {entry:?}, which is not a folder (looked for {}); paths are relative to the composition's root (its own folder unless it sets \"root\"), like \"block_path\"", p.display());
        }
        push(p);
    }
    Ok(out)
}

/// Every `*.sksl` below `dir`, sorted, so loading is deterministic.
pub fn plugin_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = vec![];
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).with_context(|| format!("reading {}", d.display()))? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|x| x == "sksl") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let d = env::temp_dir().join(format!("tm-fxsearch-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn plugin_files_are_read_recursively_and_sorted() {
        let root = scratch("kit");
        fs::create_dir_all(root.join("post")).unwrap();
        fs::write(root.join("post/mosaic.sksl"), "x").unwrap();
        fs::write(root.join("top.sksl"), "x").unwrap();
        fs::write(root.join("notes.md"), "x").unwrap();
        let files: Vec<_> = plugin_files(&root).unwrap().iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(files, ["mosaic.sksl", "top.sksl"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_declared_folder_that_does_not_exist_is_named() {
        let root = scratch("declared");
        let e = include_dirs(&root, &["../nowhere".to_string()]).unwrap_err().to_string();
        assert!(e.contains("fx_path") && e.contains("../nowhere"), "{e}");
        fs::create_dir_all(root.join("shared")).unwrap();
        assert!(include_dirs(&root, &["shared".to_string()]).unwrap().contains(&root.join("shared")));
        let _ = fs::remove_dir_all(&root);
    }
}
