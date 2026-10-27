//! A text layer's own `box` is what its type sits on: the contrast rule reads the box's colour as the background.

use crate::{anim::prop::Rgba, eval::text::TextState};

/// The colour the type of `text` is read against, given what is `behind` the layer. The box is there whenever the layer
/// has one, even at a moment no character shows (the first frame of a typed bar), and counts for its own opacity.
pub fn behind(text: &TextState, behind: Rgba) -> Rgba {
    let Some(b) = &text.text_box else { return behind };
    let a = b.fill[3].clamp(0.0, 1.0);
    let mix = |i: usize| b.fill[i] * a + behind[i] * (1.0 - a);
    [mix(0), mix(1), mix(2), behind[3]]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        eval::{Content, Evaluator},
        plan::{Plan, ROOT},
    };

    fn text_state(fill: &str) -> TextState {
        let layer = json!({"type": "text", "text": "Hi", "box": {"fit": "text", "fill": fill}});
        let plan = Plan::from_value(json!({"width": 200, "height": 100, "dur": 1, "layers": [layer]}), std::path::Path::new(".")).unwrap();
        let Content::Text(t) = Evaluator::new(&plan).frame(ROOT, 0.0).unwrap().layers.remove(0).content else { panic!("the layer is text") };
        t
    }

    #[test]
    fn the_type_is_read_against_the_box_by_its_colour_and_opacity() {
        let on_black = [0.0, 0.0, 0.0, 1.0];
        assert_eq!(behind(&text_state("#e2c48a"), on_black), [226.0, 196.0, 138.0, 1.0]);
        let half = behind(&text_state("#ffffff80"), on_black);
        assert!((half[0] - 255.0 * 128.0 / 255.0).abs() < 0.01 && half[3] == 1.0, "{half:?}");
    }
}
