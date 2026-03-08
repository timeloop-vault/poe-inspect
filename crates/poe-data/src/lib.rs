//! Game data lookup tables built from parsed GGPK data.
//!
//! Holds poe-dat's raw table rows with id-based indexes for fast lookup.
//! No new domain types — poe-item will drive what we reshape.

mod game_data;

pub use game_data::{load, GameData, LoadError};
