#![allow(dead_code, unused_imports)]
pub use isekai_core::sprite::*;
pub mod texture;

pub mod draw;
#[cfg(target_os = "emscripten")]
pub mod embedded;
pub mod playback;
