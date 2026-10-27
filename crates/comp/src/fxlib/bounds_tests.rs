//! The `@bounds` grammar and what it measures.

use super::*;
use crate::fxlib::Ty;

fn param(name: &str) -> Param {
    Param { name: name.to_string(), ty: Ty::Float, default: vec![1.0], optional: false, min: None, max: None, options: vec![], doc: String::new() }
}

/// A layer 200 px wide and 60 high.
const BOX: [f64; 4] = [10.0, 20.0, 210.0, 80.0];

fn growth(text: &str, blur: f64) -> Result<[f64; 4]> {
    let params = [param("blur"), param("reach")];
    Bounds::parse(text, &params, &["map"])?.growth(BOX, &|n| match n {
        "blur" => Some(blur),
        "reach" => Some(7.0),
        _ => None,
    })
}

#[test]
fn an_expression_over_parameters_gives_pixels_per_side() {
    assert_eq!(growth("expand=blur*3", 10.0).unwrap(), [30.0; 4]);
    assert_eq!(growth("expand_x=reach+(blur-2)/2", 10.0).unwrap(), [11.0, 0.0, 11.0, 0.0]);
    assert_eq!(growth("expand_y=max(blur,reach)", 3.0).unwrap(), [0.0, 7.0, 0.0, 7.0]);
    assert_eq!(growth("expand=-blur", 10.0).unwrap(), [0.0; 4], "growth never shrinks the layer");
    assert_eq!(growth("none", 10.0).unwrap(), [0.0; 4]);
}

#[test]
fn mistakes_name_the_problem_and_the_valid_choices() {
    let text = |t: &str| growth(t, 1.0).unwrap_err().to_string() + &format!("{:#}", growth(t, 1.0).unwrap_err());
    assert!(text("expand=blurr*3").contains("not a float parameter") && text("expand=blurr*3").contains("blur"), "{}", text("expand=blurr*3"));
    assert!(text("expand=2*").contains("expected a number"));
    assert!(text("expand=sqrt(2)").contains("no function"));
    assert!(text("grow=3").contains("unknown @bounds key"));
    assert!(text("expand=1 expand_x=2").contains("alone"));
    assert!(text("source=mp").contains("not a layer input"));
    assert!(text("").contains("`none`"));
    assert!(growth("expand=1/0", 1.0).unwrap_err().to_string().contains("not a finite number"));
}

#[test]
fn a_source_bound_parses_and_says_it_is_not_supported_when_asked_for_pixels() {
    let b = Bounds::parse("source=map", &[], &["map"]).unwrap();
    assert!(b.grows() && b.describe() == "source=map");
    assert!(b.growth(BOX, &|_| None).unwrap_err().to_string().contains("not support"));
}

#[test]
fn the_tag_reads_back_as_it_was_written() {
    let params = [param("blur")];
    for t in ["none", "expand=blur*3", "expand_x=blur expand_y=2"] {
        assert_eq!(Bounds::parse(t, &params, &[]).unwrap().describe(), t);
    }
    assert!(!Bounds::Undeclared.grows() && !Bounds::Within.grows());
}

#[test]
fn the_layers_own_width_and_height_can_be_part_of_an_expression() {
    // 300 % of a 200 x 60 layer is 400 px wider and 120 higher, so 200 and 60 a side
    assert_eq!(growth("expand_x=box_w*(3-1)/2 expand_y=box_h*(3-1)/2", 1.0).unwrap(), [200.0, 60.0, 200.0, 60.0]);
    assert_eq!(growth("expand=box_w*0.5+blur", 4.0).unwrap(), [104.0; 4]);
    assert_eq!(growth("expand_x=box_w*(0.5-1)/2", 1.0).unwrap(), [0.0; 4], "a layer that shrinks to half grows nothing");
}

#[test]
fn a_parameter_may_not_hide_the_layers_size() {
    let params = [param("box_w")];
    let e = Bounds::parse("expand=box_w", &params, &[]).unwrap_err().to_string();
    assert!(e.contains("box_w") && e.contains("rename the parameter"), "{e}");
}
