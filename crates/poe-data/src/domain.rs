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
/// These are not part of the GGPK `stat_descriptions.txt` format strings.
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
/// Trade API convention, not in GGPK (verified 2026-03-15).
/// Checked: `BaseItemTypes` (no `TradeMarketCategory` field), `ItemClasses`,
/// `ItemClassCategories`. GGG's trade site uses its own category URL scheme.
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

/// Whether an item class is a weapon (has weapon filters on trade site).
///
/// TODO: Replace with `ItemClasses` capability flags from GGPK.
/// The `ItemClasses` table has `CanHaveInfluence`, `CanBeFractured`, etc.
/// and the `ItemClassCategories` FK distinguishes weapon/armour/accessory.
/// See `docs/data-driven-plan.md` Phase 2.
#[must_use]
pub fn is_weapon_class(item_class: &str) -> bool {
    item_class_trade_category(item_class).is_some_and(|cat| cat.starts_with("weapon."))
}

/// Whether an item class is armour/shield/quiver (has armour filters on trade site).
///
/// TODO: Replace with `ItemClasses` capability flags from GGPK.
/// See `is_weapon_class` comment above.
#[must_use]
pub fn is_armour_class(item_class: &str) -> bool {
    item_class_trade_category(item_class).is_some_and(|cat| cat.starts_with("armour."))
}

// ── Mod domain mapping ───────────────────────────────────────────────────
//
// WHY HARDCODED: The GGPK `Mods.datc64` stores a numeric `domain` field that
// partitions mods by item type (equipment, jewels, flasks, monster mods, etc.).
// The numeric → meaning mapping isn't in any GGPK table — it's implicit in
// GGG's code. Community tools (RePoE, poedb) document these values.
//
// `find_eligible_mod()` uses this to filter out mods from wrong domains
// (e.g., monster mods, abyss jewel mods on equipment, etc.). Without
// domain filtering, mods with high `default` spawn weights (like monster
// mods) pollute the results.
//
// Confirmed via 3.28 GGPK mod analysis.

/// Map an item class (from Ctrl+Alt+C header) to the expected mod domain(s).
///
/// Returns the GGPK numeric domain values that are valid for this item class.
/// Mods outside these domains should not be considered for this item type.
///
/// Known domains:
/// - 1 = item (regular equipment: armour, weapons, accessories)
/// - 2 = flask
/// - 3 = monster
/// - 5 = area
/// - 9 = crafted (bench crafts — also `generation_type=10`)
/// - 10 = jewel
/// - 11 = atlas passive (legacy)
/// - 13 = abyss jewel
/// - 19 = delve (fossil-specific)
#[must_use]
pub fn item_class_mod_domains(item_class: &str) -> &'static [u32] {
    match item_class {
        "Abyss Jewels" => &[13, 9],
        "Jewels" | "Cluster Jewels" => &[10, 9],
        "Life Flasks" | "Mana Flasks" | "Hybrid Flasks" | "Utility Flasks" => &[2, 9],
        // All regular equipment (armour, weapons, accessories, shields, quivers)
        _ => &[1, 9],
    }
}

// ── Local stat display fallbacks ──────────────────────────────────────────
//
// WHY HARDCODED: Local stats used in armour mods (flat armour, evasion,
// energy shield) don't have entries in stat_descriptions.txt. PoE renders
// them using the base defence property display, not the stat description
// system. When building `stat_id_to_templates`, we first try stripping the
// `local_` prefix to find the non-local equivalent. This table covers
// remaining cases where the naming convention doesn't match.
//
// Known case: `local_energy_shield` → non-local is `base_maximum_energy_shield`,
// not `energy_shield` (which doesn't exist in the stats table).

/// Fallback mapping from local stat IDs to their non-local equivalents,
/// for cases where stripping the `local_` prefix doesn't find a match.
///
/// Used by `set_reverse_index()` to populate `stat_id_to_templates`.
pub const LOCAL_STAT_NONLOCAL_FALLBACKS: &[(&str, &str)] =
    &[("local_energy_shield", "base_maximum_energy_shield")];

// ── Quality prefix ──────────────────────────────────────────────────────────
//
// WHY HARDCODED: The PoE client prepends a localized quality prefix to item
// names when quality > 0 (e.g., "Superior Ezomyte Tower Shield"). This prefix
// is NOT part of the base item type name in the GGPK BaseItemTypes table.
// The trade API rejects the prefixed name as "Unknown item base type".
//
// The prefix appears on:
// - Normal items with quality > 0 (always "Superior" in English)
// - Unidentified Magic/Rare/Unique items with quality > 0
//
// The GGPK does contain this string in a localization table, but we only
// support English currently. Confirmed via 3.28 Mirage.

/// The English quality prefix prepended to item names by the `PoE` client.
///
/// Used by `poe-item`'s resolver to strip the prefix from base type names
/// before sending to the trade API or looking up in `BaseItemTypes`.
pub const QUALITY_PREFIX: &str = "Superior ";

/// Strip the quality prefix from an item name, if present.
///
/// Returns the original string unchanged if the prefix is not found.
#[must_use]
pub fn strip_quality_prefix(name: &str) -> &str {
    name.strip_prefix(QUALITY_PREFIX).unwrap_or(name)
}

/// Map a mod's display type to the trade API stat category prefix.
///
/// Trade API convention, not in GGPK (verified 2026-03-15).
/// The trade API's `stats.json` organizes stat filters by category
/// (explicit, implicit, fractured, crafted, enchant). This mapping
/// determines which category prefix to use when building trade stat IDs.
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
        "pseudo" => "pseudo",
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
        assert_eq!(
            item_class_trade_category("Body Armours"),
            Some("armour.chest")
        );
        assert_eq!(item_class_trade_category("Shields"), Some("armour.shield"));

        // Weapons
        assert_eq!(item_class_trade_category("Bows"), Some("weapon.bow"));
        assert_eq!(
            item_class_trade_category("Two Hand Axes"),
            Some("weapon.twoaxe")
        );
        assert_eq!(item_class_trade_category("Wands"), Some("weapon.wand"));
        assert_eq!(
            item_class_trade_category("Thrusting One Hand Swords"),
            Some("weapon.onesword"),
        );

        // Accessories
        assert_eq!(item_class_trade_category("Rings"), Some("accessory.ring"));
        assert_eq!(
            item_class_trade_category("Amulets"),
            Some("accessory.amulet")
        );

        // Jewels
        assert_eq!(item_class_trade_category("Jewels"), Some("jewel"));
        assert_eq!(
            item_class_trade_category("Abyss Jewels"),
            Some("jewel.abyss")
        );

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

// ── Pseudo stat definitions ─────────────────────────────────────────────────
//
// WHY HARDCODED: The GGPK has no concept of pseudo stats. These are trade API
// constructs that aggregate multiple mods into a single searchable value.
// ModFamily names from GGPK tell us which mods belong to which group, but
// the aggregation rules (which families sum together, with what multipliers)
// are game mechanics knowledge not declared in any data file.
//
// Reference: Awakened PoE Trade pseudo rules
//   `_reference/awakened-poe-trade/renderer/src/web/price-check/filters/pseudo/index.ts`
// Trade API pseudo IDs: `crates/poe-trade/tests/fixtures/trade_stats_3.28.json`
// ModFamily list: `crates/poe-data/data/mod_families.txt`

/// A component of a pseudo stat — one mod family that contributes to the aggregate.
#[derive(Debug, Clone)]
pub struct PseudoComponent {
    /// `ModFamily` name from GGPK (e.g., `"Strength"`).
    pub family: &'static str,
    /// Multiplier applied to the stat value (e.g., 0.5 for Strength → Life).
    pub multiplier: f64,
    /// If true, this pseudo only appears when this component has a value on the item.
    pub required: bool,
}

/// Definition of a pseudo stat — which mod families contribute and how.
#[derive(Debug, Clone)]
pub struct PseudoDefinition {
    /// ID matching trade API suffix (e.g., `"pseudo_total_life"`).
    pub id: &'static str,
    /// Display label template (e.g., `"+# total maximum Life"`).
    pub label: &'static str,
    /// Component families with multipliers.
    pub components: &'static [PseudoComponent],
}

const fn comp(family: &'static str, multiplier: f64, required: bool) -> PseudoComponent {
    PseudoComponent {
        family,
        multiplier,
        required,
    }
}

/// Phase 1 pseudo stat definitions (~20 commonly used for pricing).
///
/// Each definition maps a pseudo stat to one or more `ModFamily` groups.
/// At load time, families are resolved to concrete `stat_ids` via the Mods table.
pub static PSEUDO_DEFINITIONS: &[PseudoDefinition] = &[
    // ── Resistances ──────────────────────────────────────────────────
    PseudoDefinition {
        id: "pseudo_total_fire_resistance",
        label: "(Pseudo) +#% total to Fire Resistance",
        components: &[
            comp("FireResistance", 1.0, false),
            comp("FireResistancePrefix", 1.0, false),
            comp("AllResistances", 1.0, false),
            comp("AllResistancesWithChaos", 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_cold_resistance",
        label: "(Pseudo) +#% total to Cold Resistance",
        components: &[
            comp("ColdResistance", 1.0, false),
            comp("ColdResistancePrefix", 1.0, false),
            comp("AllResistances", 1.0, false),
            comp("AllResistancesWithChaos", 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_lightning_resistance",
        label: "(Pseudo) +#% total to Lightning Resistance",
        components: &[
            comp("LightningResistance", 1.0, false),
            comp("LightningResistancePrefix", 1.0, false),
            comp("AllResistances", 1.0, false),
            comp("AllResistancesWithChaos", 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_chaos_resistance",
        label: "(Pseudo) +#% total to Chaos Resistance",
        components: &[
            comp("ChaosResistance", 1.0, false),
            comp("ChaosResistancePrefix", 1.0, false),
            comp("AllResistancesWithChaos", 1.0, false),
        ],
    },
    // ── Attributes ───────────────────────────────────────────────────
    PseudoDefinition {
        id: "pseudo_total_strength",
        label: "(Pseudo) +# total to Strength",
        components: &[
            comp("Strength", 1.0, false),
            comp("AllAttributes", 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_dexterity",
        label: "(Pseudo) +# total to Dexterity",
        components: &[
            comp("Dexterity", 1.0, false),
            comp("AllAttributes", 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_intelligence",
        label: "(Pseudo) +# total to Intelligence",
        components: &[
            comp("Intelligence", 1.0, false),
            comp("AllAttributes", 1.0, false),
        ],
    },
    // ── Life / Mana / ES ─────────────────────────────────────────────
    PseudoDefinition {
        id: "pseudo_total_life",
        label: "(Pseudo) +# total maximum Life",
        components: &[
            comp("IncreasedLife", 1.0, true),
            // Each point of Strength gives 0.5 life
            comp("Strength", 0.5, false),
            comp("AllAttributes", 0.5, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_mana",
        label: "(Pseudo) +# total maximum Mana",
        components: &[
            comp("IncreasedMana", 1.0, true),
            // Each point of Intelligence gives 0.5 mana
            comp("Intelligence", 0.5, false),
            comp("AllAttributes", 0.5, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_energy_shield",
        label: "(Pseudo) +# total maximum Energy Shield",
        components: &[comp("IncreasedEnergyShield", 1.0, false)],
    },
    PseudoDefinition {
        id: "pseudo_increased_energy_shield",
        label: "(Pseudo) #% total increased maximum Energy Shield",
        components: &[
            comp("MaximumLifeIncreasePercent", 1.0, false), // TODO: verify family
        ],
    },
    // ── Speed ────────────────────────────────────────────────────────
    PseudoDefinition {
        id: "pseudo_increased_movement_speed",
        label: "(Pseudo) #% increased Movement Speed",
        components: &[comp("MovementVelocity", 1.0, false)],
    },
    // ── Damage ───────────────────────────────────────────────────────
    PseudoDefinition {
        id: "pseudo_increased_physical_damage",
        label: "(Pseudo) #% total increased Physical Damage",
        components: &[comp("PhysicalDamage", 1.0, false)],
    },
];

/// Returns all pseudo stat definitions.
#[must_use]
pub fn pseudo_definitions() -> &'static [PseudoDefinition] {
    PSEUDO_DEFINITIONS
}
