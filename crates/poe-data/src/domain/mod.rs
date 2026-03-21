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

mod inheritance;
mod pseudos;
mod tiers;
mod trade_mappings;

// ── Re-exports (public API unchanged) ───────────────────────────────────────

pub use inheritance::{LOCAL_STAT_NONLOCAL_FALLBACKS, inherited_tags_for_parent};
pub use pseudos::{
    DPS_PSEUDO_DEFINITIONS, DpsPseudoDefinition, DpsPseudoKind, PSEUDO_DEFINITIONS,
    PseudoComponent, PseudoDefinition, dps_pseudo_definitions, dps_weapon_filter, is_dps_pseudo,
    pseudo_definitions,
};
pub use tiers::{
    TierQuality, classify_rank, classify_tier, classify_tier_relative, rarity_to_ggpk_id,
};
pub use trade_mappings::{
    QUALITY_PREFIX, TRADE_STAT_SUFFIXES, is_armour_class, is_weapon_class, item_class_mod_domains,
    item_class_trade_category, mod_trade_category, strip_quality_prefix,
};
