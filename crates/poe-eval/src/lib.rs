//! Evaluate parsed items against user-defined filter rules.
//!
//! This crate contains zero `PoE` domain knowledge — all game-specific
//! data comes from `poe-data`'s `GameData`. Evaluation is pure logic:
//! predicates, rules, and matching.

pub mod evaluate;
pub mod predicate;
pub mod profile;
pub mod rule;

pub use evaluate::{evaluate, score};
pub use predicate::Predicate;
pub use profile::{Profile, ScoreResult};
pub use rule::Rule;
