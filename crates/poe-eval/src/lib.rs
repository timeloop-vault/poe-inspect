//! Evaluate parsed items against user-defined filter rules.
//!
//! This crate contains zero `PoE` domain knowledge — all game-specific
//! data comes from `poe-data`'s `GameData`. Evaluation is pure logic:
//! predicates, rules, and matching.

pub mod evaluate;
pub mod predicate;
pub mod rule;

pub use evaluate::evaluate;
pub use predicate::Predicate;
pub use rule::Rule;
