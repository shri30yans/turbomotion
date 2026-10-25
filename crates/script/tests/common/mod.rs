//! Scratch folders for tests. Each test process gets its own folder, so two `cargo test` runs sharing one target
//! directory (parallel agents, an IDE beside a terminal) never overwrite each other's film scripts or SDK trees.
#![allow(dead_code)]
use std::{fs, path::{Path, PathBuf}};

/// `target/tmp/<group>/<name>-<pid>`: created fresh, at the same depth for every test.
pub fn scratch(group: &str, name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(group).join(format!("{name}-{}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}
