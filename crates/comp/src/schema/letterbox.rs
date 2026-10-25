//! `"letterbox": [w, h]`: black bars that leave a `w x h` picture visible in the middle of the frame (a 4:3 reference
//! inside a 9:16 film, a 2.39:1 band in 16:9). The bars are ordinary solid layers, so `inspect` and `check` see them.
//!
//! With a `timeline` they sit above the shots and their transitions (so a push or dissolve never smears the bars) and
//! below overlays and the film's own layers. Without one they sit above every layer.

use anyhow::{ensure, Result};
use serde_json::{json, Value};

/// Layer ids of the bars, so a script can find or hide them.
pub const IDS: [&str; 2] = ["letterbox_a", "letterbox_b"];

/// The bars for a `width x height` frame that shows `visible` in the middle.
pub fn bars(width: f64, height: f64, visible: [f64; 2]) -> Result<Vec<Value>> {
    let [vw, vh] = visible;
    ensure!(vw > 0.0 && vh > 0.0, "letterbox must be [width, height] of the visible picture in pixels, both above 0, got [{vw}, {vh}]");
    ensure!(vw <= width && vh <= height, "letterbox [{vw}, {vh}] is larger than the {width}x{height} frame; it is the size of the picture that stays visible");
    let mut out = vec![];
    let (top, side) = ((height - vh) / 2.0, (width - vw) / 2.0);
    let bar = |id: &str, size: [f64; 2], pos: [f64; 2]| json!({"type": "solid", "id": id, "color": "#000000", "size": size, "pos": pos});
    if top > 0.0 {
        out.push(bar(IDS[0], [width, top], [width / 2.0, top / 2.0]));
        out.push(bar(IDS[1], [width, top], [width / 2.0, height - top / 2.0]));
    }
    if side > 0.0 {
        let ids = if top > 0.0 { ["letterbox_c", "letterbox_d"] } else { IDS };
        out.push(bar(ids[0], [side, height], [side / 2.0, height / 2.0]));
        out.push(bar(ids[1], [side, height], [width - side / 2.0, height / 2.0]));
    }
    Ok(out)
}

/// Read the root's `letterbox` and build its bars; `None` when the film has none.
pub fn of_root(root: &Value) -> Result<Option<Vec<Value>>> {
    let Some(lb) = root.get("letterbox").filter(|v| !v.is_null()) else { return Ok(None) };
    let arr = lb.as_array().filter(|a| a.len() == 2 && a.iter().all(Value::is_number));
    let Some(a) = arr else { anyhow::bail!("letterbox must be [width, height] (numbers), got {lb}") };
    let (w, h) = (root.get("width").and_then(Value::as_f64).unwrap_or(1920.0), root.get("height").and_then(Value::as_f64).unwrap_or(1080.0));
    bars(w, h, [a[0].as_f64().unwrap_or(0.0), a[1].as_f64().unwrap_or(0.0)]).map(Some)
}

/// For films without a timeline: put the bars on top of the existing layers.
pub fn apply_flat(root: &mut Value) -> Result<()> {
    if root.get("timeline").is_some() {
        return Ok(());
    }
    if let Some(bars) = of_root(root)? {
        let layers = root.as_object_mut().expect("a composition is an object").entry("layers").or_insert_with(|| json!([]));
        layers.as_array_mut().ok_or_else(|| anyhow::anyhow!("\"layers\" must be a list"))?.extend(bars);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_4_3_picture_in_9_16_gets_a_bar_above_and_below() {
        let b = bars(1080.0, 1920.0, [1080.0, 810.0]).unwrap();
        assert_eq!(b.len(), 2);
        assert_eq!(b[0]["size"], json!([1080.0, 555.0]));
        assert_eq!(b[0]["pos"], json!([540.0, 277.5]));
        assert_eq!(b[1]["pos"], json!([540.0, 1642.5]));
    }

    #[test]
    fn pillars_and_bad_sizes() {
        assert_eq!(bars(1920.0, 1080.0, [1440.0, 1080.0]).unwrap().len(), 2);
        assert_eq!(bars(1920.0, 1080.0, [1920.0, 1080.0]).unwrap().len(), 0);
        assert_eq!(bars(1920.0, 1080.0, [1000.0, 500.0]).unwrap().len(), 4);
        assert!(bars(1920.0, 1080.0, [2000.0, 500.0]).unwrap_err().to_string().contains("larger than"));
        assert!(of_root(&json!({"letterbox": [1, "x"]})).is_err());
    }
}
