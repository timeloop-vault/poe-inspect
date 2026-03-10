//! Open affix analysis for resolved items.
//!
//! Determines how many prefix/suffix slots are used vs available,
//! whether the item can be modified, and summarizes the affix layout.
//! All game data (max affixes per rarity) comes from `poe-data`.

use poe_data::GameData;
use poe_item::types::{ModSlot, ModSource, ResolvedItem, StatusKind};

/// Full affix analysis for an item.
#[derive(Debug, Clone)]
pub struct AffixSummary {
    pub prefixes: SlotSummary,
    pub suffixes: SlotSummary,
    /// Whether the item can be modified at all.
    pub modifiable: Modifiability,
}

/// Summary of one affix slot type (prefix or suffix).
#[derive(Debug, Clone)]
pub struct SlotSummary {
    /// Number of mods currently filling this slot.
    pub used: u32,
    /// Maximum allowed for this item's rarity (None if unknown/not applicable).
    pub max: Option<u32>,
    /// Number of open slots (max - used), or None if max is unknown.
    pub open: Option<u32>,
    /// Whether one of the used slots is a bench craft (removable).
    pub has_crafted: bool,
}

/// Whether and why an item can or cannot be modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifiability {
    /// Item can be modified normally.
    Yes,
    /// Item is corrupted — cannot be modified.
    Corrupted,
    /// Item is mirrored — cannot be modified.
    Mirrored,
    /// Item is split — cannot be modified further.
    Split,
    /// Rarity doesn't support affixes (Normal, Gem, Currency, etc.)
    NotApplicable,
}

/// Analyze the affix layout of an item.
pub fn analyze_affixes(item: &ResolvedItem, gd: &GameData) -> AffixSummary {
    let modifiable = check_modifiability(item);

    let (prefix_used, prefix_crafted) = count_slot(item, ModSlot::Prefix);
    let (suffix_used, suffix_crafted) = count_slot(item, ModSlot::Suffix);

    let (prefix_max, suffix_max) = resolve_max_affixes(item, gd);

    AffixSummary {
        prefixes: SlotSummary {
            used: prefix_used,
            max: prefix_max,
            open: prefix_max.map(|m| m.saturating_sub(prefix_used)),
            has_crafted: prefix_crafted,
        },
        suffixes: SlotSummary {
            used: suffix_used,
            max: suffix_max,
            open: suffix_max.map(|m| m.saturating_sub(suffix_used)),
            has_crafted: suffix_crafted,
        },
        modifiable,
    }
}

/// Check if the item can be modified.
fn check_modifiability(item: &ResolvedItem) -> Modifiability {
    // Check for blocking statuses
    for s in &item.statuses {
        match s {
            StatusKind::Corrupted => return Modifiability::Corrupted,
            StatusKind::Mirrored => return Modifiability::Mirrored,
            StatusKind::Split => return Modifiability::Split,
            _ => {}
        }
    }

    // Check rarity — only Magic/Rare can have explicit affixes
    let rarity_str = format!("{:?}", item.header.rarity);
    if poe_data::domain::rarity_to_ggpk_id(&rarity_str).is_none() {
        return Modifiability::NotApplicable;
    }

    Modifiability::Yes
}

/// Count mods in a slot, returning (total, has_crafted).
fn count_slot(item: &ResolvedItem, slot: ModSlot) -> (u32, bool) {
    let mut count = 0u32;
    let mut has_crafted = false;
    for m in item.all_mods() {
        if m.header.slot == slot {
            count += 1;
            if m.header.source == ModSource::MasterCrafted {
                has_crafted = true;
            }
        }
    }
    (count, has_crafted)
}

/// Look up max prefix/suffix counts for this item's rarity.
fn resolve_max_affixes(item: &ResolvedItem, gd: &GameData) -> (Option<u32>, Option<u32>) {
    let rarity_str = format!("{:?}", item.header.rarity);
    let Some(rarity_id) = poe_data::domain::rarity_to_ggpk_id(&rarity_str) else {
        return (None, None);
    };

    let prefix_max = gd
        .max_prefixes(rarity_id)
        .and_then(|v| u32::try_from(v).ok());
    let suffix_max = gd
        .max_suffixes(rarity_id)
        .and_then(|v| u32::try_from(v).ok());

    (prefix_max, suffix_max)
}
