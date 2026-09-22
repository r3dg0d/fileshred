//! Filesystem inspection for secure-delete suitability.

pub mod classify;
pub mod detect;

pub use classify::{DeleteAdvice, StorageClass};
pub use detect::{inspect_path, PathInspection};
