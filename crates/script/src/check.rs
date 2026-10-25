//! Invoke an installed TypeScript compiler against the film and the selected on-disk SDK.
//! The checker never installs packages or executes the film.
use std::{fs, path::Path, process::Command, sync::atomic::{AtomicU64, Ordering}};
use std::path::PathBuf;
use anyhow::{ensure, Context, Result};
use serde_json::json;
use crate::loader::{sdk_directory, SDK};

static NEXT_CONFIG: AtomicU64 = AtomicU64::new(0);
pub struct Report { pub success: bool, pub diagnostics: String }

pub fn check(file: &Path, sdk: Option<&Path>, compiler: &Path) -> Result<Report> {
    let file = fs::canonicalize(file).with_context(|| format!("reading film {}", file.display()))?;
    let dir = std::env::temp_dir().join("turbomotion-typecheck").join(format!("{}-{}", std::process::id(), NEXT_CONFIG.fetch_add(1, Ordering::Relaxed)));
    fs::create_dir_all(&dir)?;
    let sdk = match sdk_directory(sdk)? {
        Some(dir) => dir,
        None => materialise_embedded_sdk(&dir.join("sdk"))?,
    };
    let config = dir.join("tsconfig.json");
    fs::write(&config, serde_json::to_vec_pretty(&json!({"files":[file],"compilerOptions":{
        "noEmit":true,"strict":true,"target":"ES2022","module":"ESNext","moduleResolution":"Bundler",
        "allowImportingTsExtensions":true,"types":[],"lib":["ES2022","DOM"],
        "paths":{"turbomotion":[sdk.join("index.ts")],"turbomotion/*":[sdk.join("*")]}
    }}))?)?;
    let mut process = if compiler.extension().is_some_and(|ext| ext == "js") {
        let mut node = Command::new("node"); node.arg(compiler); node
    } else { Command::new(compiler) };
    let output = process.args(["--project", config.to_str().context("typecheck config path is not UTF-8")?, "--pretty", "false"])
        .output().with_context(|| format!("running TypeScript compiler {}; install TypeScript locally or pass --tsc <path-to-tsc-or-tsc.js>; no packages are downloaded automatically", compiler.display()))?;
    let diagnostics = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    ensure!(output.status.success() || !diagnostics.trim().is_empty(), "TypeScript compiler exited {} without diagnostics; check --tsc", output.status);
    Ok(Report { success: output.status.success(), diagnostics })
}

/// Write the SDK compiled into this binary to `target`, so `tsc` checks the same sources the film will run against.
fn materialise_embedded_sdk(target: &Path) -> Result<PathBuf> {
    for (name, source) in SDK {
        let path = target.join(format!("{name}.ts"));
        fs::create_dir_all(path.parent().context("SDK module path has no parent")?)?;
        fs::write(&path, source)?;
    }
    Ok(target.to_path_buf())
}
