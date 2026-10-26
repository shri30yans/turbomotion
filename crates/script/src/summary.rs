//! The one line `sdk build` and `sdk render` print about what a script produced.

use serde_json::Value;

/// Layers a built film holds, counted where authors put them: the film itself, every shot, every named composition.
#[derive(Debug, PartialEq, Eq)]
pub struct Counts {
    pub shots: usize,
    pub layers: usize,
}

fn len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map_or(0, Vec::len)
}

pub fn count(film: &Value) -> Counts {
    let shots = film.pointer("/timeline/shots").and_then(Value::as_array);
    let in_shots = shots.map_or(0, |all| all.iter().map(|s| len(s.get("layers"))).sum());
    let in_comps = film.get("comps").and_then(Value::as_object).map_or(0, |all| all.values().map(|c| len(c.get("layers"))).sum());
    Counts { shots: shots.map_or(0, Vec::len), layers: len(film.get("layers")) + in_shots + in_comps }
}

pub fn summary(film: &Value) -> String {
    let n = |k: &str| film.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    let Counts { shots, layers } = count(film);
    let dur = if n("dur") > 0.0 { format!("{:.2}s", n("dur")) } else { "timeline length".into() };
    format!("{}x{} at {} fps, {dur}, {shots} shots, {layers} layers", n("width"), n("height"), n("fps"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn layers_are_counted_in_the_film_its_shots_and_its_compositions() {
        let film = json!({"width": 8, "height": 4, "fps": 10, "layers": [{}],
            "comps": {"card": {"layers": [{}, {}]}},
            "timeline": {"shots": [{"id": "a", "layers": [{}, {}, {}]}, {"id": "b"}]}});
        assert_eq!(count(&film), Counts { shots: 2, layers: 6 });
        assert_eq!(summary(&film), "8x4 at 10 fps, timeline length, 2 shots, 6 layers");
    }
}
