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

// ── League mechanic name prefixes ──────────────────────────────────────────
//
// WHY HARDCODED: The PoE client prepends league mechanic names to unique item
// names (e.g., "Foulborn Soulthirst"). The trade API rejects the prefixed name
// as "Unknown item name". The prefix must be stripped before querying.
//
// The GGPK contains the prefix string in ClientStrings
// (`ModDescriptionLineBrequelMutated` = "Foulborn Unique Modifier").
// Confirmed via 3.28 Mirage.

/// League mechanic prefixes prepended to unique item names by the `PoE` client.
///
/// The trade API does not recognize these prefixes — they must be stripped
/// from the item name before building trade queries.
pub const LEAGUE_NAME_PREFIXES: &[&str] = &["Foulborn "];

/// Strip any known league mechanic prefix from an item name.
///
/// Returns the original string unchanged if no prefix is found.
#[must_use]
pub fn strip_league_prefix(name: &str) -> &str {
    for prefix in LEAGUE_NAME_PREFIXES {
        if let Some(stripped) = name.strip_prefix(prefix) {
            return stripped;
        }
    }
    name
}

/// Whether an item name has a known league mechanic prefix (e.g., "Foulborn").
///
/// Used to set the trade API `mutated` misc filter.
#[must_use]
pub fn has_league_prefix(name: &str) -> bool {
    LEAGUE_NAME_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
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
