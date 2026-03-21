// ── Pseudo stat definitions ─────────────────────────────────────────────────
//
// WHY HARDCODED: The GGPK has no concept of pseudo stats. These are trade API
// constructs that aggregate multiple mods into a single searchable value.
// The GGPK `ModFamily` table groups mods for anti-stacking purposes (e.g., you
// can't have two `IncreasedLife` mods), but families are mod-level tags — a
// hybrid armour+life mod in `IncreasedLife` family has BOTH armour and life
// stat_ids, so family→stat_id mapping is ambiguous for hybrids.
//
// Instead, pseudo components list explicit stat_ids. Use `mod_families.txt`
// as a discovery aid when authoring new definitions.
//
// Trade API pseudo IDs: `crates/poe-trade/tests/fixtures/trade_stats_3.28.json`
// ModFamily list: `crates/poe-data/data/mod_families.txt`

/// A component of a pseudo stat — a set of `stat_ids` that contribute to the aggregate.
#[derive(Debug, Clone)]
pub struct PseudoComponent {
    /// Stat IDs that match this component (e.g., `["base_fire_damage_resistance_%"]`).
    pub stat_ids: &'static [&'static str],
    /// Multiplier applied to the stat value (e.g., 0.5 for Strength → Life).
    pub multiplier: f64,
    /// If true, this pseudo only appears when this component has a value on the item.
    pub required: bool,
}

/// Definition of a pseudo stat — which `stat_ids` contribute and how.
#[derive(Debug, Clone)]
pub struct PseudoDefinition {
    /// ID matching trade API suffix (e.g., `"pseudo_total_life"`).
    pub id: &'static str,
    /// Display label template (e.g., `"(Pseudo) +# total maximum Life"`).
    pub label: &'static str,
    /// Component `stat_ids` with multipliers.
    pub components: &'static [PseudoComponent],
}

const fn comp(
    stat_ids: &'static [&'static str],
    multiplier: f64,
    required: bool,
) -> PseudoComponent {
    PseudoComponent {
        stat_ids,
        multiplier,
        required,
    }
}

/// Pseudo stat definitions — trade API aggregates computed from item stats.
///
/// Each component lists explicit `stat_ids` (including local_ variants for weapon
/// mods). Use `crates/poe-data/data/mod_families.txt` as a reference when
/// adding new definitions — families help discover which `stat_ids` are related,
/// but the definition must use exact `stat_ids` (not family names).
pub static PSEUDO_DEFINITIONS: &[PseudoDefinition] = &[
    // ── Resistances ──────────────────────────────────────────────────
    PseudoDefinition {
        id: "pseudo_total_fire_resistance",
        label: "(Pseudo) +#% total to Fire Resistance",
        components: &[
            comp(&["base_fire_damage_resistance_%"], 1.0, false),
            comp(&["base_resist_all_elements_%"], 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_cold_resistance",
        label: "(Pseudo) +#% total to Cold Resistance",
        components: &[
            comp(&["base_cold_damage_resistance_%"], 1.0, false),
            comp(&["base_resist_all_elements_%"], 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_lightning_resistance",
        label: "(Pseudo) +#% total to Lightning Resistance",
        components: &[
            comp(&["base_lightning_damage_resistance_%"], 1.0, false),
            comp(&["base_resist_all_elements_%"], 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_chaos_resistance",
        label: "(Pseudo) +#% total to Chaos Resistance",
        components: &[comp(&["base_chaos_damage_resistance_%"], 1.0, false)],
    },
    PseudoDefinition {
        id: "pseudo_total_elemental_resistance",
        label: "(Pseudo) +#% total Elemental Resistance",
        components: &[
            comp(&["base_fire_damage_resistance_%"], 1.0, false),
            comp(&["base_cold_damage_resistance_%"], 1.0, false),
            comp(&["base_lightning_damage_resistance_%"], 1.0, false),
            // All-elements counts for each of the 3 elemental resistances
            comp(&["base_resist_all_elements_%"], 3.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_resistance",
        label: "(Pseudo) +#% total Resistance",
        components: &[
            comp(&["base_fire_damage_resistance_%"], 1.0, false),
            comp(&["base_cold_damage_resistance_%"], 1.0, false),
            comp(&["base_lightning_damage_resistance_%"], 1.0, false),
            comp(&["base_chaos_damage_resistance_%"], 1.0, false),
            // All-elements counts for 3 elemental resistances (not chaos)
            comp(&["base_resist_all_elements_%"], 3.0, false),
        ],
    },
    // ── Attributes ───────────────────────────────────────────────────
    PseudoDefinition {
        id: "pseudo_total_strength",
        label: "(Pseudo) +# total to Strength",
        components: &[
            comp(&["additional_strength"], 1.0, false),
            comp(
                &[
                    "additional_strength_and_dexterity",
                    "additional_strength_and_intelligence",
                ],
                1.0,
                false,
            ),
            comp(&["additional_all_attributes"], 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_dexterity",
        label: "(Pseudo) +# total to Dexterity",
        components: &[
            comp(&["additional_dexterity"], 1.0, false),
            comp(
                &[
                    "additional_strength_and_dexterity",
                    "additional_dexterity_and_intelligence",
                ],
                1.0,
                false,
            ),
            comp(&["additional_all_attributes"], 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_intelligence",
        label: "(Pseudo) +# total to Intelligence",
        components: &[
            comp(&["additional_intelligence"], 1.0, false),
            comp(
                &[
                    "additional_strength_and_intelligence",
                    "additional_dexterity_and_intelligence",
                ],
                1.0,
                false,
            ),
            comp(&["additional_all_attributes"], 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_all_attributes",
        label: "(Pseudo) +# total to all Attributes",
        components: &[comp(&["additional_all_attributes"], 1.0, false)],
    },
    // ── Life / Mana / ES ─────────────────────────────────────────────
    PseudoDefinition {
        id: "pseudo_total_life",
        label: "(Pseudo) +# total maximum Life",
        components: &[
            comp(&["base_maximum_life"], 1.0, true),
            // Each point of Strength gives 0.5 life
            comp(&["additional_strength"], 0.5, false),
            comp(
                &[
                    "additional_strength_and_dexterity",
                    "additional_strength_and_intelligence",
                ],
                0.5,
                false,
            ),
            comp(&["additional_all_attributes"], 0.5, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_mana",
        label: "(Pseudo) +# total maximum Mana",
        components: &[
            comp(&["base_maximum_mana"], 1.0, true),
            // Each point of Intelligence gives 0.5 mana
            comp(&["additional_intelligence"], 0.5, false),
            comp(
                &[
                    "additional_strength_and_intelligence",
                    "additional_dexterity_and_intelligence",
                ],
                0.5,
                false,
            ),
            comp(&["additional_all_attributes"], 0.5, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_total_energy_shield",
        label: "(Pseudo) +# total maximum Energy Shield",
        components: &[comp(
            &["base_maximum_energy_shield", "local_energy_shield"],
            1.0,
            false,
        )],
    },
    // ── Speed ────────────────────────────────────────────────────────
    PseudoDefinition {
        id: "pseudo_increased_movement_speed",
        label: "(Pseudo) #% increased Movement Speed",
        components: &[comp(&["base_movement_velocity_+%"], 1.0, false)],
    },
    PseudoDefinition {
        id: "pseudo_total_attack_speed",
        label: "(Pseudo) +#% total Attack Speed",
        components: &[comp(
            &["attack_speed_+%", "local_attack_speed_+%"],
            1.0,
            false,
        )],
    },
    PseudoDefinition {
        id: "pseudo_total_cast_speed",
        label: "(Pseudo) +#% total Cast Speed",
        components: &[comp(&["base_cast_speed_+%"], 1.0, false)],
    },
    // ── Critical Strike ─────────────────────────────────────────────
    PseudoDefinition {
        id: "pseudo_global_critical_strike_chance",
        label: "(Pseudo) +#% Global Critical Strike Chance",
        components: &[comp(&["critical_strike_chance_+%"], 1.0, false)],
    },
    PseudoDefinition {
        id: "pseudo_global_critical_strike_multiplier",
        label: "(Pseudo) +#% Global Critical Strike Multiplier",
        components: &[comp(&["base_critical_strike_multiplier_+"], 1.0, false)],
    },
    // ── Damage ───────────────────────────────────────────────────────
    PseudoDefinition {
        id: "pseudo_increased_physical_damage",
        label: "(Pseudo) #% total increased Physical Damage",
        components: &[comp(
            &["physical_damage_+%", "local_physical_damage_+%"],
            1.0,
            false,
        )],
    },
    PseudoDefinition {
        id: "pseudo_increased_elemental_damage",
        label: "(Pseudo) #% increased Elemental Damage",
        components: &[comp(&["elemental_damage_+%"], 1.0, false)],
    },
    PseudoDefinition {
        id: "pseudo_increased_fire_damage",
        label: "(Pseudo) #% increased Fire Damage",
        components: &[
            comp(&["fire_damage_+%"], 1.0, false),
            comp(&["elemental_damage_+%"], 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_increased_cold_damage",
        label: "(Pseudo) #% increased Cold Damage",
        components: &[
            comp(&["cold_damage_+%"], 1.0, false),
            comp(&["elemental_damage_+%"], 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_increased_lightning_damage",
        label: "(Pseudo) #% increased Lightning Damage",
        components: &[
            comp(&["lightning_damage_+%"], 1.0, false),
            comp(&["elemental_damage_+%"], 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_increased_spell_damage",
        label: "(Pseudo) #% increased Spell Damage",
        components: &[comp(&["spell_damage_+%"], 1.0, false)],
    },
    // ── Accuracy ────────────────────────────────────────────────────
    PseudoDefinition {
        id: "pseudo_total_accuracy_rating",
        label: "(Pseudo) +# total Accuracy Rating",
        components: &[comp(&["accuracy_rating"], 1.0, false)],
    },
    // ── Regeneration ────────────────────────────────────────────────
    PseudoDefinition {
        id: "pseudo_total_life_regen",
        label: "(Pseudo) # Life Regenerated per Second",
        components: &[
            // GGPK stores regen as per-minute; display divides by 60
            comp(&["life_regeneration_rate_per_minute_%"], 1.0, false),
        ],
    },
    PseudoDefinition {
        id: "pseudo_increased_mana_regen",
        label: "(Pseudo) #% increased Mana Regeneration Rate",
        components: &[comp(&["mana_regeneration_rate_+%"], 1.0, false)],
    },
];

/// Returns all pseudo stat definitions.
#[must_use]
pub fn pseudo_definitions() -> &'static [PseudoDefinition] {
    PSEUDO_DEFINITIONS
}

// ── Pseudo hierarchy (subsumption) ──────────────────────────────────────
//
// WHY HARDCODED: Some pseudos are strictly broader than others. Total
// resistance covers elemental resistance which covers individual element
// resistances. When a broader pseudo is active, narrower ones are
// redundant in a trade search. This hierarchy doesn't exist in the GGPK.

/// Pseudo IDs that a given pseudo subsumes (makes redundant).
///
/// When the key pseudo is auto-selected, the value pseudos should be
/// auto-excluded to avoid redundant filters. User overrides still win.
///
/// Game mechanic, not in GGPK (verified 2026-03-21).
#[must_use]
pub fn pseudo_subsumes(pseudo_id: &str) -> &'static [&'static str] {
    match pseudo_id {
        // Total resistance covers elemental + individual resistances
        "pseudo_total_resistance" => &[
            "pseudo_total_elemental_resistance",
            "pseudo_total_fire_resistance",
            "pseudo_total_cold_resistance",
            "pseudo_total_lightning_resistance",
            "pseudo_total_chaos_resistance",
        ],
        // Elemental resistance covers individual element resistances
        "pseudo_total_elemental_resistance" => &[
            "pseudo_total_fire_resistance",
            "pseudo_total_cold_resistance",
            "pseudo_total_lightning_resistance",
        ],
        _ => &[],
    }
}

// ── DPS pseudo definitions ──────────────────────────────────────────────
//
// WHY SEPARATE: DPS is computed from displayed weapon properties (damage × APS),
// not summed from stat_ids like regular pseudos. The GGPK has no DPS concept —
// it stores base damage + local mods, and the PoE client computes final values.
// The trade API handles DPS via `weapon_filters` (pdps/edps/dps), not pseudo IDs.

/// Which kind of DPS this definition computes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpsPseudoKind {
    Physical,
    Elemental,
    Chaos,
    Total,
}

/// Definition of a DPS pseudo stat — computed from weapon properties, not stat sums.
#[derive(Debug, Clone)]
pub struct DpsPseudoDefinition {
    /// Internal ID (e.g., `"pseudo_physical_dps"`). NOT a trade API pseudo stat ID.
    pub id: &'static str,
    /// Display label template (e.g., `"(Pseudo) # Physical DPS"`).
    pub label: &'static str,
    /// Which DPS component this represents.
    pub kind: DpsPseudoKind,
    /// Trade API `weapon_filters` key (e.g., `"pdps"`). `None` if no trade filter exists.
    pub trade_weapon_filter: Option<&'static str>,
}

pub static DPS_PSEUDO_DEFINITIONS: &[DpsPseudoDefinition] = &[
    DpsPseudoDefinition {
        id: "pseudo_physical_dps",
        label: "(Pseudo) # Physical DPS",
        kind: DpsPseudoKind::Physical,
        trade_weapon_filter: Some("pdps"),
    },
    DpsPseudoDefinition {
        id: "pseudo_elemental_dps",
        label: "(Pseudo) # Elemental DPS",
        kind: DpsPseudoKind::Elemental,
        trade_weapon_filter: Some("edps"),
    },
    DpsPseudoDefinition {
        id: "pseudo_chaos_dps",
        label: "(Pseudo) # Chaos DPS",
        kind: DpsPseudoKind::Chaos,
        // Trade API has no chaos DPS filter
        trade_weapon_filter: None,
    },
    DpsPseudoDefinition {
        id: "pseudo_total_dps",
        label: "(Pseudo) # Total DPS",
        kind: DpsPseudoKind::Total,
        trade_weapon_filter: Some("dps"),
    },
];

/// Returns all DPS pseudo stat definitions.
#[must_use]
pub fn dps_pseudo_definitions() -> &'static [DpsPseudoDefinition] {
    DPS_PSEUDO_DEFINITIONS
}

/// Whether a pseudo stat ID is a DPS pseudo (not tradeable as a pseudo stat).
#[must_use]
pub fn is_dps_pseudo(pseudo_id: &str) -> bool {
    DPS_PSEUDO_DEFINITIONS.iter().any(|d| d.id == pseudo_id)
}

/// Get the `weapon_filters` key for a DPS pseudo stat ID, if one exists.
#[must_use]
pub fn dps_weapon_filter(pseudo_id: &str) -> Option<&'static str> {
    DPS_PSEUDO_DEFINITIONS
        .iter()
        .find(|d| d.id == pseudo_id)
        .and_then(|d| d.trade_weapon_filter)
}
