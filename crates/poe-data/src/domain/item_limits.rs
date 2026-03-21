// ── Per-item-class affix limits ─────────────────────────────────────────────
//
// WHY HARDCODED: The GGPK `Rarity.datc64` table provides global max prefix/suffix
// counts per rarity (Normal=0, Magic=1, Rare=3, Unique=varies). However, certain
// item classes have lower limits that are hardcoded in GGG's game client, not stored
// in any GGPK table. The `ItemClasses.datc64` schema has no mod limit fields
// (verified against dat-schema for 3.28 Mirage).
//
// Known overrides (verified via PoE wiki + in-game observation, 2026-03-21):
// - All jewel types (Jewels, Abyss Jewels, Cluster Jewels): Rare = 2 prefix / 2 suffix
//   (vs the global Rare limit of 3/3)
// - Magic items: 1/1 across all item classes (no override needed)
//
// If no override exists, callers should fall back to the Rarity table.

/// Per-item-class affix limit override.
///
/// Returns `Some((max_prefix, max_suffix))` if this item class has limits
/// that differ from the global `Rarity` table. Returns `None` to use the
/// rarity-based defaults.
///
/// Game mechanic, not in GGPK (verified 2026-03-21).
/// Checked: `ItemClasses.datc64` (no mod limit fields), `Rarity.datc64` (global only).
#[must_use]
pub fn item_class_affix_limit(item_class: &str, rarity_id: &str) -> Option<(i32, i32)> {
    match (item_class, rarity_id) {
        // Jewels: all types capped at 2/2 for Rare (global Rare = 3/3)
        ("Jewels" | "Abyss Jewels" | "Cluster Jewels", "Rare") => Some((2, 2)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jewel_overrides() {
        assert_eq!(item_class_affix_limit("Jewels", "Rare"), Some((2, 2)));
        assert_eq!(item_class_affix_limit("Abyss Jewels", "Rare"), Some((2, 2)));
        assert_eq!(
            item_class_affix_limit("Cluster Jewels", "Rare"),
            Some((2, 2))
        );
    }

    #[test]
    fn jewel_magic_uses_default() {
        assert_eq!(item_class_affix_limit("Jewels", "Magic"), None);
        assert_eq!(item_class_affix_limit("Abyss Jewels", "Magic"), None);
    }

    #[test]
    fn equipment_uses_default() {
        assert_eq!(item_class_affix_limit("Body Armours", "Rare"), None);
        assert_eq!(item_class_affix_limit("Boots", "Rare"), None);
        assert_eq!(item_class_affix_limit("Bows", "Rare"), None);
        assert_eq!(item_class_affix_limit("Rings", "Rare"), None);
    }
}
