//! TypeScript films for turbomotion. `strip` removes types (oxc), `run` executes the script on QuickJS and returns the
//! composition JSON, `cli` is `turbomotion sdk …`. No Node, no browser.

pub mod cli;
pub mod check;
mod assets;
mod fallback;
mod loader;
mod module_path;
pub mod scaffold;
mod measure;
mod animation;
pub mod shot;
pub mod run;
pub mod strip;
pub mod summary;
pub mod watch;

pub use run::{build, build_full, Built, Options};

/// The SDK's TypeScript sources as `(module name, source)`, for tools that document it.
pub fn sdk_sources() -> &'static [(&'static str, &'static str)] {
    loader::SDK
}
