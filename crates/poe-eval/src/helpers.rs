//! Shared helpers used across evaluation modules.

use poe_item::types::Rarity;

/// Convert a `Rarity` to its GGPK numeric ID.
///
/// Used by affix analysis and open-mod-count evaluation to look up
/// rarity-dependent limits (max prefixes/suffixes).
pub(crate) fn rarity_ggpk_id(rarity: Rarity) -> Option<&'static str> {
    poe_data::domain::rarity_to_ggpk_id(&format!("{rarity:?}"))
}
