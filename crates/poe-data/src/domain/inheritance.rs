// ── Inherited tags ──────────────────────────────────────────────────────────
//
// WHY HARDCODED: In the GGPK, BaseItemTypes.TagsKeys only stores tags
// SPECIFIC to that base type (e.g., Vaal Regalia → [int_armour, default]).
// Equipment also inherits tags from its abstract parent defined in .ot files
// (e.g., AbstractBow defines [bow, weapon, two_hand_weapon, default, ranged]).
// These .ot files are binary assets in the GGPK bundles — not datc64 tables —
// so we can't extract them through poe-dat. The tags below are verified against
// mod spawn_weight_tags in the Mods table (2026-03-18).
//
// Used by `GameData::resolve_inherited_tags()` to merge inherited tags with
// base-type-specific tags, enabling `find_eligible_mod()` to work for all items.

/// Map from `BaseItemTypes.InheritsFrom` metadata path → tag IDs that the
/// abstract parent contributes. Only includes equipment types relevant for
/// mod spawning.
pub fn inherited_tags_for_parent(inherits_from: &str) -> &'static [&'static str] {
    match inherits_from {
        // ── Weapons ────────────────────────────────────────────────────
        // One-hand
        s if s.ends_with("/AbstractClaw") => &["weapon", "one_hand_weapon", "claw", "default"],
        s if s.ends_with("/AbstractDagger") => &["weapon", "one_hand_weapon", "dagger", "default"],
        s if s.ends_with("/AbstractRuneDagger") => {
            &["weapon", "one_hand_weapon", "dagger", "default"]
        }
        s if s.ends_with("/AbstractOneHandAxe") => &["weapon", "one_hand_weapon", "axe", "default"],
        s if s.ends_with("/AbstractOneHandMace") => {
            &["weapon", "one_hand_weapon", "mace", "default"]
        }
        s if s.ends_with("/AbstractSceptre") => {
            &["weapon", "one_hand_weapon", "sceptre", "default"]
        }
        s if s.ends_with("/AbstractOneHandSword") => {
            &["weapon", "one_hand_weapon", "sword", "default"]
        }
        s if s.ends_with("/AbstractOneHandSwordThrusting") => {
            &["weapon", "one_hand_weapon", "sword", "default"]
        }
        s if s.ends_with("/AbstractWand") => &["weapon", "one_hand_weapon", "wand", "default"],
        // Two-hand
        s if s.ends_with("/AbstractBow") => &["weapon", "two_hand_weapon", "bow", "default"],
        s if s.ends_with("/AbstractStaff") => &["weapon", "two_hand_weapon", "staff", "default"],
        s if s.ends_with("/AbstractWarstaff") => &["weapon", "two_hand_weapon", "staff", "default"],
        s if s.ends_with("/AbstractTwoHandAxe") => &["weapon", "two_hand_weapon", "axe", "default"],
        s if s.ends_with("/AbstractTwoHandMace") => {
            &["weapon", "two_hand_weapon", "mace", "default"]
        }
        s if s.ends_with("/AbstractTwoHandSword") => {
            &["weapon", "two_hand_weapon", "sword", "default"]
        }
        // ── Armour ─────────────────────────────────────────────────────
        s if s.ends_with("/AbstractBodyArmour") => &["armour", "body_armour", "default"],
        s if s.ends_with("/AbstractBoots") => &["armour", "boots", "default"],
        s if s.ends_with("/AbstractGloves") => &["armour", "gloves", "default"],
        s if s.ends_with("/AbstractHelmet") => &["armour", "helmet", "default"],
        s if s.ends_with("/AbstractShield") => &["armour", "shield", "default"],
        // ── Jewellery ──────────────────────────────────────────────────
        s if s.ends_with("/AbstractAmulet") => &["amulet", "default"],
        s if s.ends_with("/AbstractRing") => &["ring", "default"],
        s if s.ends_with("/AbstractBelt") => &["belt", "default"],
        // ── Quiver ─────────────────────────────────────────────────────
        s if s.ends_with("/AbstractQuiver") => &["quiver", "default"],
        // ── Jewels ─────────────────────────────────────────────────────
        s if s.ends_with("/AbstractJewel") => &["jewel", "default"],
        s if s.ends_with("/AbstractAbyssJewel") => &["abyss_jewel", "default"],
        // ── Flasks ─────────────────────────────────────────────────────
        s if s.ends_with("/AbstractLifeFlask") => &["flask", "default"],
        s if s.ends_with("/AbstractManaFlask") => &["flask", "default"],
        s if s.ends_with("/AbstractHybridFlask") => &["flask", "default"],
        s if s.ends_with("/AbstractUtilityFlask") => &["flask", "default"],
        // ── Tinctures ──────────────────────────────────────────────────
        s if s.ends_with("/AbstractTincture") => &["tincture", "default"],
        _ => &[],
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
