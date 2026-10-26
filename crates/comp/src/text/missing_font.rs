//! The message for a font name that is not installed, not a declared role and not a file: everything the author can
//! do next, with the real names to copy.

use std::path::{Path, PathBuf};

use turbomotion_core::suggest::suggest;

use super::layout::FontBook;

const FONT_FILES: [&str; 5] = ["ttf", "otf", "ttc", "otc", "woff"];
/// How many installed families an error lists when none is close to the name.
const SAMPLE: usize = 12;

/// Why `family` did not resolve and how to fix it. `installed` is the sorted list of installed family names.
pub fn explain(book: &dyn FontBook, family: &str, installed: &[String]) -> String {
    let names: Vec<&str> = installed.iter().map(String::as_str).collect();
    let mut out = format!("{family:?} is not an installed font family, a declared font role or a font file.");
    match suggest(family, &names) {
        Some(near) => out.push_str(&format!(" Did you mean the installed family {near:?}?")),
        None => out.push_str(&format!(" {} families are installed (`turbomotion comp fonts` lists them), for example {}.", installed.len(), installed.iter().take(SAMPLE).map(|f| format!("{f:?}")).collect::<Vec<_>>().join(", "))),
    }
    let roles = book.roles();
    out.push_str(&format!(" Declared roles: {}.", if roles.is_empty() { "none".to_string() } else { roles.join(", ") }));
    let files = font_files(&book.resolve(""));
    let squashed = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
    let wanted = squashed(family);
    match files.iter().find(|f| !wanted.is_empty() && squashed(&f.stem).starts_with(&wanted)) {
        Some(f) => out.push_str(&format!(" A file that looks like it is in the film's root: declare it with comp({{ fonts: {{ {family:?}: {:?} }} }}) and keep using the name.", f.rel)),
        None if files.is_empty() => out.push_str(" To use a font file, declare it with comp({ fonts: { \"Name\": \"path/to/Font.woff\" } }) (paths are relative to the film's root); no font files were found directly in the root or its fonts/ folder."),
        None => out.push_str(&format!(" Font files in the film's root: {}. Declare one with comp({{ fonts: {{ \"Name\": \"<file>\" }} }}).", files.iter().take(SAMPLE).map(|f| f.rel.clone()).collect::<Vec<_>>().join(", "))),
    }
    out
}

struct FontFile {
    /// Path relative to the root, as it would be written in `fonts`.
    rel: String,
    stem: String,
}

/// Font files directly in `root` and in `root/fonts`, sorted.
fn font_files(root: &Path) -> Vec<FontFile> {
    let mut found: Vec<FontFile> = Vec::new();
    for sub in ["", "fonts"] {
        let Ok(entries) = std::fs::read_dir(root.join(sub)) else { continue };
        let mut paths: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            let is_font = p.extension().and_then(|e| e.to_str()).is_some_and(|e| FONT_FILES.contains(&e.to_ascii_lowercase().as_str()));
            let (Some(name), Some(stem)) = (p.file_name().and_then(|n| n.to_str()), p.file_stem().and_then(|n| n.to_str())) else { continue };
            if is_font {
                found.push(FontFile { rel: if sub.is_empty() { name.to_string() } else { format!("{sub}/{name}") }, stem: stem.to_string() });
            }
        }
    }
    found
}
