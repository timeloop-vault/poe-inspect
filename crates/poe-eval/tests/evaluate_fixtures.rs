#![allow(clippy::float_cmp)] // Scores are computed from integer weights; exact comparison is fine.

use std::path::PathBuf;
use std::sync::OnceLock;

use poe_dat::tables::{BaseItemTypeRow, RarityRow};
use poe_data::GameData;
use poe_eval::affix;
use poe_eval::predicate::{
    Cmp, InfluenceValue, ModSlotKind, Predicate, RarityValue, StatCondition, StatusValue,
};
use poe_eval::profile::{Profile, ScoringRule};
use poe_eval::rule::Rule;
use poe_eval::tier;
use poe_eval::{Modifiability, TierQuality};
use poe_eval::{evaluate, score};
use poe_item::types::ResolvedItem;

fn fixture(name: &str) -> String {
    let path = format!("{}/../../fixtures/items/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

/// Load full game data (with reverse index for `stat_id` resolution).
/// Cached via `OnceLock` so it's loaded at most once per test run.
fn full_game_data() -> &'static GameData {
    static GD: OnceLock<GameData> = OnceLock::new();
    GD.get_or_init(|| {
        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/poe-data/data");
        poe_data::load(&data_dir).expect("full game data required — run poe-data extraction first")
    })
}

fn resolve_full(name: &str) -> ResolvedItem {
    let raw = poe_item::parse(&fixture(name)).unwrap();
    poe_item::resolve(&raw, full_game_data())
}

fn test_game_data(base_names: &[&str]) -> GameData {
    let base_item_types: Vec<BaseItemTypeRow> = base_names
        .iter()
        .map(|name| BaseItemTypeRow {
            id: String::new(),
            item_class: None,
            width: 1,
            height: 1,
            name: (*name).to_string(),
            inherits_from: String::new(),
            drop_level: 1,
            implicit_mods: vec![],
            tags: vec![],
        })
        .collect();

    GameData::new(
        vec![],
        vec![],
        vec![],
        vec![],
        base_item_types,
        vec![],
        vec![],
        vec![],
        vec![],
    )
}

fn test_game_data_with_rarities(base_names: &[&str]) -> GameData {
    let base_item_types: Vec<BaseItemTypeRow> = base_names
        .iter()
        .map(|name| BaseItemTypeRow {
            id: String::new(),
            item_class: None,
            width: 1,
            height: 1,
            name: (*name).to_string(),
            inherits_from: String::new(),
            drop_level: 1,
            implicit_mods: vec![],
            tags: vec![],
        })
        .collect();

    let rarities = vec![
        RarityRow {
            id: "Normal".into(),
            min_mods: 0,
            max_mods: 0,
            max_prefix: 0,
            max_suffix: 0,
            text: "Normal".into(),
        },
        RarityRow {
            id: "Magic".into(),
            min_mods: 1,
            max_mods: 2,
            max_prefix: 1,
            max_suffix: 1,
            text: "Magic".into(),
        },
        RarityRow {
            id: "Rare".into(),
            min_mods: 4,
            max_mods: 6,
            max_prefix: 3,
            max_suffix: 3,
            text: "Rare".into(),
        },
        RarityRow {
            id: "Unique".into(),
            min_mods: 0,
            max_mods: 0,
            max_prefix: 0,
            max_suffix: 0,
            text: "Unique".into(),
        },
    ];

    GameData::new(
        vec![],
        vec![],
        vec![],
        vec![],
        base_item_types,
        vec![],
        vec![],
        vec![],
        rarities,
    )
}

fn resolve(name: &str, gd: &GameData) -> ResolvedItem {
    let raw = poe_item::parse(&fixture(name)).unwrap();
    poe_item::resolve(&raw, gd)
}

// ─── Rarity predicates ──────────────────────────────────────────────────────

#[test]
fn rarity_eq() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let rule = Rule::pred(Predicate::Rarity {
        op: Cmp::Eq,
        value: RarityValue::Rare,
    });
    assert!(evaluate(&item, &rule, &gd));

    let rule = Rule::pred(Predicate::Rarity {
        op: Cmp::Eq,
        value: RarityValue::Magic,
    });
    assert!(!evaluate(&item, &rule, &gd));
}

#[test]
fn rarity_ge() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let rule = Rule::pred(Predicate::Rarity {
        op: Cmp::Ge,
        value: RarityValue::Magic,
    });
    assert!(evaluate(&item, &rule, &gd));

    let rule = Rule::pred(Predicate::Rarity {
        op: Cmp::Ge,
        value: RarityValue::Unique,
    });
    assert!(!evaluate(&item, &rule, &gd));
}

// ─── Header predicates ──────────────────────────────────────────────────────

#[test]
fn item_class_match() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let rule = Rule::pred(Predicate::ItemClass {
        op: Cmp::Eq,
        value: "Belts".to_string(),
    });
    assert!(evaluate(&item, &rule, &gd));

    let rule = Rule::pred(Predicate::ItemClass {
        op: Cmp::Eq,
        value: "Rings".to_string(),
    });
    assert!(!evaluate(&item, &rule, &gd));
}

#[test]
fn base_type_match() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let rule = Rule::pred(Predicate::BaseType {
        op: Cmp::Eq,
        value: "Leather Belt".to_string(),
    });
    assert!(evaluate(&item, &rule, &gd));
}

#[test]
fn base_type_contains() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let rule = Rule::pred(Predicate::BaseTypeContains {
        value: "Belt".to_string(),
    });
    assert!(evaluate(&item, &rule, &gd));

    let rule = Rule::pred(Predicate::BaseTypeContains {
        value: "Ring".to_string(),
    });
    assert!(!evaluate(&item, &rule, &gd));
}

// ─── Item level ──────────────────────────────────────────────────────────────

#[test]
fn item_level_comparison() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let rule = Rule::pred(Predicate::ItemLevel {
        op: Cmp::Ge,
        value: 50,
    });
    assert!(evaluate(&item, &rule, &gd));

    let rule = Rule::pred(Predicate::ItemLevel {
        op: Cmp::Gt,
        value: 50,
    });
    assert!(!evaluate(&item, &rule, &gd));
}

// ─── Mod predicates ─────────────────────────────────────────────────────────

#[test]
fn mod_count_prefix() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let rule = Rule::pred(Predicate::ModCount {
        slot: ModSlotKind::Prefix,
        op: Cmp::Ge,
        value: 1,
    });
    assert!(evaluate(&item, &rule, &gd));
}

#[test]
fn has_mod_named() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let rule = Rule::pred(Predicate::HasModNamed {
        name: "Studded".to_string(),
    });
    assert!(evaluate(&item, &rule, &gd));

    let rule = Rule::pred(Predicate::HasModNamed {
        name: "Nonexistent".to_string(),
    });
    assert!(!evaluate(&item, &rule, &gd));
}

#[test]
fn has_stat_presence_via_stat_value() {
    let gd = full_game_data();
    let item = resolve("rare-belt-crafted.txt", gd);

    // Presence check: StatValue with Ge 0 (any non-negative value)
    let rule = Rule::pred(Predicate::StatValue {
        conditions: vec![StatCondition {
            text: Some("maximum Life".to_string()),
            stat_ids: vec!["base_maximum_life".to_string()],
            value_index: 0,
            op: Cmp::Ge,
            value: 0,
        }],
    });
    assert!(evaluate(&item, &rule, gd));
}

// ─── Stat values (require full game data for stat_id resolution) ────────────

#[test]
fn stat_value_check() {
    let gd = full_game_data();
    let item = resolve_full("rare-belt-crafted.txt");

    // Belt has two life mods: implicit +32 and prefix +49.
    // Single condition checks all matching stats — any match satisfies the predicate.
    let rule = Rule::pred(Predicate::StatValue {
        conditions: vec![StatCondition {
            text: None,
            stat_ids: vec!["base_maximum_life".into()],
            value_index: 0,
            op: Cmp::Ge,
            value: 30,
        }],
    });
    assert!(evaluate(&item, &rule, gd));

    // >= 40 should pass because the prefix has +49
    let rule = Rule::pred(Predicate::StatValue {
        conditions: vec![StatCondition {
            text: None,
            stat_ids: vec!["base_maximum_life".into()],
            value_index: 0,
            op: Cmp::Ge,
            value: 40,
        }],
    });
    assert!(evaluate(&item, &rule, gd));

    // >= 55 should fail — neither 32 nor 49 reaches 55
    let rule = Rule::pred(Predicate::StatValue {
        conditions: vec![StatCondition {
            text: None,
            stat_ids: vec!["base_maximum_life".into()],
            value_index: 0,
            op: Cmp::Ge,
            value: 55,
        }],
    });
    assert!(!evaluate(&item, &rule, gd));
}

#[test]
fn stat_value_checks_all_matching_mods() {
    // Bug: find_stat_value returned on the first matching stat line.
    // This item has two "+# to maximum Life" mods: +24 (hybrid T3) and +139 (pure T4).
    // StatValue > 100 should match because the 139 exceeds the threshold,
    // even though the 24 (which appears first) does not.
    let gd = full_game_data();
    let item = resolve_full("rare-body-armour-craft-hybrid-and-normal-life-mod.txt");

    let rule = Rule::pred(Predicate::StatValue {
        conditions: vec![StatCondition {
            text: None,
            stat_ids: vec!["base_maximum_life".into()],
            value_index: 0,
            op: Cmp::Gt,
            value: 100,
        }],
    });
    assert!(evaluate(&item, &rule, gd));

    // Neither +24 nor +139 exceeds 150
    let rule_high = Rule::pred(Predicate::StatValue {
        conditions: vec![StatCondition {
            text: None,
            stat_ids: vec!["base_maximum_life".into()],
            value_index: 0,
            op: Cmp::Gt,
            value: 150,
        }],
    });
    assert!(!evaluate(&item, &rule_high, gd));
}

#[test]
fn roll_percent_check() {
    let gd = full_game_data();
    let item = resolve_full("rare-belt-crafted.txt");

    // Belt has implicit +32(25-40) and prefix +49(40-54).
    // Implicit: 32 is 46% of 25..40 range.
    // Prefix: 49 is 64% of 40..54 range.
    // Any match: >= 40 should pass (both exceed 40%).
    let rule = Rule::pred(Predicate::RollPercent {
        text: None,
        stat_ids: vec!["base_maximum_life".to_string()],
        value_index: 0,
        op: Cmp::Ge,
        value: 40,
    });
    assert!(evaluate(&item, &rule, gd));

    // >= 90 should fail — neither roll is that high
    let rule = Rule::pred(Predicate::RollPercent {
        text: None,
        stat_ids: vec!["base_maximum_life".to_string()],
        value_index: 0,
        op: Cmp::Ge,
        value: 90,
    });
    assert!(!evaluate(&item, &rule, gd));
}

// ─── Influence / status ─────────────────────────────────────────────────────

#[test]
fn has_influence() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-boots-eater-exarch.txt", &gd);

    let rule = Rule::pred(Predicate::HasInfluence {
        influence: InfluenceValue::SearingExarch,
    });
    assert!(evaluate(&item, &rule, &gd));
}

#[test]
fn has_corrupted_status() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-map-city-square-delirium.txt", &gd);

    let rule = Rule::pred(Predicate::HasStatus {
        status: StatusValue::Corrupted,
    });
    assert!(evaluate(&item, &rule, &gd));
}

// ─── Rule combinators ───────────────────────────────────────────────────────

#[test]
fn all_combinator() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let rule = Rule::all(vec![
        Rule::pred(Predicate::Rarity {
            op: Cmp::Eq,
            value: RarityValue::Rare,
        }),
        Rule::pred(Predicate::ItemClass {
            op: Cmp::Eq,
            value: "Belts".to_string(),
        }),
        Rule::pred(Predicate::ItemLevel {
            op: Cmp::Ge,
            value: 50,
        }),
    ]);
    assert!(evaluate(&item, &rule, &gd));
}

#[test]
fn any_combinator() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let rule = Rule::any(vec![
        Rule::pred(Predicate::ItemClass {
            op: Cmp::Eq,
            value: "Rings".to_string(),
        }),
        Rule::pred(Predicate::ItemClass {
            op: Cmp::Eq,
            value: "Belts".to_string(),
        }),
    ]);
    assert!(evaluate(&item, &rule, &gd));
}

#[test]
fn not_combinator() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let rule = Rule::negate(Rule::pred(Predicate::HasStatus {
        status: StatusValue::Corrupted,
    }));
    assert!(evaluate(&item, &rule, &gd));
}

#[test]
fn complex_rule() {
    let gd = full_game_data();
    let item = resolve("rare-belt-crafted.txt", gd);

    // "Is a rare belt with ilvl >= 50 AND has life AND is not corrupted"
    let rule = Rule::all(vec![
        Rule::pred(Predicate::Rarity {
            op: Cmp::Eq,
            value: RarityValue::Rare,
        }),
        Rule::pred(Predicate::BaseTypeContains {
            value: "Belt".to_string(),
        }),
        Rule::pred(Predicate::ItemLevel {
            op: Cmp::Ge,
            value: 50,
        }),
        Rule::pred(Predicate::StatValue {
            conditions: vec![StatCondition {
                text: Some("maximum Life".to_string()),
                stat_ids: vec!["base_maximum_life".to_string()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            }],
        }),
        Rule::negate(Rule::pred(Predicate::HasStatus {
            status: StatusValue::Corrupted,
        })),
    ]);
    assert!(evaluate(&item, &rule, gd));
}

// ─── Open mods (requires rarity data) ──────────────────────────────────────

#[test]
fn open_mods_without_game_data() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    // Without rarity data, open mods should be 0 (can't determine max)
    let rule = Rule::pred(Predicate::OpenMods {
        slot: ModSlotKind::Prefix,
        op: Cmp::Ge,
        value: 1,
    });
    assert!(!evaluate(&item, &rule, &gd));
}

// ─── Scoring profiles ───────────────────────────────────────────────────────

fn belt_profile() -> Profile {
    Profile {
        name: "Life Belt".to_string(),
        description: "Scores belts for life-based builds".to_string(),
        filter: Some(Rule::all(vec![
            Rule::pred(Predicate::Rarity {
                op: Cmp::Eq,
                value: RarityValue::Rare,
            }),
            Rule::pred(Predicate::BaseTypeContains {
                value: "Belt".to_string(),
            }),
        ])),
        scoring: vec![
            ScoringRule {
                label: "Has life".to_string(),
                weight: 10.0,
                rule: Rule::pred(Predicate::StatValue {
                    conditions: vec![StatCondition {
                        text: Some("maximum Life".to_string()),
                        stat_ids: vec!["base_maximum_life".to_string()],
                        value_index: 0,
                        op: Cmp::Ge,
                        value: 0,
                    }],
                }),
            },
            ScoringRule {
                label: "Has resistances".to_string(),
                weight: 5.0,
                rule: Rule::any(vec![
                    Rule::pred(Predicate::StatValue {
                        conditions: vec![StatCondition {
                            text: Some("Fire Resistance".to_string()),
                            stat_ids: vec!["base_fire_damage_resistance_%".to_string()],
                            value_index: 0,
                            op: Cmp::Ge,
                            value: 0,
                        }],
                    }),
                    Rule::pred(Predicate::StatValue {
                        conditions: vec![StatCondition {
                            text: Some("Cold Resistance".to_string()),
                            stat_ids: vec!["base_cold_damage_resistance_%".to_string()],
                            value_index: 0,
                            op: Cmp::Ge,
                            value: 0,
                        }],
                    }),
                    Rule::pred(Predicate::StatValue {
                        conditions: vec![StatCondition {
                            text: Some("Lightning Resistance".to_string()),
                            stat_ids: vec!["base_lightning_damage_resistance_%".to_string()],
                            value_index: 0,
                            op: Cmp::Ge,
                            value: 0,
                        }],
                    }),
                ]),
            },
            ScoringRule {
                label: "Has armour".to_string(),
                weight: 3.0,
                rule: Rule::pred(Predicate::StatValue {
                    conditions: vec![StatCondition {
                        text: Some("Armour".to_string()),
                        stat_ids: vec!["base_physical_damage_reduction_rating".to_string()],
                        value_index: 0,
                        op: Cmp::Ge,
                        value: 0,
                    }],
                }),
            },
            ScoringRule {
                label: "Not corrupted".to_string(),
                weight: 2.0,
                rule: Rule::negate(Rule::pred(Predicate::HasStatus {
                    status: StatusValue::Corrupted,
                })),
            },
        ],
    }
}

#[test]
fn score_matching_profile() {
    let gd = full_game_data();
    let item = resolve("rare-belt-crafted.txt", gd);

    let result = score(&item, &belt_profile(), gd);

    assert!(result.applicable);
    assert!(result.score > 0.0);
    // Belt has life, resistances, armour, and is not corrupted
    assert!(result.matched.len() >= 3);
}

#[test]
fn score_filter_rejects() {
    let gd = full_game_data();
    let item = resolve("unique-ring-ventors-gamble.txt", gd);

    let result = score(&item, &belt_profile(), gd);

    // Ring should not match belt profile filter
    assert!(!result.applicable);
    assert_eq!(result.score, 0.0);
    assert!(result.matched.is_empty());
}

#[test]
fn score_detailed_breakdown() {
    let gd = full_game_data();
    let item = resolve("rare-belt-crafted.txt", gd);

    let result = score(&item, &belt_profile(), gd);

    // Check that we get labels back
    assert!(result.applicable);
    let labels: Vec<&str> = result.matched.iter().map(|m| m.label.as_str()).collect();
    assert!(labels.contains(&"Has life"));
    assert!(labels.contains(&"Not corrupted"));

    // Score should be sum of matched weights
    let expected_score: f64 = result.matched.iter().map(|m| m.weight).sum();
    assert!((result.score - expected_score).abs() < f64::EPSILON);
}

#[test]
fn profile_no_filter() {
    let gd = full_game_data();
    let item = resolve("unique-ring-ventors-gamble.txt", gd);

    // Profile with no filter — applies to everything
    let profile = Profile {
        name: "Has Life".to_string(),
        description: String::new(),
        filter: None,
        scoring: vec![ScoringRule {
            label: "life".to_string(),
            weight: 10.0,
            rule: Rule::pred(Predicate::StatValue {
                conditions: vec![StatCondition {
                    text: Some("maximum Life".to_string()),
                    stat_ids: vec!["base_maximum_life".to_string()],
                    value_index: 0,
                    op: Cmp::Ge,
                    value: 0,
                }],
            }),
        }],
    };

    let result = score(&item, &profile, gd);
    assert!(result.applicable);
    assert_eq!(result.score, 10.0);
}

// ─── Tier analysis ──────────────────────────────────────────────────────────

#[test]
fn tier_analysis_belt() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let summary = tier::analyze_tiers(&item, &gd);

    // Belt has 4 explicit mods: T7, T7, T6, T5 — all mid/low
    // Plus 1 implicit (no tier) + 1 crafted (no tier)
    assert!(summary.mods.len() >= 4);

    // Worst = Low (T7), best = Mid (T5)
    assert_eq!(summary.worst_explicit, TierQuality::Low);
    assert_eq!(summary.best_explicit, TierQuality::Mid);
    assert!(summary.quality_counts.low >= 2); // Two T7 mods
}

#[test]
fn tier_analysis_mixed_tiers() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-axe-fractured.txt", &gd);

    let summary = tier::analyze_tiers(&item, &gd);

    // Has T1, T1, T2, T4, T5 — best is Best (T1)
    assert_eq!(summary.best_explicit, TierQuality::Best);
    assert!(summary.quality_counts.best >= 2); // Two T1 mods
}

#[test]
fn tier_analysis_unique_has_no_tiers() {
    let gd = test_game_data(&[]);
    let item = resolve("unique-ring-ventors-gamble.txt", &gd);

    let summary = tier::analyze_tiers(&item, &gd);

    // Unique mods have no tiers — all Unknown
    assert_eq!(summary.worst_explicit, TierQuality::Unknown);
    assert_eq!(summary.best_explicit, TierQuality::Unknown);
}

#[test]
fn profile_serializes_to_json() {
    let profile = belt_profile();
    let json = serde_json::to_string_pretty(&profile).unwrap();
    assert!(json.contains("Life Belt"));
    assert!(json.contains("maximum Life"));

    // Round-trip
    let deserialized: Profile = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "Life Belt");
    assert_eq!(deserialized.scoring.len(), 4);
}

// ─── Affix analysis ────────────────────────────────────────────────────────

#[test]
fn affix_rare_belt_with_rarity_data() {
    // Belt has 2 prefixes, 3 suffixes (one crafted). Rare max = 3/3.
    let gd = test_game_data_with_rarities(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let summary = affix::analyze_affixes(&item, &gd);

    assert_eq!(summary.modifiable, Modifiability::Yes);
    assert_eq!(summary.prefixes.used, 2);
    assert_eq!(summary.prefixes.max, Some(3));
    assert_eq!(summary.prefixes.open, Some(1));
    assert!(!summary.prefixes.has_crafted);

    assert_eq!(summary.suffixes.used, 3);
    assert_eq!(summary.suffixes.max, Some(3));
    assert_eq!(summary.suffixes.open, Some(0));
    assert!(summary.suffixes.has_crafted); // "of Craft" is master crafted
}

#[test]
fn affix_rare_belt_without_rarity_data() {
    // Without rarity data, max/open should be None
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let summary = affix::analyze_affixes(&item, &gd);

    assert_eq!(summary.prefixes.used, 2);
    assert_eq!(summary.prefixes.max, None);
    assert_eq!(summary.prefixes.open, None);
}

#[test]
fn affix_corrupted_item() {
    let gd = test_game_data_with_rarities(&[]);
    let item = resolve("rare-amulet-talisman-corrupted.txt", &gd);

    let summary = affix::analyze_affixes(&item, &gd);

    assert_eq!(summary.modifiable, Modifiability::Corrupted);
    // Still reports counts even though unmodifiable
    assert_eq!(summary.prefixes.used, 3);
    assert_eq!(summary.suffixes.used, 3);
}

#[test]
fn affix_unique_not_applicable() {
    let gd = test_game_data_with_rarities(&[]);
    let item = resolve("unique-ring-ventors-gamble.txt", &gd);

    let summary = affix::analyze_affixes(&item, &gd);

    // Unique items don't have standard prefix/suffix slots
    assert_eq!(summary.prefixes.max, Some(0));
    assert_eq!(summary.suffixes.max, Some(0));
}

#[test]
fn affix_crafted_suffix_detected() {
    let gd = test_game_data_with_rarities(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let summary = affix::analyze_affixes(&item, &gd);

    // Suffixes has a bench craft — removing it would open a slot
    assert!(summary.suffixes.has_crafted);
    assert!(!summary.prefixes.has_crafted);
}

// ─── Per-item-class affix limits ─────────────────────────────────────────────

/// Rare jewels have max 2 prefix / 2 suffix (not the global 3/3).
/// A full rare cobalt jewel (2p + 2s) should have 0 open slots.
#[test]
fn affix_rare_jewel_max_2_prefix_2_suffix() {
    let gd = test_game_data_with_rarities(&[]);
    let item = resolve("rare-jewel-cobalt-mirrored-corrupted.txt", &gd);

    let summary = affix::analyze_affixes(&item, &gd);

    // Jewel-specific limit: 2/2 for Rare
    assert_eq!(summary.prefixes.max, Some(2));
    assert_eq!(summary.suffixes.max, Some(2));
    assert_eq!(summary.prefixes.used, 2);
    assert_eq!(summary.suffixes.used, 2);
    assert_eq!(summary.prefixes.open, Some(0));
    assert_eq!(summary.suffixes.open, Some(0));
}

/// Rare abyss jewels also have max 2 prefix / 2 suffix.
#[test]
fn affix_rare_abyss_jewel_max_2_prefix_2_suffix() {
    let gd = test_game_data_with_rarities(&[]);
    let item = resolve("rare-abyss-jewel-ghastly-eye.txt", &gd);

    let summary = affix::analyze_affixes(&item, &gd);

    // Abyss jewel: same 2/2 limit as regular jewels
    assert_eq!(summary.prefixes.max, Some(2));
    assert_eq!(summary.suffixes.max, Some(2));
    assert_eq!(summary.prefixes.used, 2);
    assert_eq!(summary.suffixes.used, 2);
    assert_eq!(summary.prefixes.open, Some(0));
    assert_eq!(summary.suffixes.open, Some(0));
}

/// `OpenMods` predicate respects jewel-specific limits.
/// A full rare jewel (2p + 2s) should have 0 open affixes.
#[test]
fn open_mods_respects_jewel_limits() {
    let gd = test_game_data_with_rarities(&[]);
    let item = resolve("rare-abyss-jewel-ghastly-eye.txt", &gd);

    // Should have 0 open affixes (2p + 2s = full)
    let rule_open = Rule::pred(Predicate::OpenMods {
        slot: ModSlotKind::Affix,
        op: Cmp::Ge,
        value: 1,
    });
    assert!(!evaluate(&item, &rule_open, &gd));

    // Verify it would have matched with old 3/3 logic (2 open slots)
    // by checking the count is exactly 0
    let rule_zero = Rule::pred(Predicate::OpenMods {
        slot: ModSlotKind::Affix,
        op: Cmp::Eq,
        value: 0,
    });
    assert!(evaluate(&item, &rule_zero, &gd));
}

/// Equipment still uses global rarity limits (3/3 for Rare).
#[test]
fn affix_equipment_still_uses_rarity_defaults() {
    let gd = test_game_data_with_rarities(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let summary = affix::analyze_affixes(&item, &gd);

    // Belt uses default Rare limits: 3/3
    assert_eq!(summary.prefixes.max, Some(3));
    assert_eq!(summary.suffixes.max, Some(3));
}

// ─── StatValue multi-condition (same-mod check) ─────────────────────────────

// Body armour fixture mod layout (used by many tests below):
// - "Urchin's" (hybrid): local_base_physical_damage_reduction_rating + base_maximum_life
// - "Carapaced" (pure armour): local_base_physical_damage_reduction_rating
// - "Fecund" (pure life): base_maximum_life
// - "of the Ice": base_cold_damage_resistance_%
// - "of the Volcano": base_fire_damage_resistance_%

/// Multi-condition `StatValue` matches when ALL conditions are on a SINGLE mod.
#[test]
fn stat_value_multi_matches_same_mod() {
    let gd = full_game_data();
    let item = resolve_full("rare-body-armour-craft-hybrid-and-normal-life-mod.txt");

    // armour + life on the SAME mod → should match Urchin's
    let rule = Rule::pred(Predicate::StatValue {
        conditions: vec![
            StatCondition {
                text: Some("+# to Armour".into()),
                stat_ids: vec!["local_base_physical_damage_reduction_rating".into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            },
            StatCondition {
                text: Some("+# to maximum Life".into()),
                stat_ids: vec!["base_maximum_life".into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            },
        ],
    });
    assert!(
        evaluate(&item, &rule, gd),
        "multi-condition should match Urchin's (armour + life on same mod)"
    );
}

/// Multi-condition must NOT match when stats exist on DIFFERENT mods.
#[test]
fn stat_value_multi_does_not_match_across_mods() {
    let gd = full_game_data();
    let item = resolve_full("rare-helmet-crafted.txt");

    // Helmet has life (Rotund) but no armour mod. Multi(armour + life) → false.
    let rule = Rule::pred(Predicate::StatValue {
        conditions: vec![
            StatCondition {
                text: None,
                stat_ids: vec!["local_base_physical_damage_reduction_rating".into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            },
            StatCondition {
                text: None,
                stat_ids: vec!["base_maximum_life".into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            },
        ],
    });
    assert!(
        !evaluate(&item, &rule, gd),
        "multi-condition should not match when stats are on different mods"
    );
}

/// Item with BOTH stats on SEPARATE mods does NOT trigger multi-condition,
/// even though separate single-condition checks (via `Rule::All`) would match.
#[test]
fn stat_value_multi_rejects_cross_mod_on_shield() {
    let gd = full_game_data();
    let item = resolve_full("rare-shield-crafted.txt");

    // Shield has armour (Carapaced) and life (Virile) on SEPARATE mods.
    let rule = Rule::pred(Predicate::StatValue {
        conditions: vec![
            StatCondition {
                text: None,
                stat_ids: vec!["local_base_physical_damage_reduction_rating".into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            },
            StatCondition {
                text: None,
                stat_ids: vec!["base_maximum_life".into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            },
        ],
    });
    assert!(
        !evaluate(&item, &rule, gd),
        "multi-condition must not match when armour and life are on separate mods"
    );

    // Contrast: Rule::All with two single-condition StatValues WOULD match.
    let rule_all = Rule::all(vec![
        Rule::pred(Predicate::StatValue {
            conditions: vec![StatCondition {
                text: None,
                stat_ids: vec!["local_base_physical_damage_reduction_rating".into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            }],
        }),
        Rule::pred(Predicate::StatValue {
            conditions: vec![StatCondition {
                text: None,
                stat_ids: vec!["base_maximum_life".into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            }],
        }),
    ]);
    assert!(
        evaluate(&item, &rule_all, gd),
        "Rule::All should match when stats exist on different mods"
    );
}

/// Multi-condition fails when one stat is completely absent from the item.
#[test]
fn stat_value_multi_fails_when_stat_missing() {
    let gd = full_game_data();
    let item = resolve_full("rare-body-armour-craft-hybrid-and-normal-life-mod.txt");

    // body armour has no lightning resistance
    let rule = Rule::pred(Predicate::StatValue {
        conditions: vec![
            StatCondition {
                text: None,
                stat_ids: vec!["base_maximum_life".into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            },
            StatCondition {
                text: None,
                stat_ids: vec!["base_lightning_damage_resistance_%".into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            },
        ],
    });
    assert!(
        !evaluate(&item, &rule, gd),
        "multi-condition should fail when one stat is absent"
    );
}

/// Empty conditions returns false (no vacuous truth).
#[test]
fn stat_value_empty_conditions_does_not_match() {
    let gd = full_game_data();
    let item = resolve_full("rare-body-armour-craft-hybrid-and-normal-life-mod.txt");

    let rule = Rule::pred(Predicate::StatValue { conditions: vec![] });
    assert!(
        !evaluate(&item, &rule, gd),
        "empty conditions should not match"
    );
}

/// Multi-condition with value thresholds on the same mod.
#[test]
fn stat_value_multi_with_value_thresholds() {
    let gd = full_game_data();
    let item = resolve_full("rare-body-armour-craft-hybrid-and-normal-life-mod.txt");

    let armour_id = "local_base_physical_damage_reduction_rating";
    let life_id = "base_maximum_life";

    // Urchin's has armour + life on same mod.
    // With low thresholds → should match
    let rule = Rule::pred(Predicate::StatValue {
        conditions: vec![
            StatCondition {
                text: None,
                stat_ids: vec![armour_id.into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 1,
            },
            StatCondition {
                text: None,
                stat_ids: vec![life_id.into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 1,
            },
        ],
    });
    assert!(evaluate(&item, &rule, gd));

    // With impossibly high threshold on life → should fail (Urchin's life is low)
    let rule_high = Rule::pred(Predicate::StatValue {
        conditions: vec![
            StatCondition {
                text: None,
                stat_ids: vec![armour_id.into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 1,
            },
            StatCondition {
                text: None,
                stat_ids: vec![life_id.into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 200,
            },
        ],
    });
    assert!(!evaluate(&item, &rule_high, gd));
}

/// Contrast: multi-condition vs `Rule::All` on same item.
#[test]
fn stat_value_multi_vs_rule_all_on_body_armour() {
    let gd = full_game_data();
    let item = resolve_full("rare-body-armour-craft-hybrid-and-normal-life-mod.txt");

    let armour_id = "local_base_physical_damage_reduction_rating";
    let life_id = "base_maximum_life";
    let cold_res_id = "base_cold_damage_resistance_%";

    // Multi-condition: armour + life → matches Urchin's (same mod)
    let multi_armour_life = Rule::pred(Predicate::StatValue {
        conditions: vec![
            StatCondition {
                text: None,
                stat_ids: vec![armour_id.into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            },
            StatCondition {
                text: None,
                stat_ids: vec![life_id.into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            },
        ],
    });
    assert!(evaluate(&item, &multi_armour_life, gd));

    // Multi-condition: life + cold res → NO mod has both
    let multi_life_cold = Rule::pred(Predicate::StatValue {
        conditions: vec![
            StatCondition {
                text: None,
                stat_ids: vec![life_id.into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            },
            StatCondition {
                text: None,
                stat_ids: vec![cold_res_id.into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            },
        ],
    });
    assert!(!evaluate(&item, &multi_life_cold, gd));

    // Rule::All: life + cold res → matches across mods
    let all_life_cold = Rule::all(vec![
        Rule::pred(Predicate::StatValue {
            conditions: vec![StatCondition {
                text: None,
                stat_ids: vec![life_id.into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            }],
        }),
        Rule::pred(Predicate::StatValue {
            conditions: vec![StatCondition {
                text: None,
                stat_ids: vec![cold_res_id.into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 0,
            }],
        }),
    ]);
    assert!(evaluate(&item, &all_life_cold, gd));
}

/// Multi-condition in scoring profile — weighted contribution.
#[test]
fn stat_value_multi_in_scoring_profile() {
    let gd = full_game_data();
    let body_armour = resolve_full("rare-body-armour-craft-hybrid-and-normal-life-mod.txt");
    let shield = resolve_full("rare-shield-crafted.txt");

    let profile = Profile {
        name: "Hybrid Armour+Life".into(),
        description: "Rewards items with hybrid armour+life mods".into(),
        filter: None,
        scoring: vec![ScoringRule {
            label: "Hybrid armour + life".into(),
            weight: 20.0,
            rule: Rule::pred(Predicate::StatValue {
                conditions: vec![
                    StatCondition {
                        text: Some("+# to Armour".into()),
                        stat_ids: vec!["local_base_physical_damage_reduction_rating".into()],
                        value_index: 0,
                        op: Cmp::Ge,
                        value: 0,
                    },
                    StatCondition {
                        text: Some("+# to maximum Life".into()),
                        stat_ids: vec!["base_maximum_life".into()],
                        value_index: 0,
                        op: Cmp::Ge,
                        value: 0,
                    },
                ],
            }),
        }],
    };

    // Body armour has Urchin's (hybrid) → scores 20
    let result = score(&body_armour, &profile, gd);
    assert!(result.applicable);
    assert_eq!(
        result.score, 20.0,
        "body armour with hybrid mod should score 20"
    );
    assert_eq!(result.matched.len(), 1);

    // Shield has armour + life on SEPARATE mods → scores 0
    let result = score(&shield, &profile, gd);
    assert!(result.applicable);
    assert_eq!(
        result.score, 0.0,
        "shield without hybrid mod should score 0"
    );
    assert_eq!(result.matched.len(), 0);
}

/// `StatValue` serialization round-trip with conditions.
#[test]
fn stat_value_conditions_serialize_roundtrip() {
    let pred = Predicate::StatValue {
        conditions: vec![
            StatCondition {
                text: Some("+# to Armour".into()),
                stat_ids: vec!["local_base_physical_damage_reduction_rating".into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 50,
            },
            StatCondition {
                text: Some("+# to maximum Life".into()),
                stat_ids: vec!["base_maximum_life".into()],
                value_index: 0,
                op: Cmp::Ge,
                value: 20,
            },
        ],
    };

    let json = serde_json::to_string(&pred).unwrap();
    assert!(json.contains("\"StatValue\""));
    assert!(json.contains("\"conditions\""));
    assert!(json.contains("\"stat_ids\""));
    assert!(json.contains("local_base_physical_damage_reduction_rating"));
    assert!(json.contains("base_maximum_life"));

    let deserialized: Predicate = serde_json::from_str(&json).unwrap();
    match deserialized {
        Predicate::StatValue { conditions } => {
            assert_eq!(conditions.len(), 2);
            assert_eq!(
                conditions[0].stat_ids,
                vec!["local_base_physical_damage_reduction_rating"]
            );
            assert_eq!(conditions[1].value, 20);
        }
        _ => panic!("expected StatValue variant"),
    }
}

/// Fractured mod source prefix parses correctly.
#[test]
fn fractured_mod_source_parses() {
    let item = resolve_full("rare-helmet-fractured-rarity.txt");

    assert_eq!(item.explicits.len(), 4);
    assert!(
        item.explicits
            .iter()
            .any(|m| m.header.name.as_deref() == Some("Dragon's"))
    );
}

/// Unique item unscalable mods resolve `stat_ids` after stripping "— Unscalable Value" suffix.
#[test]
fn unique_unscalable_mods_resolve_stat_ids() {
    let item = resolve_full("unique-body-armour-doryanis-prototype.txt");

    assert_eq!(item.explicits.len(), 6);

    // Standard stats resolve normally
    let armour_es = &item.explicits[0].stat_lines[0];
    assert!(
        armour_es
            .stat_ids
            .as_ref()
            .unwrap()
            .iter()
            .any(|s| s == "local_armour_and_energy_shield_+%")
    );
    assert!(!armour_es.is_unscalable);

    // Unscalable stats: suffix stripped, stat_ids resolved, flag set
    let no_lightning = &item.explicits[2].stat_lines[0];
    assert!(
        no_lightning
            .stat_ids
            .as_ref()
            .unwrap()
            .iter()
            .any(|s| s == "deal_no_non_lightning_damage")
    );
    assert!(no_lightning.is_unscalable);
    assert_eq!(no_lightning.display_text, "Deal no Non-Lightning Damage");

    let armour_lightning = &item.explicits[3].stat_lines[0];
    assert!(armour_lightning.is_unscalable);
    assert!(armour_lightning.stat_ids.is_some());

    let resist_no_apply = &item.explicits[4].stat_lines[0];
    assert!(resist_no_apply.is_unscalable);
    assert!(resist_no_apply.stat_ids.is_some());

    let nearby = &item.explicits[5].stat_lines[0];
    assert!(nearby.is_unscalable);
    assert!(nearby.stat_ids.is_some());
}

/// Debug: trace `stat_ids` through the full pipeline for a body armour with hybrid mods.
#[test]
fn trace_stat_ids_body_armour() {
    let gd = full_game_data();
    let item = resolve_full("rare-body-armour-craft-hybrid-and-normal-life-mod.txt");

    println!("\n=== RESOLVED ITEM STAT IDS ===");
    for m in item.implicits.iter().chain(item.explicits.iter()) {
        println!("\nMod: {:?} ({:?})", m.header.name, m.header.slot);
        for sl in &m.stat_lines {
            if sl.is_reminder {
                continue;
            }
            println!("  display: {}", sl.display_text);
            println!("  stat_ids: {:?}", sl.stat_ids);
        }
    }

    // Now trace what suggestions would provide
    let suggestions = gd.stat_suggestions_for_query("to Armour");
    let armour_single = suggestions.iter().find(|s| {
        matches!(s.kind, poe_data::StatSuggestionKind::Single)
            && s.stat_ids
                .iter()
                .any(|id| id.contains("physical_damage_reduction"))
    });
    println!("\n=== ARMOUR SINGLE SUGGESTION ===");
    if let Some(s) = armour_single {
        println!("  template: {}", s.template);
        println!("  stat_ids: {:?}", s.stat_ids);
    } else {
        println!("  NOT FOUND");
    }

    let life_suggestions = gd.stat_suggestions_for_query("to maximum Life");
    let life_single = life_suggestions.iter().find(|s| {
        matches!(s.kind, poe_data::StatSuggestionKind::Single)
            && s.stat_ids.iter().any(|id| id == "base_maximum_life")
    });
    println!("\n=== LIFE SINGLE SUGGESTION ===");
    if let Some(s) = life_single {
        println!("  template: {}", s.template);
        println!("  stat_ids: {:?}", s.stat_ids);
    }

    let life_hybrids: Vec<_> = life_suggestions
        .iter()
        .filter(|s| matches!(&s.kind, poe_data::StatSuggestionKind::Hybrid { .. }))
        .collect();
    println!("\n=== LIFE HYBRID SUGGESTIONS ===");
    for h in &life_hybrids {
        if let poe_data::StatSuggestionKind::Hybrid {
            mod_name,
            other_stat_ids,
            other_templates,
            ..
        } = &h.kind
        {
            println!("  mod: {mod_name}");
            println!("  primary stat_ids: {:?}", h.stat_ids);
            println!("  other_stat_ids: {other_stat_ids:?}");
            println!("  other_templates: {other_templates:?}");
        }
    }
}

/// Map mods must resolve to map-specific `stat_ids` from `map_stat_descriptions`.
/// Verifies that the merged reverse index includes map mod patterns.
#[test]
fn map_mods_resolve_stat_ids() {
    let item = resolve_full("rare-map-abomination-t17.txt");

    let all_stat_ids: Vec<_> = item
        .explicits
        .iter()
        .flat_map(|m| &m.stat_lines)
        .filter(|sl| !sl.is_reminder)
        .filter_map(|sl| sl.stat_ids.as_ref())
        .flatten()
        .collect();

    println!("Map mod stat_ids:");
    for m in &item.explicits {
        println!(
            "  {:?}: {:?}",
            m.header.name,
            m.stat_lines
                .iter()
                .filter(|sl| !sl.is_reminder)
                .map(|sl| (&sl.display_text, &sl.stat_ids))
                .collect::<Vec<_>>()
        );
    }

    // At least some map mods should have stat_ids starting with "map_"
    let map_stat_count = all_stat_ids
        .iter()
        .filter(|id| id.starts_with("map_"))
        .count();
    assert!(
        map_stat_count >= 3,
        "expected at least 3 map-specific stat_ids, got {map_stat_count} (all: {all_stat_ids:?})"
    );
}

/// Abyss jewel mods must resolve to non-local `stat_ids`.
/// "Vaporous" (+# to Evasion Rating) on a jewel is `base_evasion_rating`,
/// NOT `local_base_evasion_rating` (which is for armour items).
#[test]
fn abyss_jewel_resolves_non_local_stat_ids() {
    let item = resolve_full("rare-abyss-jewel-searching-eye.txt");

    // "Vaporous" — flat evasion on jewel must be non-local
    let vaporous = item
        .explicits
        .iter()
        .find(|m| m.header.name.as_deref() == Some("Vaporous"))
        .expect("Vaporous mod should exist");
    let evasion_stat = &vaporous.stat_lines[0];
    assert_eq!(
        evasion_stat.stat_ids.as_ref().unwrap(),
        &["base_evasion_rating"],
        "Jewel evasion should be non-local (base_evasion_rating)"
    );

    // "of the Ranger" — accuracy on jewel must be non-local
    let ranger = item
        .explicits
        .iter()
        .find(|m| m.header.name.as_deref() == Some("of the Ranger"))
        .expect("of the Ranger mod should exist");
    let accuracy_stat = &ranger.stat_lines[0];
    assert_eq!(
        accuracy_stat.stat_ids.as_ref().unwrap(),
        &["accuracy_rating"],
        "Jewel accuracy should be non-local (accuracy_rating)"
    );
}

// ── Pseudo stat tests ───────────────────────────────────────────────────────

#[test]
fn pseudo_physical_damage_computed() {
    let item = resolve_full("rare-warstaff-sol-pile.txt");

    // Item has two physical damage mods: 44% + 19% = 63%
    let phys_pseudo = item.pseudo_mods.iter().find(|m| {
        m.stat_lines.iter().any(|sl| {
            sl.stat_ids.as_ref().is_some_and(|ids| {
                ids.iter()
                    .any(|id| id == "pseudo_increased_physical_damage")
            })
        })
    });

    assert!(
        phys_pseudo.is_some(),
        "should have pseudo_increased_physical_damage mod. All explicits: {:?}",
        item.explicits
            .iter()
            .flat_map(|m| m.stat_lines.iter())
            .map(|sl| format!("{} → {:?}", sl.display_text, sl.stat_ids))
            .collect::<Vec<_>>()
    );

    let value = phys_pseudo.unwrap().stat_lines[0].values[0].current;
    assert_eq!(value, 63, "44% + 19% = 63% total physical damage");
}

#[test]
fn pseudo_stat_value_predicate_works() {
    use poe_eval::predicate::{Cmp, Predicate, StatCondition};
    use poe_eval::rule::Rule;

    let item = resolve_full("rare-warstaff-sol-pile.txt");

    // StatValue with pseudo stat_id should match via all_mods()
    let rule = Rule::Pred(Predicate::StatValue {
        conditions: vec![StatCondition {
            text: Some("(Pseudo) #% total increased Physical Damage".to_string()),
            stat_ids: vec!["pseudo_increased_physical_damage".to_string()],
            value_index: 0,
            op: Cmp::Ge,
            value: 60,
        }],
    });

    let result = poe_eval::evaluate(&item, &rule, full_game_data());
    assert!(result, "pseudo physical damage 63 >= 60 should match");

    // Should NOT match with higher threshold
    let rule_high = Rule::Pred(Predicate::StatValue {
        conditions: vec![StatCondition {
            text: Some("(Pseudo) #% total increased Physical Damage".to_string()),
            stat_ids: vec!["pseudo_increased_physical_damage".to_string()],
            value_index: 0,
            op: Cmp::Ge,
            value: 100,
        }],
    });

    let result_high = poe_eval::evaluate(&item, &rule_high, full_game_data());
    assert!(
        !result_high,
        "pseudo physical damage 63 < 100 should not match"
    );
}

// ─── StatTier + TierCount predicates ────────────────────────────────────────

#[test]
fn stat_tier_matches_explicit_mod() {
    use poe_eval::predicate::TierKindFilter;

    let gd = full_game_data();
    // rare-warstaff has "Heavy" (Tier 8) and "Squire's" (Tier 8) phys damage,
    // "Cerulean" (Tier 8) mana, "of Puncturing" (Tier 3) crit chance.
    let item = resolve_full("rare-warstaff-sol-pile.txt");

    // Tier <= 8 should match the phys damage mod
    let rule = Rule::pred(Predicate::StatTier {
        text: None,
        stat_ids: vec!["local_physical_damage_+%".into()],
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 8,
        source: None,
    });
    assert!(
        evaluate(&item, &rule, gd),
        "T8 phys mod should match tier <= 8"
    );

    // Tier <= 2 should NOT match (phys is T8)
    let rule_strict = Rule::pred(Predicate::StatTier {
        text: None,
        stat_ids: vec!["local_physical_damage_+%".into()],
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 2,
        source: None,
    });
    assert!(
        !evaluate(&item, &rule_strict, gd),
        "T8 phys mod should not match tier <= 2"
    );

    // Crit is T3 — tier == 3 should match (local stat for weapons)
    let rule_crit = Rule::pred(Predicate::StatTier {
        text: None,
        stat_ids: vec!["local_critical_strike_chance_+%".into()],
        kind: TierKindFilter::Tier,
        op: Cmp::Eq,
        value: 3,
        source: None,
    });
    assert!(
        evaluate(&item, &rule_crit, gd),
        "T3 crit should match tier == 3"
    );
}

#[test]
fn stat_tier_pseudo_uses_worst_contributing_tier() {
    use poe_eval::predicate::TierKindFilter;

    let gd = full_game_data();
    // rare-warstaff has two phys damage mods: "Heavy" (T8) and "Squire's" (T8).
    // Pseudo "total increased Physical Damage" should have worst tier = 8.
    let item = resolve_full("rare-warstaff-sol-pile.txt");

    // Verify pseudo has a tier set
    let pseudo_mod = item.pseudo_mods.iter().find(|m| {
        m.stat_lines.iter().any(|sl| {
            sl.stat_ids.as_ref().is_some_and(|ids| {
                ids.iter()
                    .any(|id| id == "pseudo_increased_physical_damage")
            })
        })
    });
    assert!(pseudo_mod.is_some(), "should have pseudo phys damage");
    let pseudo = pseudo_mod.unwrap();
    assert!(
        pseudo.header.tier.is_some(),
        "pseudo should have aggregate tier, got {:?}",
        pseudo.header
    );
    assert_eq!(
        pseudo.header.tier.as_ref().unwrap().number(),
        8,
        "pseudo worst tier should be 8 (both contributing mods are T8)"
    );

    // StatTier on pseudo: tier <= 8 should match
    let rule = Rule::pred(Predicate::StatTier {
        text: None,
        stat_ids: vec!["pseudo_increased_physical_damage".into()],
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 8,
        source: None,
    });
    assert!(
        evaluate(&item, &rule, gd),
        "pseudo with worst tier 8 should match tier <= 8"
    );

    // tier <= 3 should NOT match
    let rule_strict = Rule::pred(Predicate::StatTier {
        text: None,
        stat_ids: vec!["pseudo_increased_physical_damage".into()],
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 3,
        source: None,
    });
    assert!(
        !evaluate(&item, &rule_strict, gd),
        "pseudo with worst tier 8 should not match tier <= 3"
    );
}

#[test]
fn tier_count_matches_mod_count() {
    use poe_eval::predicate::TierKindFilter;

    let gd = full_game_data();
    // rare-warstaff: Heavy(T8), Cerulean(T8), Squire's(T8), of Puncturing(T3)
    // 4 mods total, all are Tier (not Rank)
    let item = resolve_full("rare-warstaff-sol-pile.txt");

    // At least 3 mods with tier <= 10 (all 4 qualify)
    let rule = Rule::pred(Predicate::TierCount {
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 10,
        min_count: 3,
        slot: None,
        source: None,
    });
    assert!(evaluate(&item, &rule, gd), "4 mods with tier <= 10 >= 3");

    // At least 1 mod with tier <= 3 (only "of Puncturing" T3)
    let rule_t3 = Rule::pred(Predicate::TierCount {
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 3,
        min_count: 1,
        slot: None,
        source: None,
    });
    assert!(evaluate(&item, &rule_t3, gd), "1 mod with tier <= 3");

    // At least 2 mods with tier <= 3 — should fail
    let rule_two_t3 = Rule::pred(Predicate::TierCount {
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 3,
        min_count: 2,
        slot: None,
        source: None,
    });
    assert!(
        !evaluate(&item, &rule_two_t3, gd),
        "only 1 mod with tier <= 3, need 2"
    );

    // Slot filter: at least 1 prefix with tier <= 8
    let rule_prefix = Rule::pred(Predicate::TierCount {
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 8,
        min_count: 1,
        slot: Some(ModSlotKind::Prefix),
        source: None,
    });
    assert!(evaluate(&item, &rule_prefix, gd), "3 prefixes with T8");
}

// ─── ModSourceKind + ModSlotKind::Affix predicates ──────────────────────────

#[test]
fn tier_count_source_fractured_filters_correctly() {
    use poe_eval::predicate::{ModSourceKind, TierKindFilter};

    let gd = full_game_data();
    // rare-gloves-fractured-t1: Scorching(T1, Fractured), Icy(T4), of Masterstroke(T1), of the Ice(T2)
    let item = resolve_full("rare-gloves-fractured-t1.txt");

    // At least 1 fractured mod with tier <= 1 (Scorching T1 is fractured)
    let rule = Rule::pred(Predicate::TierCount {
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 1,
        min_count: 1,
        slot: None,
        source: Some(ModSourceKind::Fractured),
    });
    assert!(
        evaluate(&item, &rule, gd),
        "Scorching T1 is fractured, should match"
    );

    // At least 2 fractured mods — should fail (only 1 is fractured)
    let rule_two = Rule::pred(Predicate::TierCount {
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 10,
        min_count: 2,
        slot: None,
        source: Some(ModSourceKind::Fractured),
    });
    assert!(
        !evaluate(&item, &rule_two, gd),
        "only 1 fractured mod, need 2"
    );

    // At least 1 regular mod with tier <= 1 (of Masterstroke T1 is regular)
    let rule_regular = Rule::pred(Predicate::TierCount {
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 1,
        min_count: 1,
        slot: None,
        source: Some(ModSourceKind::Regular),
    });
    assert!(
        evaluate(&item, &rule_regular, gd),
        "of Masterstroke T1 is regular, should match"
    );

    // No fractured mod with tier <= 1 in suffix slot (Scorching is a prefix)
    let rule_frac_suffix = Rule::pred(Predicate::TierCount {
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 1,
        min_count: 1,
        slot: Some(ModSlotKind::Suffix),
        source: Some(ModSourceKind::Fractured),
    });
    assert!(
        !evaluate(&item, &rule_frac_suffix, gd),
        "no fractured suffix at T1"
    );
}

#[test]
fn tier_count_affix_slot_combines_prefix_and_suffix() {
    use poe_eval::predicate::TierKindFilter;

    let gd = full_game_data();
    // rare-gloves-fractured-t1: Scorching(T1 Prefix), Icy(T4 Prefix),
    //                           of Masterstroke(T1 Suffix), of the Ice(T2 Suffix)
    // T1-T2 affixes: Scorching(T1), of Masterstroke(T1), of the Ice(T2) = 3
    let item = resolve_full("rare-gloves-fractured-t1.txt");

    // At least 3 affixes with tier <= 2
    let rule = Rule::pred(Predicate::TierCount {
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 2,
        min_count: 3,
        slot: Some(ModSlotKind::Affix),
        source: None,
    });
    assert!(
        evaluate(&item, &rule, gd),
        "3 affixes with tier <= 2 (T1+T1+T2)"
    );

    // At least 4 affixes with tier <= 2 — should fail
    let rule_four = Rule::pred(Predicate::TierCount {
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 2,
        min_count: 4,
        slot: Some(ModSlotKind::Affix),
        source: None,
    });
    assert!(
        !evaluate(&item, &rule_four, gd),
        "only 3 affixes with tier <= 2, need 4"
    );

    // All 4 affixes with tier <= 10
    let rule_all = Rule::pred(Predicate::TierCount {
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 10,
        min_count: 4,
        slot: Some(ModSlotKind::Affix),
        source: None,
    });
    assert!(
        evaluate(&item, &rule_all, gd),
        "all 4 affixes have tier <= 10"
    );
}

#[test]
fn tier_count_affix_with_fractured_source() {
    use poe_eval::predicate::{ModSourceKind, TierKindFilter};

    let gd = full_game_data();
    let item = resolve_full("rare-gloves-fractured-t1.txt");

    // 1 fractured affix at T1 (Scorching is fractured prefix)
    let rule = Rule::pred(Predicate::TierCount {
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 1,
        min_count: 1,
        slot: Some(ModSlotKind::Affix),
        source: Some(ModSourceKind::Fractured),
    });
    assert!(evaluate(&item, &rule, gd), "1 fractured affix at T1");
}

#[test]
fn stat_tier_source_fractured_filters_correctly() {
    use poe_eval::predicate::{ModSourceKind, TierKindFilter};

    let gd = full_game_data();
    // rare-helmet-fractured-dual-influence: "Encased" (T2, Fractured prefix, +Armour)
    let item = resolve_full("rare-helmet-fractured-dual-influence.txt");

    // StatTier on local_armour — T2 fractured, should match with source filter
    let rule = Rule::pred(Predicate::StatTier {
        text: None,
        stat_ids: vec!["local_base_physical_damage_reduction_rating".into()],
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 2,
        source: Some(ModSourceKind::Fractured),
    });
    assert!(
        evaluate(&item, &rule, gd),
        "fractured T2 armour mod should match"
    );

    // Same stat but with Regular source — should NOT match (it's fractured)
    let rule_regular = Rule::pred(Predicate::StatTier {
        text: None,
        stat_ids: vec!["local_base_physical_damage_reduction_rating".into()],
        kind: TierKindFilter::Tier,
        op: Cmp::Le,
        value: 2,
        source: Some(ModSourceKind::Regular),
    });
    assert!(
        !evaluate(&item, &rule_regular, gd),
        "armour mod is fractured, not regular"
    );
}

// ─── Boolean stat matching (enchants with no numeric values) ────────────────

#[test]
fn boolean_enchant_stat_matches_on_presence() {
    let gd = full_game_data();
    let item = resolve_full("blueprint-normal-bunker.txt");

    // "Heist Targets are always Replica Unique Items" has stat_id but no numeric values.
    // StatValue should treat it as a presence check.
    let rule = Rule::pred(Predicate::StatValue {
        conditions: vec![StatCondition {
            text: None,
            stat_ids: vec!["heist_blueprint_reward_always_unique".to_string()],
            value_index: 0,
            op: Cmp::Ge,
            value: 1,
        }],
    });
    assert!(
        evaluate(&item, &rule, gd),
        "boolean enchant stat should match on presence"
    );
}

#[test]
fn boolean_enchant_stat_does_not_match_wrong_id() {
    let gd = full_game_data();
    let item = resolve_full("blueprint-normal-bunker.txt");

    // Stat ID that is NOT on this item — should not match.
    let rule = Rule::pred(Predicate::StatValue {
        conditions: vec![StatCondition {
            text: None,
            stat_ids: vec!["heist_blueprint_reward_always_experimented".to_string()],
            value_index: 0,
            op: Cmp::Ge,
            value: 1,
        }],
    });
    assert!(
        !evaluate(&item, &rule, gd),
        "wrong stat_id should not match"
    );
}
