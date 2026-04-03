//! Game data lookup tables built from parsed GGPK data.
//!
//! Holds poe-dat's raw table rows with id-based indexes for fast lookup.
//! Also owns hardcoded `PoE` domain knowledge (see `domain` module).

pub mod browser;
pub mod domain;
mod game_data;

pub use game_data::{
    GameData, LoadError, StatSuggestion, StatSuggestionKind, UniqueItemEntry, load,
};

// Re-export ReverseIndex so downstream crates don't need a direct poe-dat dependency.
pub use poe_dat::stat_desc::ReverseIndex;
