use anyhow::Result;
use serde_json::{json, Value};

use super::*;
use crate::anim::expr::{Arg, Env};

struct E;
impl Env for E {
    fn var(&self, n: &str) -> Option<f64> {
        if n == "time" { Some(0.5) } else { None }
    }
    fn call(&self, _: &str, _: &[Arg]) -> Option<Result<f64>> {
        None
    }
    fn salt(&self) -> u32 {
        1
    }
}

fn p(v: Value) -> Prop {
    Prop::parse(&v).unwrap()
}

#[test]
fn keys_hold_and_ease_into_the_key() {
    let k = p(json!([[1, 0], [2, 100, "power2.in"]]));
    assert_eq!(k.base(0.0), 0.0);
    assert_eq!(k.base(1.5), 12.5);
    assert_eq!(k.base(9.0), 100.0);
}

#[test]
fn tween_and_expression_on_top() {
    let t = p(json!({"from": 0, "to": 10, "at": 1, "dur": 2, "ease": "none"}));
    assert_eq!(t.base(2.0), 5.0);
    let e = p(json!({"value": [[0, 10], [1, 20]], "expr": "value * 2"}));
    assert_eq!(e.at(1.0, &E).unwrap(), 40.0);
    assert_eq!(p(json!("time * 4")).at(0.0, &E).unwrap(), 2.0);
}

#[test]
fn loops_cycle_pingpong_and_offset() {
    let c = p(json!({"keys": [[0, 0], [1, 10, "none"]], "loop": "cycle"}));
    assert_eq!(c.base(2.25), 2.5);
    let pp = p(json!({"keys": [[0, 0], [1, 10, "none"]], "loop": "pingpong"}));
    assert_eq!(pp.base(1.25), 7.5);
    let o = p(json!({"keys": [[0, 0], [1, 10, "none"]], "loop": "offset"}));
    assert_eq!(o.base(2.5), 25.0);
}

#[test]
fn mistakes_are_specific() {
    let e = Prop::parse(&json!([[2, 0], [1, 5]])).unwrap_err().to_string();
    assert!(e.contains("time order"), "{e}");
    let e = Prop::parse(&json!({"frm": 1})).unwrap_err().to_string();
    assert!(e.contains("valid fields"), "{e}");
    assert!(Prop::parse(&json!([[0, 0, "wobbly.out"]])).is_err());
}

#[test]
fn vectors_and_colours_animate() {
    let v = VecProp::parse(&json!({"from": [0, 0], "to": [100, 50], "at": 0, "dur": 1, "ease": "none"}), 2..=3).unwrap();
    assert_eq!(v.at(0.5, &E).unwrap(), vec![50.0, 25.0]);
    let v = VecProp::parse(&json!({"keys": [[0, [0, 0]], [1, [10, 20], "none"]]}), 2..=3).unwrap();
    assert_eq!(v.at(0.5, &E).unwrap(), vec![5.0, 10.0]);
    let c = ColorProp::parse(&json!({"from": "#000000", "to": "#ff0000", "dur": 1, "ease": "none"})).unwrap();
    assert_eq!(c.at(0.5, &E).unwrap()[0], 127.5);
    assert_eq!(parse_color_static(&json!("#f80")).unwrap(), [255.0, 136.0, 0.0, 1.0]);
}

#[test]
fn misplaced_vector_and_tween_lists_explain_both_forms() {
    let e = format!("{:#}", VecProp::parse(&json!([[0, [0, 0]], [1, [10, 20]], [2, [5, 5]]]), 2..=3).unwrap_err());
    assert!(e.contains("\"keys\"") && e.contains("\"from\""), "{e}");
    let e = format!("{:#}", VecProp::parse(&json!([[0, [0, 0]], [1, [10, 20]]]), 2..=2).unwrap_err());
    assert!(e.contains("wrapper"), "{e}");
    let e = format!("{:#}", Prop::parse(&json!([{"from": 0, "to": 1, "dur": 1}, {"from": 1, "to": 0, "at": 1}])).unwrap_err());
    assert!(e.contains("tween object") && !e.contains("needs \"t\""), "{e}");
    let e = format!("{:#}", Prop::parse(&json!([[0, [1, 2]], [1, [3, 4]]])).unwrap_err());
    assert!(e.contains("one number") && e.contains("keys"), "{e}");
    // per-axis tweens stay valid
    let v = VecProp::parse(&json!([{"from": 0, "to": 100, "dur": 1, "ease": "none"}, 5]), 2..=2).unwrap();
    assert_eq!(v.at(0.5, &E).unwrap(), vec![50.0, 5.0]);
}

#[test]
fn colour_keys_work_bare_or_inside_keys() {
    let bare = ColorProp::parse(&json!([[0, "#000000"], [1, "#ff0000", "none"]])).unwrap();
    let wrapped = ColorProp::parse(&json!({"keys": [[0, "#000000"], [1, "#ff0000", "none"]]})).unwrap();
    assert_eq!(bare.at(0.5, &E).unwrap(), wrapped.at(0.5, &E).unwrap());
    assert_eq!(wrapped.at(0.5, &E).unwrap()[0], 127.5);
    let e = ColorProp::parse(&json!({"colour": "#fff"})).unwrap_err().to_string();
    assert!(e.contains("keys"), "{e}");
}
