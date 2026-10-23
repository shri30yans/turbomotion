//! Review tools: one still per shot, and the layout QA that replaces eyeballing every shot.

use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::Path,
};

use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

use crate::{
    film::Shot,
    probe::{self, TextBox},
    render::{Project, Renderer},
};

/// The shots named in `only`, or all of them; an unknown id is an error.
fn select<'a>(project: Project<'a>, only: &[String]) -> Result<Vec<&'a Shot>> {
    for id in only {
        ensure!(project.film.shots.iter().any(|s| &s.id == id), "no shot {id:?} in the film");
    }
    Ok(project.film.shots.iter().filter(|s| only.is_empty() || only.contains(&s.id)).collect())
}

/// Write `<dir>/<shot id>.png` for each selected shot, `at` seconds into it (default: settled).
pub fn stills(project: Project, dir: &Path, only: &[String], at: Option<f32>) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let mut renderer = Renderer::new(project)?;
    for shot in select(project, only)? {
        renderer.draw(shot.still_frame(at, project.film.fps))?;
        let path = dir.join(format!("{}.png", shot.id));
        fs::write(&path, renderer.png()?).with_context(|| format!("writing {}", path.display()))?;
        println!("still {}", shot.id);
    }
    Ok(())
}

/// How far from the frame's edge text may sit.
const SAFE_MARGIN: f32 = 24.0;
/// Overlapping text only matters beyond this many pixels in both directions...
const OVERLAP_MIN: f32 = 4.0;
/// ...and beyond this share of the smaller box.
const OVERLAP_SHARE: f32 = 0.12;
/// Where in a shot the layout is judged, as fractions of its length.
const PROBES: [f32; 3] = [0.4, 0.75, 0.97];
/// Every n-th shot (from the 5th) is redrawn after another frame to prove drawing is stateless.
const DETERMINISM_EVERY: usize = 9;
/// How far ahead the determinism check wanders before coming back.
const WANDER_FRAMES: u32 = 45;

fn layout_issues(boxes: &[TextBox], size: (f32, f32)) -> Vec<String> {
    let visible: Vec<&TextBox> = boxes
        .iter()
        .filter(|b| b.rect.width() > 2.0 && b.rect.height() > 2.0 && b.rect.right > 0.0 && b.rect.bottom > 0.0 && b.rect.left < size.0 && b.rect.top < size.1)
        .collect();
    let mut issues = Vec::new();
    for b in &visible {
        let r = b.rect;
        if r.left < SAFE_MARGIN || r.top < SAFE_MARGIN || r.right > size.0 - SAFE_MARGIN || r.bottom > size.1 - SAFE_MARGIN {
            issues.push(format!("outside-safe: {:?} [{},{},{},{}]", b.text, r.left.round(), r.top.round(), r.right.round(), r.bottom.round()));
        }
    }
    for (i, a) in visible.iter().enumerate() {
        for b in &visible[i + 1..] {
            let w = a.rect.right.min(b.rect.right) - a.rect.left.max(b.rect.left);
            let h = a.rect.bottom.min(b.rect.bottom) - a.rect.top.max(b.rect.top);
            let smaller = (a.rect.width() * a.rect.height()).min(b.rect.width() * b.rect.height());
            if w > OVERLAP_MIN && h > OVERLAP_MIN && w * h > OVERLAP_SHARE * smaller {
                issues.push(format!("overlap: {:?} × {:?}", a.text, b.text));
            }
        }
    }
    issues
}

fn frame_hash(renderer: &mut Renderer, frame: u32, pixels: &mut Vec<u8>) -> Result<u64> {
    renderer.draw(frame)?;
    renderer.pixels(pixels)?;
    let mut hasher = DefaultHasher::new();
    pixels.hash(&mut hasher);
    Ok(hasher.finish())
}

/// Probe text layout at three moments per shot and check that sampled frames redraw identically;
/// write the report to `report` and return how many frames had layout issues.
pub fn qa(project: Project, report: &Path, only: &[String]) -> Result<usize> {
    let film = project.film;
    let size = (film.width as f32, film.height as f32);
    let mut renderer = Renderer::new(project)?;
    let (mut issues, mut determinism): (Vec<Value>, Vec<Value>) = (Vec::new(), Vec::new());

    for shot in select(project, only)? {
        for fraction in PROBES {
            let frame = shot.start_frame + (shot.duration_in_frames as f32 * fraction).floor() as u32;
            let (drawn, boxes) = probe::record(|| renderer.draw(frame));
            drawn?;
            let found = layout_issues(&boxes, size);
            if !found.is_empty() {
                issues.push(json!({"shot": shot.id, "type": shot.kind, "args": shot.args.join(" "), "frame": frame, "issues": found}));
            }
        }
    }
    if only.is_empty() {
        let mut pixels = Vec::new();
        for shot in film.shots.iter().skip(DETERMINISM_EVERY / 2).step_by(DETERMINISM_EVERY) {
            let frame = shot.start_frame + (shot.duration_in_frames as f32 * 0.6).floor() as u32;
            let first = frame_hash(&mut renderer, frame, &mut pixels)?;
            frame_hash(&mut renderer, (frame + WANDER_FRAMES).min(film.duration_in_frames - 1), &mut pixels)?;
            let again = frame_hash(&mut renderer, frame, &mut pixels)?;
            determinism.push(json!({"shot": shot.id, "frame": frame, "identical": first == again}));
        }
    }
    let identical = determinism.iter().filter(|d| d["identical"] == true).count();
    let summary = format!("qa: {} frames with layout issues; determinism {identical}/{} identical", issues.len(), determinism.len());
    fs::write(report, serde_json::to_string_pretty(&json!({"issues": issues, "determinism": determinism}))?).with_context(|| format!("writing {}", report.display()))?;
    println!("{summary}");
    Ok(issues.len())
}

#[cfg(test)]
mod tests {
    use skia_safe::Rect;

    use super::*;

    fn boxed(text: &str, l: f32, t: f32, r: f32, b: f32) -> TextBox {
        TextBox { text: text.into(), rect: Rect::new(l, t, r, b) }
    }

    #[test]
    fn flags_text_near_the_edge_and_text_on_text() {
        let size = (1920.0, 1080.0);
        let ok = [boxed("a", 100.0, 100.0, 300.0, 140.0), boxed("b", 100.0, 200.0, 300.0, 240.0)];
        assert!(layout_issues(&ok, size).is_empty());
        let edge = [boxed("tag", 10.0, 100.0, 300.0, 140.0)];
        assert_eq!(layout_issues(&edge, size).len(), 1);
        let clash = [boxed("a", 100.0, 100.0, 300.0, 140.0), boxed("b", 150.0, 110.0, 350.0, 150.0)];
        assert_eq!(layout_issues(&clash, size), ["overlap: \"a\" × \"b\""]);
    }

    #[test]
    fn ignores_text_off_screen_and_slivers() {
        let size = (1920.0, 1080.0);
        let parked = [boxed("far", 3000.0, 100.0, 3200.0, 140.0), boxed("also far", 3050.0, 100.0, 3250.0, 140.0)];
        assert!(layout_issues(&parked, size).is_empty());
        let touching = [boxed("a", 100.0, 100.0, 300.0, 140.0), boxed("b", 298.0, 100.0, 500.0, 140.0)];
        assert!(layout_issues(&touching, size).is_empty());
    }
}
