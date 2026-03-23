//! Tier quality analysis for item modifiers.
//!
//! Maps mod tiers (from Ctrl+Alt+C `{ }` headers) to quality levels
//! for visual feedback in the overlay. Uses `poe-data`'s domain knowledge
//! for tier classification — this module is pure evaluation logic.

use poe_data::GameData;
use poe_data::domain::{TierQuality, classify_rank, classify_tier, classify_tier_relative};
use poe_item::types::{ModSlot, ModSource, ModTierKind, ResolvedItem, ResolvedMod};

/// Tier analysis for a single mod.
#[derive(Debug, Clone)]
pub struct ModTierInfo {
    /// Mod name from the `{ }` header (if available).
    pub name: Option<String>,
    /// Mod slot (Prefix, Suffix, Implicit, etc.).
    pub slot: ModSlot,
    /// Raw tier number (if available).
    pub tier: Option<u32>,
    /// Total number of tiers for this mod (from GGPK Mods table).
    pub total_tiers: Option<u32>,
    /// Computed quality level.
    pub quality: TierQuality,
}

/// Tier analysis for an entire item.
#[derive(Debug, Clone)]
pub struct ItemTierSummary {
    /// Per-mod tier info, in the same order as `ResolvedItem.mods`.
    pub mods: Vec<ModTierInfo>,
    /// Lowest (worst) quality among all explicit mods with tiers.
    pub worst_explicit: TierQuality,
    /// Highest (best) quality among all explicit mods with tiers.
    pub best_explicit: TierQuality,
    /// Count of explicit mods at each quality level.
    pub quality_counts: QualityCounts,
}

/// Count of explicit mods at each tier quality level.
#[derive(Debug, Clone, Default)]
pub struct QualityCounts {
    pub best: u32,
    pub great: u32,
    pub good: u32,
    pub mid: u32,
    pub low: u32,
}

/// Analyze a single mod's tier, using per-mod tier counts when available.
fn analyze_mod(m: &ResolvedMod, gd: &GameData) -> ModTierInfo {
    let tier = match &m.header.tier {
        Some(ModTierKind::Tier(n) | ModTierKind::Rank(n)) => Some(*n),
        None => None,
    };

    let total_tiers = m
        .header
        .name
        .as_deref()
        .and_then(|name| gd.tier_count_for_mod(name));

    let quality = match &m.header.tier {
        Some(ModTierKind::Tier(n)) => match total_tiers {
            Some(total) => classify_tier_relative(*n, total),
            None => classify_tier(*n),
        },
        Some(ModTierKind::Rank(n)) => classify_rank(*n),
        None => TierQuality::Unknown,
    };

    ModTierInfo {
        name: m.header.name.clone(),
        slot: m.header.slot,
        tier,
        total_tiers,
        quality,
    }
}

/// Analyze all mods on an item.
pub fn analyze_tiers(item: &ResolvedItem, gd: &GameData) -> ItemTierSummary {
    let all_mods: Vec<&ResolvedMod> = item.all_mods().collect();
    let mods: Vec<ModTierInfo> = all_mods.iter().map(|m| analyze_mod(m, gd)).collect();

    let mut worst_explicit = TierQuality::Best;
    let mut best_explicit = TierQuality::Low;
    let mut counts = QualityCounts::default();
    let mut has_explicit = false;

    for (info, resolved_mod) in mods.iter().zip(&all_mods) {
        // Only count natural prefix/suffix for explicit tier summary
        // (skip implicits, crafted mods, influence mods, uniques)
        if !matches!(info.slot, ModSlot::Prefix | ModSlot::Suffix) {
            continue;
        }
        if resolved_mod.header.source == ModSource::MasterCrafted {
            continue;
        }
        if info.quality == TierQuality::Unknown {
            continue;
        }

        has_explicit = true;

        // Ord: Best < Great < Good < Mid < Low
        // "worst" = highest in this ordering (towards Low)
        // "best" = lowest in this ordering (towards Best)
        if info.quality > worst_explicit {
            worst_explicit = info.quality;
        }
        if info.quality < best_explicit {
            best_explicit = info.quality;
        }

        match info.quality {
            TierQuality::Best => counts.best += 1,
            TierQuality::Great => counts.great += 1,
            TierQuality::Good => counts.good += 1,
            TierQuality::Mid => counts.mid += 1,
            TierQuality::Low => counts.low += 1,
            TierQuality::Unknown => {}
        }
    }

    if !has_explicit {
        worst_explicit = TierQuality::Unknown;
        best_explicit = TierQuality::Unknown;
    }

    ItemTierSummary {
        mods,
        worst_explicit,
        best_explicit,
        quality_counts: counts,
    }
}
