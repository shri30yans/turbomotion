//! Authoring tools that rewrite or produce composition JSON without touching the renderer: they take a file and emit a
//! file, so an agent can chain them (`svg` → `reframe` → `check`) and every result is still a plain `comp.json`.

pub mod reframe;
pub mod svg;
