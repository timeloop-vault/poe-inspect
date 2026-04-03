// ── Tier quality classification ─────────────────────────────────────────────
//
// WHY HARDCODED: The GGPK doesn't have a "tier quality" concept. Mod tiers
// (T1, T2, etc.) are an implicit ordering derived from `Mods.datc64` — mods
// in the same family sorted by level requirement, with T1 being highest level.
// The Ctrl+Alt+C `{ Tier: N }` header exposes the tier number, but there's
// no GGPK table mapping tier numbers to quality labels.
//
// The convention "lower tier = better" is universal in PoE and has never
// changed. The quality bucketing (T1=Best, T3-4=Good, T7+=Low) is our
// interpretation for display purposes.

/// Quality level for a mod tier. Lower tier number = better quality.
///
/// Ordered so that `Best < Great < Good < Mid < Low < Unknown`,
/// which means "better" sorts first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum TierQuality {
    /// Tier 1 — best possible.
    Best,
    /// Tier 2 — near-best.
    Great,
    /// Tier 3-4 — good.
    Good,
    /// Tier 5-6 — mediocre.
    Mid,
    /// Tier 7+ — low.
    Low,
    /// No tier info available.
    Unknown,
}

/// Classify a mod tier number into a quality level (absolute fallback).
///
/// For regular mods: lower tier = better (T1 = best, T7+ = low).
/// Prefer `classify_tier_relative()` when the total tier count is known.
#[must_use]
pub fn classify_tier(tier: u32) -> TierQuality {
    match tier {
        1 => TierQuality::Best,
        2 => TierQuality::Great,
        3 | 4 => TierQuality::Good,
        5 | 6 => TierQuality::Mid,
        _ => TierQuality::Low,
    }
}

/// Classify a mod tier relative to the total number of tiers for that mod.
///
/// A 3-tier mod's T3 is "Low" (worst for that mod), even though the absolute
/// `classify_tier(3)` would call it "Good". This gives accurate coloring
/// by considering each mod's actual tier range from the GGPK Mods table.
#[must_use]
pub fn classify_tier_relative(tier: u32, total_tiers: u32) -> TierQuality {
    if tier == 0 || total_tiers == 0 || tier > total_tiers {
        return TierQuality::Unknown;
    }
    if total_tiers == 1 {
        return TierQuality::Best;
    }
    if total_tiers == 2 {
        return if tier == 1 {
            TierQuality::Best
        } else {
            TierQuality::Low
        };
    }
    // 3+ tiers: position as fraction (0.0 = best, 1.0 = worst)
    let position = f64::from(tier - 1) / f64::from(total_tiers - 1);
    if position < 0.01 {
        TierQuality::Best // T1
    } else if position < 0.25 {
        TierQuality::Great // top 25%
    } else if position < 0.50 {
        TierQuality::Good // 25-50%
    } else if position < 0.75 {
        TierQuality::Mid // 50-75%
    } else {
        TierQuality::Low // bottom 25%
    }
}

// ── Crafted mod rank classification ─────────────────────────────────────────
//
// WHY HARDCODED: Bench crafts use "Rank" instead of "Tier" and the ordering
// is REVERSED: Rank 1 = lowest/weakest bench craft, higher ranks = better
// values and higher ilvl requirements. There's no GGPK table that tells us
// the max rank per craft family, so we can't compute a quality level from
// the rank alone — we'd need `CraftingBenchOptions.datc64` for that.
//
// For now, we use a simple heuristic: bench crafts are typically 3-4 ranks,
// so we classify accordingly. This will be replaced by proper lookup once
// we extract `CraftingBenchOptions` (see docs/crafting-tiers.md).

/// Classify a crafted mod rank into a quality level.
///
/// For bench crafts: higher rank = better (opposite of tiers).
/// Rank 1 = weakest bench craft, Rank 3+ = best available.
#[must_use]
pub fn classify_rank(rank: u32) -> TierQuality {
    match rank {
        4.. => TierQuality::Best,
        3 => TierQuality::Great,
        2 => TierQuality::Good,
        1 => TierQuality::Mid,
        0 => TierQuality::Unknown,
    }
}

// ── Rarity ID mapping ───────────────────────────────────────────────────────
//
// WHY HARDCODED: The `Rarity.datc64` table has an `Id` field with string
// values like "Normal", "Magic", "Rare", "Unique". However, `poe-item`'s
// `Rarity` enum is parsed from Ctrl+Alt+C text (which also uses these exact
// strings). The mapping between poe-item's enum and the GGPK rarity ID is
// trivial and stable, but it IS domain knowledge — higher layers shouldn't
// need to know the GGPK's rarity ID strings.
//
// NOTE: If the GGPK rarity IDs ever change (extremely unlikely), update here.

/// Map a rarity string (from Ctrl+Alt+C "Rarity:" line) to the GGPK
/// `Rarity.datc64` table ID for lookups.
///
/// Returns `None` for rarities that don't have affix limits (Gem, Currency, etc.).
#[must_use]
pub fn rarity_to_ggpk_id(rarity: &str) -> Option<&'static str> {
    match rarity {
        "Normal" => Some("Normal"),
        "Magic" => Some("Magic"),
        "Rare" => Some("Rare"),
        "Unique" => Some("Unique"),
        _ => None,
    }
}
