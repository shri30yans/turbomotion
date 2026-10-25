//! The reference pages in `docs/reference` are generated; they must match what the code says today.

use std::{path::Path, process::Command};

#[test]
fn the_generated_reference_is_current() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new(env!("CARGO_BIN_EXE_turbomotion")).args(["docs", "--check"]).current_dir(&root).output().unwrap();
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
}
