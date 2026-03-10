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

pub use affix::{AffixSummary, Modifiability, analyze_affixes};
pub use evaluate::{evaluate, score};
pub use item_result::{ItemEvaluation, WatchingProfileInput, evaluate_item};
pub use poe_data::domain::TierQuality;
pub use predicate::Predicate;
pub use profile::{Profile, ScoreResult};
pub use rule::Rule;
pub use schema::{PredicateSchema, predicate_schema};
pub use tier::{ItemTierSummary, analyze_tiers};
