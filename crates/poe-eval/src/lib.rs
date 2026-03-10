//! Evaluate parsed items against user-defined filter rules.
//!
//! This crate contains zero `PoE` domain knowledge — all game-specific
//! data comes from `poe-data`'s `GameData`. Evaluation is pure logic:
//! predicates, rules, and matching.

pub mod affix;
pub mod evaluate;
pub mod item_result;
pub mod predicate;
pub mod profile;
pub mod rule;
pub mod schema;
pub mod tier;

pub use affix::{analyze_affixes, AffixSummary, Modifiability};
pub use evaluate::{evaluate, score};
pub use item_result::{evaluate_item, ItemEvaluation, WatchingProfileInput};
pub use predicate::Predicate;
pub use profile::{Profile, ScoreResult};
pub use rule::Rule;
pub use schema::{predicate_schema, PredicateSchema};
pub use poe_data::domain::TierQuality;
pub use tier::{analyze_tiers, ItemTierSummary};
