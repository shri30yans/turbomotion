//! TypeScript to JavaScript: types are removed (ES module syntax stays), and a source map is kept
//! so a runtime error can be reported against the line the author wrote.

use std::path::Path;

use anyhow::{bail, Result};
use oxc::{
    allocator::Allocator,
    codegen::{Codegen, CodegenOptions},
    diagnostics::OxcDiagnostic,
    parser::Parser,
    semantic::SemanticBuilder,
    span::SourceType,
    transformer::{TransformOptions, Transformer},
};

/// A stripped module: plain JavaScript plus the way back to the original lines.
#[derive(Debug)]
pub struct Stripped {
    pub js: String,
    map: Option<oxc_sourcemap::SourceMap>,
}

impl Stripped {
    /// The original 1-based `(line, column)` of a generated 1-based position, if the map knows it.
    pub fn original(&self, line: u32, col: u32) -> Option<(u32, u32)> {
        let map = self.map.as_ref()?;
        let table = map.generate_lookup_table();
        let tok = map.lookup_token(&table, line.saturating_sub(1), col.saturating_sub(1))?;
        Some((tok.get_src_line() + 1, tok.get_src_col() + 1))
    }
}

/// Strip the types from `source` (named `name` in messages). `.js` files pass through the same pipeline.
pub fn strip(name: &str, source: &str) -> Result<Stripped> {
    let path = Path::new(name);
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts()).with_module(true);
    let alloc = Allocator::default();
    let parsed = Parser::new(&alloc, source, source_type).parse();
    if !parsed.errors.is_empty() || parsed.panicked {
        bail!("{}", describe(name, source, &parsed.errors));
    }
    let mut program = parsed.program;
    let built = SemanticBuilder::new().build(&program);
    let scoping = built.semantic.into_scoping();
    let done = Transformer::new(&alloc, path, &TransformOptions::default()).build_with_scoping(scoping, &mut program);
    if !done.errors.is_empty() {
        bail!("{}", describe(name, source, &done.errors));
    }
    let out = Codegen::new().with_options(CodegenOptions { source_map_path: Some(path.to_path_buf()), ..CodegenOptions::default() }).build(&program);
    Ok(Stripped { js: out.code, map: out.map })
}

/// `file:line:col: message` for each diagnostic, with the offending line.
fn describe(name: &str, source: &str, errors: &[OxcDiagnostic]) -> String {
    errors
        .iter()
        .map(|e| {
            let at = e.labels.as_ref().and_then(|l| l.first()).map(|l| l.offset());
            match at {
                Some(off) => {
                    let (line, col) = line_col(source, off);
                    let text = source.lines().nth(line - 1).unwrap_or("").trim_end();
                    format!("{name}:{line}:{col}: {}\n    {text}", e.message)
                }
                None => format!("{name}: {}", e.message),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let head = &source[..offset.min(source.len())];
    let line = head.matches('\n').count() + 1;
    let col = head.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_types_and_keeps_modules() {
        let s = strip("a.ts", "import { x } from 'y';\ninterface P { a: number }\nexport const f = (n: number): string => x(`${n}`) as string;\n").unwrap();
        assert!(!s.js.contains("interface") && !s.js.contains(": number"), "{}", s.js);
        assert!(s.js.contains("import") && s.js.contains("export"), "{}", s.js);
    }

    #[test]
    fn syntax_errors_name_file_and_line() {
        let e = strip("film.ts", "const a = 1;\nconst b = ;\n").unwrap_err().to_string();
        assert!(e.contains("film.ts:2:"), "{e}");
    }
}
