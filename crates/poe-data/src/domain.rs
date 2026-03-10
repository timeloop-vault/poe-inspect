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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
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
    let position = (tier - 1) as f64 / (total_tiers - 1) as f64;
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

// ── Trade API stat suffixes ─────────────────────────────────────────────────
//
// WHY HARDCODED: The trade API at pathofexile.com appends classification
// suffixes to certain stat descriptions to distinguish local mods from global
// ones. These suffixes don't exist in the GGPK's stat_descriptions.txt format
// strings — they're added by GGG's trade system. When matching trade API stat
// text against our ReverseIndex templates, these suffixes must be stripped.
//
// Known suffixes (verified against 3.28 Mirage trade API):
// - "(Local)" — weapon/armour local mods (e.g., "+# to Armour (Local)")
// - "(Shields)" — shield-specific mods (e.g., "+#% Chance to Block (Shields)")

/// Suffixes that GGG's trade API appends to stat descriptions.
///
/// These are not part of the GGPK stat_descriptions.txt format strings.
/// Used by `poe-trade` when matching trade API text against the reverse index.
pub const TRADE_STAT_SUFFIXES: &[&str] = &[" (Local)", " (Shields)"];

// ── Trade API mod category prefixes ─────────────────────────────────────────
//
// WHY HARDCODED: The trade API at pathofexile.com categorizes stat filters
// by mod source (explicit, implicit, crafted, enchant, fractured). This
// mapping from mod display types to trade API category prefix strings is
// GGG trade system knowledge — it determines which stat pool a filter
// searches against. Fractured mods override the normal prefix even though
// the underlying mod is a prefix/suffix. Unique-item mods use "explicit"
// because the trade API has no dedicated unique category.
//
// Known categories (verified against 3.28 Mirage trade API):
// - "explicit" — prefix, suffix, and unique-item mods
// - "implicit" — all implicit mods (standard, Searing Exarch, Eater of Worlds)
// - "crafted" — bench-crafted mods
// - "enchant" — enchantments
// - "fractured" — fractured mods (overrides explicit)

// ── Trade API item category mapping ──────────────────────────────────────
//
// WHY HARDCODED: The trade API at pathofexile.com uses a hierarchical category
// system for item type filtering (e.g., `"armour.boots"`, `"weapon.bow"`).
// These category strings don't exist in the GGPK — they're a GGG trade system
// concept. The mapping from the Ctrl+Alt+C `Item Class:` header text to trade
// API category strings is stable across leagues.
//
// We map the raw item class string (e.g., `"Boots"`, `"Two Hand Axes"`) rather
// than using an enum, because item classes come directly from Ctrl+Alt+C text
// and the string set is defined by the GGPK (ItemClasses.datc64).
//
// Reference: awakened-poe-trade's CATEGORY_TO_TRADE_ID map.

/// Map an item class string (from Ctrl+Alt+C `Item Class:` header) to the
/// trade API category filter string.
///
/// Returns `None` for item classes that don't have a trade category filter
/// (currency, gems, quest items, etc.).
#[must_use]
pub fn item_class_trade_category(item_class: &str) -> Option<&'static str> {
    match item_class {
        // Armour
        "Body Armours" => Some("armour.chest"),
        "Boots" => Some("armour.boots"),
        "Gloves" => Some("armour.gloves"),
        "Helmets" => Some("armour.helmet"),
        "Shields" => Some("armour.shield"),
        "Quivers" => Some("armour.quiver"),

        // Weapons — one-handed
        "Claws" => Some("weapon.claw"),
        "Daggers" => Some("weapon.dagger"),
        "Rune Daggers" => Some("weapon.runedagger"),
        "One Hand Axes" => Some("weapon.oneaxe"),
        "One Hand Maces" => Some("weapon.onemace"),
        "One Hand Swords" | "Thrusting One Hand Swords" => Some("weapon.onesword"),
        "Sceptres" => Some("weapon.sceptre"),
        "Wands" => Some("weapon.wand"),

        // Weapons — two-handed
        "Bows" => Some("weapon.bow"),
        "Staves" => Some("weapon.staff"),
        "Warstaves" => Some("weapon.warstaff"),
        "Two Hand Axes" => Some("weapon.twoaxe"),
        "Two Hand Maces" => Some("weapon.twomace"),
        "Two Hand Swords" => Some("weapon.twosword"),

        // Accessories
        "Amulets" => Some("accessory.amulet"),
        "Belts" => Some("accessory.belt"),
        "Rings" => Some("accessory.ring"),
        "Trinkets" => Some("accessory.trinket"),

        // Jewels
        "Jewels" => Some("jewel"),
        "Abyss Jewels" => Some("jewel.abyss"),
        "Cluster Jewels" => Some("jewel.cluster"),

        // Flasks
        "Life Flasks" | "Mana Flasks" | "Hybrid Flasks" | "Utility Flasks" => Some("flask"),

        // Maps
        "Maps" => Some("map"),

        // Other
        "Divination Cards" => Some("card"),
        "Tinctures" => Some("tincture"),
        "Fishing Rods" => Some("weapon.rod"),

        // Heist
        "Heist Blueprints" => Some("heistmission.blueprint"),
        "Heist Contracts" => Some("heistmission.contract"),
        "Heist Tools" => Some("heistequipment.heisttool"),
        "Heist Brooches" => Some("heistequipment.heistreward"),
        "Heist Gear" => Some("heistequipment.heistweapon"),
        "Heist Cloaks" => Some("heistequipment.heistutility"),

        // Sanctum / misc
        "Sanctum Relics" => Some("sanctum.relic"),

        // No category filter for: currency, gems, fragments, quest items, etc.
        _ => None,
    }
}

/// Map a mod's display type to the trade API stat category prefix.
///
/// `display_type` is one of: `"prefix"`, `"suffix"`, `"implicit"`, `"crafted"`,
/// `"enchant"`, `"unique"` (matching `poe-item`'s `ModDisplayType` serialization).
///
/// Returns `"explicit"` for unknown display types.
#[must_use]
pub fn mod_trade_category(display_type: &str, is_fractured: bool) -> &'static str {
    if is_fractured {
        return "fractured";
    }
    match display_type {
        "implicit" => "implicit",
        "crafted" => "crafted",
        "enchant" => "enchant",
        // prefix, suffix, unique, and any unknown → explicit
        _ => "explicit",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_class_trade_categories() {
        // Armour
        assert_eq!(item_class_trade_category("Boots"), Some("armour.boots"));
        assert_eq!(item_class_trade_category("Body Armours"), Some("armour.chest"));
        assert_eq!(item_class_trade_category("Shields"), Some("armour.shield"));

        // Weapons
        assert_eq!(item_class_trade_category("Bows"), Some("weapon.bow"));
        assert_eq!(item_class_trade_category("Two Hand Axes"), Some("weapon.twoaxe"));
        assert_eq!(item_class_trade_category("Wands"), Some("weapon.wand"));
        assert_eq!(
            item_class_trade_category("Thrusting One Hand Swords"),
            Some("weapon.onesword"),
        );

        // Accessories
        assert_eq!(item_class_trade_category("Rings"), Some("accessory.ring"));
        assert_eq!(item_class_trade_category("Amulets"), Some("accessory.amulet"));

        // Jewels
        assert_eq!(item_class_trade_category("Jewels"), Some("jewel"));
        assert_eq!(item_class_trade_category("Abyss Jewels"), Some("jewel.abyss"));

        // Flasks — all variants map to "flask"
        assert_eq!(item_class_trade_category("Life Flasks"), Some("flask"));
        assert_eq!(item_class_trade_category("Utility Flasks"), Some("flask"));

        // Maps
        assert_eq!(item_class_trade_category("Maps"), Some("map"));

        // No category for currency/gems
        assert_eq!(item_class_trade_category("Stackable Currency"), None);
        assert_eq!(item_class_trade_category("Skill Gems"), None);
        assert_eq!(item_class_trade_category("Support Gems"), None);
        assert_eq!(item_class_trade_category("Map Fragments"), None);
    }
}
