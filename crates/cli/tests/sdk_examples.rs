//! Every ```` ```ts example ```` block of `docs/reference/sdk.md` is a whole film script. Each one is built with the SDK
//! and the film it makes is checked by the engine: zero findings, warnings included. A snippet that rots fails the build.

use std::{fs, path::{Path, PathBuf}, process::Command};

use turbomotion_script::{build, Options};

/// `(line of the first code line, code)` of every ```` ```ts example ```` block in `markdown`.
fn blocks(markdown: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut open: Option<(usize, String)> = None;
    for (n, line) in markdown.lines().enumerate() {
        match (&mut open, line) {
            (None, "```ts example") => open = Some((n + 2, String::new())),
            (Some(_), "```") => out.push(open.take().expect("a block is open")),
            (Some((_, code)), _) => {
                code.push_str(line);
                code.push('\n');
            }
            _ => {}
        }
    }
    assert!(open.is_none(), "a ```ts example block is never closed");
    out
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sdk-examples").join(format!("{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A photograph for an example that names one (4K: a 1080p film can zoom into it before it goes soft).
fn photo(dir: &Path) {
    let path = dir.join("photo.jpg");
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-nostdin", "-y", "-f", "lavfi", "-i", "testsrc2=size=3840x2160:rate=1", "-frames:v", "1"])
        .arg(&path)
        .output()
        .expect("ffmpeg (the engine needs it too)");
    assert!(out.status.success() && path.exists(), "{}", String::from_utf8_lossy(&out.stderr));
}

/// Build `code` as `film.ts` in its own folder, then let the engine check the film. `Err` is what a reader would see.
fn run(name: &str, code: &str) -> Result<(), String> {
    let dir = scratch(name);
    if code.contains("photo.jpg") {
        photo(&dir);
    }
    let file = dir.join("film.ts");
    fs::write(&file, code).unwrap();
    let film = build(&file, &Options::default()).map_err(|e| format!("{e:#}"))?;
    let findings = turbomotion_comp::api::validate(film, &dir).map_err(|e| format!("{e:#}"))?;
    if findings.is_empty() {
        Ok(())
    } else {
        Err(findings.iter().map(|f| f.to_string()).collect::<Vec<_>>().join("\n"))
    }
}

fn page() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/reference/sdk.md")).expect("docs/reference/sdk.md")
}

#[test]
fn every_example_on_the_sdk_page_builds_and_passes_check() {
    let found = blocks(&page());
    assert!(found.len() >= 15, "the SDK page has only {} runnable examples", found.len());
    let bad: Vec<String> = found.iter().filter_map(|(line, code)| run(&format!("line{line}"), code).err().map(|e| format!("sdk.md:{line}: {e}"))).collect();
    assert!(bad.is_empty(), "{} of {} SDK examples fail:\n{}", bad.len(), found.len(), bad.join("\n"));
}

#[test]
fn every_example_tag_in_the_sdk_sources_reaches_the_page() {
    let tagged: usize = turbomotion_script::sdk_sources().iter().map(|(_, src)| src.lines().filter(|l| l.trim() == "* @example").count()).sum();
    assert_eq!(blocks(&page()).len(), tagged, "an `@example` in a doc comment is missing from the page (an export without a doc comment is not listed): run `turbomotion docs --write`");
}

#[test]
fn a_wrong_snippet_fails_with_the_message_a_reader_needs() {
    let wrong = "import { comp, solid } from \"turbomotion\";\nconst film = comp({ width: 1920, height: 1080, fps: 30, dur: 3 });\nfilm.add(solid({ color: \"#fff\" } as any));\nexport default film;\n";
    let e = run("wrong-solid", wrong).unwrap_err();
    assert!(e.contains("solid(color, opts)"), "{e}");
    // a snippet that builds but trips the engine's lint fails too: tiny text outside the action-safe area
    let sloppy = "import { comp, text } from \"turbomotion\";\nconst film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: \"#1a1d29\" });\nfilm.add(text(\"Hello\", { id: \"t\", pos: [0, 0] }));\nexport default film;\n";
    assert!(run("sloppy", sloppy).is_err(), "lint findings must fail an example");
}

#[test]
fn the_block_finder_reads_only_runnable_blocks() {
    let md = "```ts\nexport const a = 1\n```\n\n```ts example\none();\ntwo();\n```\n\ntext\n\n```ts example\nthree();\n```\n";
    let found = blocks(md);
    assert_eq!(found, [(6, "one();\ntwo();\n".to_string()), (13, "three();\n".to_string())]);
}
