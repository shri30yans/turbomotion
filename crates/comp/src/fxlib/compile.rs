//! Compiling a plugin: the generated declarations and the standard library go in front of the author's code, one program
//! per pass, and errors are reported against the author's own line numbers.

use std::ops::Range;

use anyhow::{anyhow, ensure, Result};
use turbomotion_core::shader::Program;

use super::{header::Header, Def, Kind, Pass, Ty, FULL_SIZE};

const PRELUDE: &str = include_str!("prelude.sksl");
/// Where the standard library's image helpers begin: generators have no input picture, so they do not get them.
const IMAGE_HELPERS: &str = "// The input layer at a point";

/// The standard library without the image helpers, for generators.
fn shared_prelude() -> &'static str {
    PRELUDE.split_once(IMAGE_HELPERS).expect("prelude.sksl has the image helpers marker (a test checks it)").0
}

/// One pass of a plugin: what it is called and which lines of the file are its own.
struct Section<'a> {
    name: &'a str,
    scale: f64,
    doc: &'a str,
    body: Range<usize>,
}

/// The file's passes: one per `@pass` marker, or a single `main` over the whole file when there are none.
fn sections(h: &Header, lines: usize) -> Vec<Section<'_>> {
    if h.marks.is_empty() {
        return vec![Section { name: "main", scale: FULL_SIZE, doc: "", body: 0..lines }];
    }
    h.marks.iter().enumerate().map(|(k, m)| Section { name: &m.name, scale: m.scale, doc: &m.doc, body: m.line + 1..h.marks.get(k + 1).map_or(lines, |next| next.line) }).collect()
}

/// The uniform declarations of pass `k`: the picture, the inputs, every earlier pass, then the platform's values and the
/// parameters.
fn declarations(h: &Header, earlier: &[Section]) -> String {
    let mut head = String::new();
    if h.kind == Kind::Effect {
        head.push_str("uniform shader image;\n");
    }
    for c in h.inputs.iter().flat_map(|i| i.children()) {
        head.push_str(&format!("uniform shader {c};\n"));
    }
    for s in earlier {
        head.push_str(&format!("uniform shader {};\n", s.name));
    }
    head.push_str("uniform float2 resolution;\nuniform float time;\nuniform float2 layer_origin;\nuniform float2 buffer_size;\n");
    for p in &h.params {
        // a colour is straight sRGB as written, and arrives in the encoding the shader computes in (`@space`)
        let (layout, ty) = match p.ty {
            Ty::Float | Ty::Choice => ("", "float"),
            Ty::Vec2 => ("", "float2"),
            Ty::Color => ("layout(color) ", "float4"),
        };
        head.push_str(&format!("{layout}uniform {ty} {};\n", p.name));
    }
    head
}

/// The file as pass `k` sees it: the other passes' lines are blanked, not removed, so the author's line numbers hold.
fn author_source(src: &str, sections: &[Section], k: usize) -> String {
    src.lines()
        .enumerate()
        .map(|(n, line)| if sections.len() > 1 && sections.iter().enumerate().any(|(j, s)| j != k && s.body.contains(&n)) { "" } else { line })
        .collect::<Vec<_>>()
        .join("\n")
}

impl Def {
    /// Parse the header, compile every pass, and check the header against the code.
    pub fn compile(src: &str, origin: &str) -> Result<Def> {
        let h = Header::parse(src)?;
        let sections = sections(&h, src.lines().count());
        let mut passes = Vec::with_capacity(sections.len());
        let mut used_uniforms: Vec<String> = vec![];
        for (k, s) in sections.iter().enumerate() {
            let library = if h.kind == Kind::Effect { PRELUDE } else { shared_prelude() };
            let prelude = format!("{}{library}\n", declarations(&h, &sections[..k]));
            let body = author_source(src, &sections, k);
            let program = Program::shader(&h.name, &format!("{prelude}{body}")).map_err(|e| anyhow!("{}", explain(&format!("{e:#}"), &body, prelude.lines().count())))?;
            used_uniforms.extend(program.uniform_names());
            passes.push(Pass { name: s.name.to_string(), scale: s.scale, doc: s.doc.to_string(), program });
        }
        for u in ["resolution", "time"].into_iter().chain(h.params.iter().map(|p| p.name.as_str())) {
            ensure!(used_uniforms.iter().any(|used| used == u), "uniform {u:?} is unused by the shader, so the compiler dropped it; use it or remove it from the header");
        }
        Ok(Def { name: h.name, kind: h.kind, summary: h.summary, params: h.params, inputs: h.inputs, origin: origin.to_string(), passes, example: h.example, bounds: h.bounds, space: h.space })
    }
}

/// Skia reports `error: <line>: ...` against the generated source; point at the author's line and show it.
fn explain(msg: &str, body: &str, offset: usize) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut out = String::new();
    for l in msg.lines() {
        let t = l.trim_start();
        let t = t.find("error: ").map_or(t, |i| &t[i..]);
        if let Some(rest) = t.strip_prefix("error: ") {
            if let Some((n, text)) = rest.split_once(": ") {
                if let Ok(n) = n.trim().parse::<usize>() {
                    // lines inside the generated prelude have no author line; show them as they are
                    let n = n.saturating_sub(offset);
                    if n == 0 {
                        out.push_str(&format!("(generated prelude) {text}\n"));
                        continue;
                    }
                    out.push_str(&format!("line {n}: {text}\n"));
                    if let Some(src) = lines.get(n.wrapping_sub(1)) {
                        out.push_str(&format!("    | {}\n", src.trim_end()));
                    }
                    continue;
                }
            }
        }
        out.push_str(l);
        out.push('\n');
    }
    out.trim_end().to_string()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prelude_splits_where_the_image_helpers_begin() {
        let shared = shared_prelude();
        assert!(!shared.is_empty() && shared.len() < PRELUDE.len());
        assert!(!shared.contains("image.eval"), "generators have no input picture to sample");
        assert!(PRELUDE.contains("image.eval"), "effects keep the helpers");
    }
}
