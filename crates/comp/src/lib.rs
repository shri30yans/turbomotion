//! After Effects-style compositions: schema → plan → eval → render (see docs/ARCHITECTURE.md).

pub mod anim;
pub mod api;
pub mod audio;
pub mod check;
pub mod eval;
pub mod fxlib;
pub mod media;
pub mod mesh;
pub mod pack;
pub mod plan;
pub mod render;
pub mod schema;
pub mod shape;
pub mod text;
pub mod tools;

pub mod catalogue;
pub mod cli;

pub mod inspect;
pub mod lookup;
