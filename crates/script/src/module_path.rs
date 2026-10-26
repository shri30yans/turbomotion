//! Where a relative import points on disk. Standard module resolution: `./x` is `x.ts`, then `x/index.ts`; a name that
//! already has an extension is used as written. Resolution touches the disk, so a miss can say exactly what was tried.

use std::path::{Component, Path, PathBuf};

/// The file `name` (as imported from the module at `importer`) refers to, or every path that was tried.
pub fn resolve(importer: &Path, name: &str) -> Result<PathBuf, Vec<PathBuf>> {
    let dir = importer.parent().unwrap_or(Path::new("."));
    let target = normalise(&dir.join(name));
    let tried = candidates(&target, name);
    tried.iter().find(|p| p.is_file()).cloned().ok_or(tried)
}

fn candidates(target: &Path, name: &str) -> Vec<PathBuf> {
    if Path::new(name).extension().is_some() {
        return vec![target.to_path_buf()];
    }
    let mut file = target.as_os_str().to_owned();
    file.push(".ts");
    vec![PathBuf::from(file), target.join("index.ts")]
}

/// Collapse `.` and `..` without touching the disk (module names must be stable keys).
pub fn normalise(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// The message for an import that matched nothing.
pub fn not_found(name: &str, tried: &[PathBuf]) -> String {
    let list = tried.iter().map(|p| format!("  {}", p.display())).collect::<Vec<_>>().join("\n");
    format!("cannot find module \"{name}\"; looked for:\n{list}")
}
