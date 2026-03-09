//! Bridge between Rust crate types and frontend TypeScript types.
//!
//! Converts `ResolvedItem` + analysis results into a serializable
//! `EvaluatedItem` that matches the frontend's `ParsedItem` interface.

use crate::WatchingProfileEntry;
use poe_data::domain::TierQuality;
use poe_data::GameData;
use poe_eval::affix::{self, Modifiability};
use poe_eval::profile::Profile;
use poe_eval::tier;
use poe_item::types::{
    InfluenceKind, ModSlot, ModSource, ModTierKind, Rarity, ResolvedItem, ResolvedMod, StatusKind,
};
use serde::Serialize;
use ts_rs::TS;

/// Serializable item for the frontend overlay.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EvaluatedItem {
    pub item_class: String,
    pub rarity: Rarity,
    pub name: String,
    pub base_type: String,
    pub item_level: u32,
    pub properties: Vec<ItemProperty>,
    pub requirements: Vec<Requirement>,
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

/// Score result from a watching profile, sent alongside the primary evaluation.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WatchingScoreInfo {
    pub profile_name: String,
    pub color: String,
    pub score: ScoreInfo,
}

/// Scoring result from evaluating an item against a profile.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
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

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct RuleMatch {
    pub label: String,
    pub weight: f64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ItemProperty {
    pub name: String,
    pub value: String,
    pub augmented: bool,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Requirement {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Modifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mod_name: Option<String>,
    #[serde(rename = "type")]
    pub mod_type: BridgeModType,
    /// Raw tier/rank number (for badge display).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<u32>,
    /// Whether this is a "tier" (regular mod) or "rank" (bench craft).
    /// Frontend uses this for badge label: "T1" vs "R1".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier_kind: Option<BridgeTierKind>,
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

/// Build an `EvaluatedItem` from a resolved item + game data + optional profile.
pub fn build_evaluated_item(
    item: &ResolvedItem,
    gd: &GameData,
    profile: Option<&Profile>,
    watching: &[WatchingProfileEntry],
) -> EvaluatedItem {
    let tier_summary = tier::analyze_tiers(item);
    let affix_summary = affix::analyze_affixes(item, gd);

    let corrupted = item
        .statuses
        .iter()
        .any(|s| matches!(s, StatusKind::Corrupted));
    let fractured = item
        .influences
        .iter()
        .any(|i| matches!(i, InfluenceKind::Fractured));

    // Split mods into implicits and explicits
    let mut implicits = Vec::new();
    let mut explicits = Vec::new();

    for (resolved_mod, tier_info) in item.mods.iter().zip(&tier_summary.mods) {
        let modifier = build_modifier(resolved_mod, tier_info.tier, tier_info.quality);

        match resolved_mod.header.slot {
            ModSlot::Implicit
            | ModSlot::SearingExarchImplicit
            | ModSlot::EaterOfWorldsImplicit => {
                implicits.push(modifier);
            }
            ModSlot::Prefix | ModSlot::Suffix | ModSlot::Unique => {
                explicits.push(modifier);
            }
        }
    }

    // Requirements
    let requirements = item
        .requirements
        .iter()
        .map(|r| Requirement {
            name: r.key.clone(),
            value: r.value.clone(),
        })
        .collect();

    // Influences (excluding Fractured which is a separate flag)
    let influences = item
        .influences
        .iter()
        .filter(|i| !matches!(i, InfluenceKind::Fractured))
        .map(|i| i.to_string())
        .collect();

    // Properties from generic sections (simplified — first generic section as properties)
    let properties = extract_properties(item);

    // Flavor text: last generic section if it looks like flavor text (no colon lines)
    let flavor_text = extract_flavor_text(item);

    // Evaluate watching profiles
    let watching_scores = watching
        .iter()
        .filter_map(|w| {
            let score = build_score_info(item, &w.profile, gd);
            // Only include if the filter passes and at least one rule matched
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
        properties,
        requirements,
        sockets: item.sockets.clone(),
        enchants: vec![], // TODO: enchants need section classification in poe-item
        implicits,
        explicits,
        influences,
        corrupted: if corrupted { Some(true) } else { None },
        fractured: if fractured { Some(true) } else { None },
        flavor_text,
        open_prefixes: affix_summary.prefixes.open.unwrap_or(0),
        open_suffixes: affix_summary.suffixes.open.unwrap_or(0),
        max_prefixes: affix_summary.prefixes.max.unwrap_or(0),
        max_suffixes: affix_summary.suffixes.max.unwrap_or(0),
        modifiable: affix_summary.modifiable == Modifiability::Yes,
        score: profile.map(|p| build_score_info(item, p, gd)),
        watching_scores,
    }
}

fn build_score_info(item: &ResolvedItem, profile: &Profile, gd: &GameData) -> ScoreInfo {
    let result = poe_eval::score(item, profile, gd);
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
    let mod_type = match (m.header.slot, m.header.source) {
        (_, ModSource::MasterCrafted) => BridgeModType::Crafted,
        (ModSlot::Prefix, _) => BridgeModType::Prefix,
        (ModSlot::Suffix, _) => BridgeModType::Suffix,
        (ModSlot::Implicit | ModSlot::SearingExarchImplicit | ModSlot::EaterOfWorldsImplicit, _) => {
            BridgeModType::Implicit
        }
        (ModSlot::Unique, _) => BridgeModType::Unique,
    };

    // Tier kind: tier for regular mods, rank for bench crafts
    let tier_kind = m.header.tier.as_ref().map(|t| match t {
        ModTierKind::Tier(_) => BridgeTierKind::Tier,
        ModTierKind::Rank(_) => BridgeTierKind::Rank,
    });

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

    let is_fractured = m
        .stat_lines
        .iter()
        .any(|sl| sl.raw_text.ends_with("(fractured)"));

    Modifier {
        mod_name: m.header.name.clone(),
        mod_type,
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
        fractured: if is_fractured { Some(true) } else { None },
    }
}

/// Extract property lines from generic sections.
/// Properties are lines containing `: ` (e.g., "Armour: 890 (augmented)").
fn extract_properties(item: &ResolvedItem) -> Vec<ItemProperty> {
    let mut props = Vec::new();
    for section in &item.properties {
        for line in section {
            if let Some((name, rest)) = line.split_once(": ") {
                let augmented = rest.contains("(augmented)");
                let value = rest
                    .replace(" (augmented)", "")
                    .replace("(augmented)", "")
                    .trim()
                    .to_string();
                props.push(ItemProperty {
                    name: name.to_string(),
                    value,
                    augmented,
                });
            }
        }
    }
    props
}

// ── Bridge-specific enums ────────────────────────────────────────────────────
// These flatten complex source types (ModSlot + ModSource, ModTierKind) into
// simple enums for the frontend. Rarity and TierQuality are generated from
// their source crates (poe-item, poe-data) via the `ts` feature.

/// Modifier source slot (bridge maps ModSlot + ModSource → this flat set).
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, rename = "ModType")]
pub enum BridgeModType {
    Prefix,
    Suffix,
    Implicit,
    Enchant,
    Unique,
    Crafted,
    Fractured,
}

/// Whether a mod number is a tier or rank (bridge splits ModTierKind's data).
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, rename = "TierKind")]
pub enum BridgeTierKind {
    Tier,
    Rank,
}

/// Extract flavor text — last generic section that has no colon-lines.
fn extract_flavor_text(item: &ResolvedItem) -> Option<String> {
    if let Some(last) = item.properties.last() {
        let has_colon = last.iter().any(|l| l.contains(": "));
        if !has_colon && !last.is_empty() {
            return Some(last.join("\n"));
        }
    }
    None
}
