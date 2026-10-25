//! TypeScript films for turbomotion. `strip` removes types (oxc), `run` executes the script on QuickJS and returns the
//! composition JSON, `cli` is `turbomotion sdk …`. No Node, no browser.

pub mod cli;
pub mod check;
mod loader;
mod measure;
mod animation;
pub mod shot;
pub mod run;
pub mod strip;

pub use run::{build, Options};

/// The SDK's TypeScript sources as `(module name, source)`, for tools that document it.
pub fn sdk_sources() -> &'static [(&'static str, &'static str)] {
    loader::SDK
}
