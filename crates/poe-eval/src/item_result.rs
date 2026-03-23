//! Item evaluation result — pure evaluation data, no display reshaping.
//!
//! Combines tier analysis + affix analysis + profile scoring into a
//! serializable struct. The app combines this with `ResolvedItem` (poe-item)
//! into a frontend payload.

use poe_data::GameData;
use poe_data::domain::TierQuality;
use poe_item::types::ResolvedItem;
use serde::Serialize;

use crate::affix::{self, Modifiability};
use crate::profile::Profile;
use crate::tier;

/// Input for a watching profile — name, color, and the profile itself.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WatchingProfileInput {
    pub name: String,
    pub color: String,
    pub profile: Profile,
}

/// Evaluation result for a parsed item.
///
/// Contains only evaluation data: tier quality per mod, affix analysis, and
/// scoring results. Display data comes from `ResolvedItem` (poe-item).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ItemEvaluation {
    /// Per-mod tier analysis, ordered to match `ResolvedItem::all_mods()`.
    pub mod_tiers: Vec<ModTierResult>,
    /// Affix slot analysis.
    pub affix_summary: AffixInfo,
    /// Primary profile score.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<ScoreInfo>,
    /// Scores from watching profiles.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub watching_scores: Vec<WatchingScoreInfo>,
}

/// Per-mod tier evaluation result (aligned with `all_mods()` ordering).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ModTierResult {
    /// Tier/rank number (from Ctrl+Alt+C header).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<u32>,
    /// Total number of tiers for this mod (from GGPK Mods table).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tiers: Option<u32>,
    /// Whether this is a "tier" (regular mod) or "rank" (bench craft).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier_kind: Option<poe_item::types::TierDisplayKind>,
    /// Quality classification from poe-data (Best/Great/Good/Mid/Low).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<TierQuality>,
}

/// Serializable affix summary for the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct AffixInfo {
    pub open_prefixes: u32,
    pub open_suffixes: u32,
    pub max_prefixes: u32,
    pub max_suffixes: u32,
    pub modifiable: bool,
}

/// Score result from a watching profile.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WatchingScoreInfo {
    pub profile_name: String,
    pub color: String,
    pub score: ScoreInfo,
}

/// Scoring result from evaluating an item against a profile.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScoreInfo {
    /// Total score (sum of matched rule weights).
    pub total: f64,
    /// Maximum possible score (sum of all rule weights).
    pub max_possible: f64,
    /// Percentage (total / `max_possible` * 100), clamped to 0-100.
    pub percent: f64,
    /// Whether the profile filter matched this item.
    pub applicable: bool,
    /// Rules that matched (label + weight).
    pub matched: Vec<RuleMatch>,
    /// Rules that didn't match (label + weight).
    pub unmatched: Vec<RuleMatch>,
}

/// A matched or unmatched scoring rule.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RuleMatch {
    pub label: String,
    pub weight: f64,
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Evaluate a resolved item: tier analysis, affix analysis, and scoring.
///
/// Returns evaluation-only data. The app combines this with `ResolvedItem`
/// for the frontend payload.
#[must_use]
pub fn evaluate_item(
    item: &ResolvedItem,
    gd: &GameData,
    profile: Option<&Profile>,
    watching: &[WatchingProfileInput],
) -> ItemEvaluation {
    let tier_summary = tier::analyze_tiers(item, gd);
    let affix_summary = affix::analyze_affixes(item, gd);

    // Build per-mod tier results aligned with all_mods() order
    let mod_tiers: Vec<ModTierResult> = item
        .all_mods()
        .zip(&tier_summary.mods)
        .map(|(m, tier_info)| {
            let tier_kind = m
                .header
                .tier
                .as_ref()
                .map(poe_item::types::ModTierKind::display_kind);
            let quality = match tier_info.quality {
                TierQuality::Unknown => None,
                q => Some(q),
            };
            ModTierResult {
                tier: tier_info.tier,
                total_tiers: tier_info.total_tiers,
                tier_kind,
                quality,
            }
        })
        .collect();

    // Evaluate watching profiles
    let watching_scores = watching
        .iter()
        .filter_map(|w| {
            let score = build_score_info(item, &w.profile, gd);
            if score.applicable && score.total > 0.0 {
                Some(WatchingScoreInfo {
                    profile_name: w.name.clone(),
                    color: w.color.clone(),
                    score,
                })
            } else {
                None
            }
        })
        .collect();

    ItemEvaluation {
        mod_tiers,
        affix_summary: AffixInfo {
            open_prefixes: affix_summary.prefixes.open.unwrap_or(0),
            open_suffixes: affix_summary.suffixes.open.unwrap_or(0),
            max_prefixes: affix_summary.prefixes.max.unwrap_or(0),
            max_suffixes: affix_summary.suffixes.max.unwrap_or(0),
            modifiable: affix_summary.modifiable == Modifiability::Yes,
        },
        score: profile.map(|p| build_score_info(item, p, gd)),
        watching_scores,
    }
}

// ── Internal helpers ────────────────────────────────────────────────────────

fn build_score_info(item: &ResolvedItem, profile: &Profile, gd: &GameData) -> ScoreInfo {
    let result = crate::score(item, profile, gd);
    let max_possible: f64 = profile.scoring.iter().map(|s| s.weight).sum();
    let percent = if max_possible > 0.0 {
        (result.score / max_possible * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    ScoreInfo {
        total: result.score,
        max_possible,
        percent,
        applicable: result.applicable,
        matched: result
            .matched
            .into_iter()
            .map(|m| RuleMatch {
                label: m.label,
                weight: m.weight,
            })
            .collect(),
        unmatched: result
            .unmatched
            .into_iter()
            .map(|m| RuleMatch {
                label: m.label,
                weight: m.weight,
            })
            .collect(),
    }
}
