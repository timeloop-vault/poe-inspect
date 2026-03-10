//! Display-ready item evaluation result.
//!
//! Combines parsing (poe-item) + tier analysis + affix analysis + profile
//! scoring into a single serializable struct for the frontend overlay.
//! This is the **only** type the app needs to render an evaluated item.

use poe_data::domain::TierQuality;
use poe_data::GameData;
use poe_item::types::{
    InfluenceKind, ModDisplayType, ModSource, Rarity, ResolvedItem, ResolvedMod, TierDisplayKind,
};
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

/// Complete evaluation result for the frontend overlay.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct EvaluatedItem {
    pub item_class: String,
    pub rarity: Rarity,
    pub name: String,
    pub base_type: String,
    pub item_level: u32,
    pub properties: Vec<poe_item::types::ItemProperty>,
    pub requirements: Vec<poe_item::types::Requirement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sockets: Option<String>,
    pub enchants: Vec<Modifier>,
    pub implicits: Vec<Modifier>,
    pub explicits: Vec<Modifier>,
    pub influences: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corrupted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fractured: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flavor_text: Option<String>,
    pub open_prefixes: u32,
    pub open_suffixes: u32,
    pub max_prefixes: u32,
    pub max_suffixes: u32,
    pub modifiable: bool,
    /// Profile score (None if no profile active or not applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<ScoreInfo>,
    /// Scores from watching profiles.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub watching_scores: Vec<WatchingScoreInfo>,
}

/// Score result from a watching profile.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct WatchingScoreInfo {
    pub profile_name: String,
    pub color: String,
    pub score: ScoreInfo,
}

/// Scoring result from evaluating an item against a profile.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScoreInfo {
    /// Total score (sum of matched rule weights).
    pub total: f64,
    /// Maximum possible score (sum of all rule weights).
    pub max_possible: f64,
    /// Percentage (total / max_possible * 100), clamped to 0-100.
    pub percent: f64,
    /// Whether the profile filter matched this item.
    pub applicable: bool,
    /// Rules that matched (label + weight).
    pub matched: Vec<RuleMatch>,
    /// Rules that didn't match (label + weight).
    pub unmatched: Vec<RuleMatch>,
}

/// A matched or unmatched scoring rule.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RuleMatch {
    pub label: String,
    pub weight: f64,
}

/// Display-ready modifier for the frontend.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Modifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mod_name: Option<String>,
    #[serde(rename = "type")]
    pub mod_type: ModDisplayType,
    /// Raw tier/rank number (for badge display).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<u32>,
    /// Whether this is a "tier" (regular mod) or "rank" (bench craft).
    /// Frontend uses this for badge label: "T1" vs "R1".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier_kind: Option<TierDisplayKind>,
    /// Quality classification from poe-data (Best/Great/Good/Mid/Low).
    /// Frontend uses this for coloring — no domain logic in the app.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<TierQuality>,
    pub tags: Vec<String>,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crafted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fractured: Option<bool>,
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Build a complete evaluation result from a resolved item.
///
/// This is the single entry point the app calls. It combines:
/// - Tier analysis (mod quality classification)
/// - Affix analysis (open slots, modifiability)
/// - Profile scoring (primary + watching profiles)
///
/// The result is directly serializable to JSON for the frontend overlay.
#[must_use]
pub fn evaluate_item(
    item: &ResolvedItem,
    gd: &GameData,
    profile: Option<&Profile>,
    watching: &[WatchingProfileInput],
) -> EvaluatedItem {
    let tier_summary = tier::analyze_tiers(item, gd);
    let affix_summary = affix::analyze_affixes(item, gd);

    // Build display mods with tier info
    let all_mods: Vec<_> = item.all_mods().collect();
    let mut implicits = Vec::new();
    let mut explicits = Vec::new();

    for (resolved_mod, tier_info) in all_mods.iter().zip(&tier_summary.mods) {
        let modifier = build_modifier(resolved_mod, tier_info.tier, tier_info.quality);
        match resolved_mod.display_type() {
            poe_item::types::ModDisplayType::Implicit => implicits.push(modifier),
            _ => explicits.push(modifier),
        }
    }

    // Influences (excluding Fractured which is a separate flag)
    let influences = item
        .influences
        .iter()
        .filter(|i| !matches!(i, InfluenceKind::Fractured))
        .map(|i| i.to_string())
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

    EvaluatedItem {
        item_class: item.header.item_class.clone(),
        rarity: item.header.rarity,
        name: item
            .header
            .name
            .clone()
            .unwrap_or_else(|| item.header.base_type.clone()),
        base_type: item.header.base_type.clone(),
        item_level: item.item_level.unwrap_or(0),
        properties: item.properties.clone(),
        requirements: item.requirements.clone(),
        sockets: item.sockets.clone(),
        enchants: vec![],
        implicits,
        explicits,
        influences,
        corrupted: if item.is_corrupted { Some(true) } else { None },
        fractured: if item.is_fractured { Some(true) } else { None },
        flavor_text: item.flavor_text.clone(),
        open_prefixes: affix_summary.prefixes.open.unwrap_or(0),
        open_suffixes: affix_summary.suffixes.open.unwrap_or(0),
        max_prefixes: affix_summary.prefixes.max.unwrap_or(0),
        max_suffixes: affix_summary.suffixes.max.unwrap_or(0),
        modifiable: affix_summary.modifiable == Modifiability::Yes,
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

fn build_modifier(m: &ResolvedMod, tier_num: Option<u32>, quality: TierQuality) -> Modifier {
    // Tier kind from poe-item's method
    let tier_kind = m.header.tier.as_ref().map(|t| t.display_kind());

    // Quality from poe-data classification (None for Unknown)
    let quality_val = match quality {
        TierQuality::Unknown => None,
        q => Some(q),
    };

    // Combine stat lines into display text
    let text = m
        .stat_lines
        .iter()
        .filter(|sl| !sl.is_reminder)
        .map(|sl| sl.display_text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // Use first stat line's value range for the roll bar
    let first_value = m
        .stat_lines
        .iter()
        .find(|sl| !sl.is_reminder && !sl.values.is_empty())
        .and_then(|sl| sl.values.first());

    let (value, min, max) = match first_value {
        Some(vr) => (
            Some(vr.current as f64),
            Some(vr.min as f64),
            Some(vr.max as f64),
        ),
        None => (None, None, None),
    };

    Modifier {
        mod_name: m.header.name.clone(),
        mod_type: m.display_type(),
        tier: tier_num,
        tier_kind,
        quality: quality_val,
        tags: m.header.tags.clone(),
        text,
        value,
        min,
        max,
        crafted: if m.header.source == ModSource::MasterCrafted {
            Some(true)
        } else {
            None
        },
        fractured: if m.is_fractured { Some(true) } else { None },
    }
}
