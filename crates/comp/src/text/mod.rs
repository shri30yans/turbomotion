//! Text: layout (`layout`), per-character animation (`animate`), named animations (`presets`), where the visible
//! characters are (`bounds`, and `boxfit` for the rectangles fitted to them) and build-time measuring (`measure`).

pub mod animate;
pub mod bounds;
pub mod boxfit;
pub mod layout;
pub mod measure;
pub mod missing_font;
pub mod place;
pub mod presets;

#[cfg(test)]
mod tests;
