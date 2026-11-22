//! Core deletion engine. It has no Tauri dependency, so it can be tested and benchmarked on its own.

pub mod deleter;
pub mod progress;
pub mod safety;
