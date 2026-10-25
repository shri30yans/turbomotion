//! Text: layout (`layout`), per-character animation (`animate`), named animations (`presets`) and build-time
//! measuring (`measure`).

pub mod animate;
pub mod layout;
pub mod measure;
pub mod place;
pub mod presets;

#[cfg(test)]
mod tests;
