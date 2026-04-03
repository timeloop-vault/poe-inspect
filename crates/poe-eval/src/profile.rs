//! Scoring profiles — named rule sets that produce numeric item scores.
//!
//! A `Profile` is a list of weighted rules. Each matching rule adds its
//! weight to the total score. The final score indicates how well an item
//! fits the profile's criteria.

use serde::{Deserialize, Serialize};

use crate::rule::Rule;

/// A named scoring profile with weighted rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Profile {
    /// Human-readable profile name (e.g., "RF Juggernaut Belt").
    pub name: String,
    /// Optional description.
    #[serde(default)]
    pub description: String,
    /// Optional filter: if set, the profile only applies to items matching
    /// this rule. Items that don't match the filter get `score = 0` and
    /// `applicable = false`.
    pub filter: Option<Rule>,
    /// Weighted scoring rules. Each match adds its weight to the total.
    pub scoring: Vec<ScoringRule>,
}

/// A rule with a numeric weight. Matching adds `weight` to the item's score.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ScoringRule {
    /// Human-readable label for this scoring criterion.
    pub label: String,
    /// Points added when the rule matches.
    pub weight: f64,
    /// The rule to test.
    pub rule: Rule,
}

/// Result of scoring an item against a profile.
#[derive(Debug, Clone)]
pub struct ScoreResult {
    /// Whether the profile's filter matched (always true if no filter).
    pub applicable: bool,
    /// Total score (sum of matched rule weights). 0 if not applicable.
    pub score: f64,
    /// Which scoring rules matched, with their labels and weights.
    pub matched: Vec<MatchedRule>,
    /// Which scoring rules did NOT match.
    pub unmatched: Vec<UnmatchedRule>,
}

/// A scoring rule that matched.
#[derive(Debug, Clone)]
pub struct MatchedRule {
    pub label: String,
    pub weight: f64,
}

/// A scoring rule that did not match.
#[derive(Debug, Clone)]
pub struct UnmatchedRule {
    pub label: String,
    pub weight: f64,
}
