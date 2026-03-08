//! Bridge between Rust crate types and frontend TypeScript types.
//!
//! Converts `ResolvedItem` + analysis results into a serializable
//! `EvaluatedItem` that matches the frontend's `ParsedItem` interface.

use poe_data::domain::TierQuality;
use poe_data::GameData;
use poe_eval::affix::{self, Modifiability};
use poe_eval::tier;
use poe_item::types::{
    InfluenceKind, ModSlot, ModSource, ModTierKind, ResolvedItem, ResolvedMod, StatusKind,
};
use serde::Serialize;

/// Serializable item for the frontend overlay.
/// Matches the TypeScript `ParsedItem` interface in `app/src/types.ts`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluatedItem {
    pub item_class: String,
    pub rarity: String,
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
}

#[derive(Debug, Serialize)]
pub struct ItemProperty {
    pub name: String,
    pub value: String,
    pub augmented: bool,
}

#[derive(Debug, Serialize)]
pub struct Requirement {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Modifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mod_name: Option<String>,
    #[serde(rename = "type")]
    pub mod_type: String,
    /// Raw tier/rank number (for badge display).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<u32>,
    /// Whether this is a "tier" (regular mod) or "rank" (bench craft).
    /// Frontend uses this for badge label: "T1" vs "R1".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier_kind: Option<String>,
    /// Quality classification from poe-data (Best/Great/Good/Mid/Low).
    /// Frontend uses this for coloring — no domain logic in the app.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
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

/// Build an `EvaluatedItem` from a resolved item + game data.
pub fn build_evaluated_item(item: &ResolvedItem, gd: &GameData) -> EvaluatedItem {
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

    EvaluatedItem {
        item_class: item.header.item_class.clone(),
        rarity: format!("{:?}", item.header.rarity),
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
    }
}

fn build_modifier(m: &ResolvedMod, tier_num: Option<u32>, quality: TierQuality) -> Modifier {
    let mod_type = match (m.header.slot, m.header.source) {
        (_, ModSource::MasterCrafted) => "crafted",
        (ModSlot::Prefix, _) => "prefix",
        (ModSlot::Suffix, _) => "suffix",
        (ModSlot::Implicit | ModSlot::SearingExarchImplicit | ModSlot::EaterOfWorldsImplicit, _) => {
            "implicit"
        }
        (ModSlot::Unique, _) => "unique",
    };

    // Tier kind: "tier" for regular mods, "rank" for bench crafts
    let tier_kind = m.header.tier.as_ref().map(|t| match t {
        ModTierKind::Tier(_) => "tier".to_string(),
        ModTierKind::Rank(_) => "rank".to_string(),
    });

    // Quality string from poe-data classification
    let quality_str = match quality {
        TierQuality::Best => Some("best"),
        TierQuality::Great => Some("great"),
        TierQuality::Good => Some("good"),
        TierQuality::Mid => Some("mid"),
        TierQuality::Low => Some("low"),
        TierQuality::Unknown => None,
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
        mod_type: mod_type.to_string(),
        tier: tier_num,
        tier_kind,
        quality: quality_str.map(String::from),
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
