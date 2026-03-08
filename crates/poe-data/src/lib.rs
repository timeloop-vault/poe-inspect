//! Game data lookup tables built from parsed GGPK data.
//!
//! Holds poe-dat's raw table rows with id-based indexes for fast lookup.
//! Also owns hardcoded `PoE` domain knowledge (see `domain` module).

pub mod domain;
mod game_data;

pub use game_data::{load, GameData, LoadError};
