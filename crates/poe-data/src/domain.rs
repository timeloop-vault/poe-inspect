//! Hardcoded `PoE` domain knowledge that is NOT extracted from the GGPK.
//!
//! **Why this module exists**: `poe-data` is the single source of truth for
//! all game knowledge. Some knowledge isn't available in the GGPK and must
//! be maintained here. This module makes that explicit — every item here
//! documents WHY it's hardcoded rather than extracted.
//!
//! **Rule**: If it's `PoE`/GGG game logic, it lives in `poe-data` — either
//! extracted from GGPK tables or hardcoded here. Higher layers (poe-item,
//! poe-eval, app) have zero `PoE` domain knowledge.
//!
//! See `docs/poe-data-gap-filling.md` for the recurring process.

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

/// Classify a mod tier number into a quality level.
///
/// This is hardcoded `PoE` domain knowledge. See module docs for rationale.
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
