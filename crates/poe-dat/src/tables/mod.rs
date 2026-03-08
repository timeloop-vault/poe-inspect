//! Typed extraction of specific dat tables from GGPK data.
//!
//! Each table is read from raw `DatFile` bytes into typed Rust structs.
//! Only the fields we need for item evaluation are extracted — unknown
//! or unused fields are skipped by offset.
//!
//! The caller is responsible for providing the raw bytes (via poe-bundle
//! or from cached files). This module is pure parsing, no I/O.

mod extract;
mod types;

pub use extract::*;
pub use types::*;
