//! Rules — combinators that compose predicates into complex conditions.
//!
//! A `Rule` is a tree of predicates joined by AND/OR/NOT logic.
//! Rules are serializable so they can be saved as JSON/TOML profiles.

use serde::{Deserialize, Serialize};

use crate::predicate::Predicate;

/// A rule is either an atomic predicate or a logical combination.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "rule_type")]
pub enum Rule {
    /// A single atomic predicate.
    Pred(Predicate),

    /// All sub-rules must match.
    All { rules: Vec<Rule> },

    /// At least one sub-rule must match.
    Any { rules: Vec<Rule> },

    /// The sub-rule must NOT match.
    Not { rule: Box<Rule> },
}

impl Rule {
    /// Shorthand: wrap a predicate as a rule.
    #[must_use]
    pub fn pred(p: Predicate) -> Self {
        Self::Pred(p)
    }

    /// Shorthand: all predicates must match.
    #[must_use]
    pub fn all(rules: Vec<Rule>) -> Self {
        Self::All { rules }
    }

    /// Shorthand: any predicate must match.
    #[must_use]
    pub fn any(rules: Vec<Rule>) -> Self {
        Self::Any { rules }
    }

    /// Shorthand: negate a rule.
    #[must_use]
    pub fn negate(rule: Rule) -> Self {
        Self::Not {
            rule: Box::new(rule),
        }
    }
}
