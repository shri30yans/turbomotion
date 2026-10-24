//! Layout QA instrumentation: while a recording is active, drawn text reports where it landed
//! and is kept only if it was mostly visible (the product of the group opacities around it).

use std::cell::RefCell;

use skia_safe::Rect;

/// A line of text as drawn, in frame pixels.
#[derive(Debug, Clone)]
pub struct TextBox {
    pub text: String,
    pub rect: Rect,
}

/// Text fainter than this counts as not yet on screen.
const VISIBLE: f32 = 0.5;

#[derive(Default)]
struct Recording {
    fades: Vec<f32>,
    boxes: Vec<TextBox>,
}

thread_local! {
    static ACTIVE: RefCell<Option<Recording>> = const { RefCell::new(None) };
}

/// Run `draw` and return the visible text it drew.
pub fn record<T>(draw: impl FnOnce() -> T) -> (T, Vec<TextBox>) {
    ACTIVE.with(|a| *a.borrow_mut() = Some(Recording::default()));
    let result = draw();
    let recording = ACTIVE.with(|a| a.borrow_mut().take()).unwrap_or_default();
    (result, recording.boxes)
}

/// Run `draw` inside a group faded to `opacity`.
pub fn fade<T>(opacity: f32, draw: impl FnOnce() -> T) -> T {
    ACTIVE.with(|a| a.borrow_mut().as_mut().map(|r| r.fades.push(opacity)));
    let result = draw();
    ACTIVE.with(|a| a.borrow_mut().as_mut().map(|r| r.fades.pop()));
    result
}

/// Report text drawn at `rect` (frame pixels).
pub fn text(text: &str, rect: Rect) {
    ACTIVE.with(|a| {
        if let Some(r) = a.borrow_mut().as_mut() {
            if r.fades.iter().product::<f32>() > VISIBLE {
                r.boxes.push(TextBox { text: text.to_owned(), rect });
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_text_that_is_mostly_visible() {
        let rect = Rect::from_wh(10.0, 10.0);
        let ((), boxes) = record(|| {
            text("plain", rect);
            fade(0.8, || {
                text("mostly", rect);
                fade(0.5, || text("faint", rect));
            });
            fade(0.2, || text("hidden", rect));
        });
        assert_eq!(boxes.iter().map(|b| b.text.as_str()).collect::<Vec<_>>(), ["plain", "mostly"]);
    }

    #[test]
    fn draws_outside_a_recording_are_ignored() {
        text("nobody listens", Rect::from_wh(1.0, 1.0));
        let ((), boxes) = record(|| ());
        assert!(boxes.is_empty());
    }
}
