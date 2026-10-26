//! Colour: the types every renderer shares and the one model of colour management.
//!
//! A picture is half-float and premultiplied. What its numbers mean is a [`Profile`]: a gamut (which colours the three
//! channels span) and a transfer (how a channel relates to light). A [`Pipeline`] says which profile a composition's
//! pictures are kept in (the *working* profile, where blending, blurring and resampling happen) and which the finished
//! frame is encoded in (the *output* profile).
//!
//! There is one mechanism for conversion: Skia's colour management. A surface is tagged with its profile's colour
//! space, and everything drawn into it (a colour literal, a gradient, an image, another picture) is converted by
//! Skia on the way in, un-premultiplying first and premultiplying after, so alpha is coverage and is never encoded.
//! Flat colours are painted by an [`Ink`] so that none of them is held in 8 bits after conversion.
//! The default pipeline (sRGB, display-encoded, in and out) tags nothing: the numbers are used as they are, which is
//! what every renderer did before colour management existed.

mod encoding;
mod ink;
mod pipeline;
mod profile;

#[cfg(test)]
mod ink_tests;
#[cfg(test)]
mod tests;

pub use encoding::Encoding;
pub use ink::Ink;
pub use pipeline::Pipeline;
pub use profile::{Primaries, Profile, Transfer};

/// An sRGB colour, one byte per channel.
pub type Rgb = [u8; 3];
