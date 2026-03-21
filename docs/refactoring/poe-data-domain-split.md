# Plan: Split poe-data domain.rs into Submodules

**Priority:** 3
**Status:** TODO
**Effort:** ~2 hours
**Risk:** Low — pure module extraction, all functions keep same signatures

## Problem

`domain.rs` is 930 lines and growing. It contains tier classification, inherited tags (58 match arms),
pseudo definitions, trade mappings, and more. Each league adds new content here.

## Target Structure

```
crates/poe-data/src/
  domain/
    mod.rs              — Re-exports, top-level doc comment (~30 lines)
    tiers.rs            — TierQuality, classify_tier(), classify_tier_quality(), classify_rank()
    inheritance.rs      — inherited_tags_for_parent() (58-arm match)
    pseudos.rs          — PSEUDO_DEFINITIONS, DPS_PSEUDO_DEFINITIONS, PseudoDefinition types
    trade_mappings.rs   — mod_trade_category(), item_class_trade_category(), TRADE_STAT_SUFFIXES
    rarity.rs           — rarity_to_ggpk_id(), RARITY_MAP
    item_classes.rs     — is_weapon_class(), is_armour_class(), equipment checks
```

## Steps

1. Create `domain/` directory with mod.rs
2. Move tier functions → `tiers.rs`
3. Move inheritance → `inheritance.rs`
4. Move pseudo definitions → `pseudos.rs`
5. Move trade mappings → `trade_mappings.rs`
6. Move rarity helpers → `rarity.rs`
7. Move item class checks → `item_classes.rs`
8. Update mod.rs to `pub use` everything (public API unchanged)
9. Run `cargo clippy --workspace --tests`
10. Run `cargo test -p poe-data`

## Constraint

The public API must not change. All functions remain at `poe_data::domain::*`.
The mod.rs uses `pub use` to re-export everything from submodules.

## Also: game_data.rs (993 lines)

Not splitting yet — the 40+ lookup methods are cohesive (all operate on GameData).
But two helpers could be extracted:

- **find_eligible_mod_candidates()**: shared filtering logic between `find_eligible_mod()` and
  `find_eligible_mods()` (~60 duplicated lines)
- **stat_suggestions_for_query()** + helpers: could become `suggestions.rs` if the file crosses 1100 lines
