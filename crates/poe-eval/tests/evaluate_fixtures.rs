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

/// Load full game data (with reverse index for stat_id resolution).
/// Cached via OnceLock so it's loaded at most once per test run.
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
fn has_stat_text() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let rule = Rule::pred(Predicate::HasStatText {
        text: "maximum Life".to_string(),
    });
    assert!(evaluate(&item, &rule, &gd));
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
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

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
        Rule::pred(Predicate::HasStatText {
            text: "maximum Life".to_string(),
        }),
        Rule::negate(Rule::pred(Predicate::HasStatus {
            status: StatusValue::Corrupted,
        })),
    ]);
    assert!(evaluate(&item, &rule, &gd));
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
                rule: Rule::pred(Predicate::HasStatText {
                    text: "maximum Life".to_string(),
                }),
            },
            ScoringRule {
                label: "Has resistances".to_string(),
                weight: 5.0,
                rule: Rule::pred(Predicate::HasStatText {
                    text: "Resistance".to_string(),
                }),
            },
            ScoringRule {
                label: "Has armour".to_string(),
                weight: 3.0,
                rule: Rule::pred(Predicate::HasStatText {
                    text: "Armour".to_string(),
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
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let result = score(&item, &belt_profile(), &gd);

    assert!(result.applicable);
    assert!(result.score > 0.0);
    // Belt has life, resistances, armour, and is not corrupted
    assert!(result.matched.len() >= 3);
}

#[test]
fn score_filter_rejects() {
    let gd = test_game_data(&[]);
    let item = resolve("unique-ring-ventors-gamble.txt", &gd);

    let result = score(&item, &belt_profile(), &gd);

    // Ring should not match belt profile filter
    assert!(!result.applicable);
    assert_eq!(result.score, 0.0);
    assert!(result.matched.is_empty());
}

#[test]
fn score_detailed_breakdown() {
    let gd = test_game_data(&[]);
    let item = resolve("rare-belt-crafted.txt", &gd);

    let result = score(&item, &belt_profile(), &gd);

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
    let gd = test_game_data(&[]);
    let item = resolve("unique-ring-ventors-gamble.txt", &gd);

    // Profile with no filter — applies to everything
    let profile = Profile {
        name: "Has Life".to_string(),
        description: String::new(),
        filter: None,
        scoring: vec![ScoringRule {
            label: "life".to_string(),
            weight: 10.0,
            rule: Rule::pred(Predicate::HasStatText {
                text: "maximum Life".to_string(),
            }),
        }],
    };

    let result = score(&item, &profile, &gd);
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

// ─── StatValue multi-condition (same-mod check) ─────────────────────────────

// Body armour fixture mod layout (used by many tests below):
// - "Urchin's" (hybrid): local_base_physical_damage_reduction_rating + base_maximum_life
// - "Carapaced" (pure armour): local_base_physical_damage_reduction_rating
// - "Fecund" (pure life): base_maximum_life
// - "of the Ice": base_cold_damage_resistance_%
// - "of the Volcano": base_fire_damage_resistance_%

/// Multi-condition StatValue matches when ALL conditions are on a SINGLE mod.
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
/// even though separate single-condition checks (via Rule::All) would match.
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

/// Contrast: multi-condition vs Rule::All on same item.
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

/// StatValue serialization round-trip with conditions.
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
    assert!(item
        .explicits
        .iter()
        .any(|m| m.header.name.as_deref() == Some("Dragon's")));
}

/// Debug: trace stat_ids through the full pipeline for a body armour with hybrid mods.
#[test]
fn trace_stat_ids_body_armour() {
    let gd = full_game_data();
    let item = resolve_full("rare-body-armour-craft-hybrid-and-normal-life-mod.txt");

    println!("\n=== RESOLVED ITEM STAT IDS ===");
    for m in item.implicits.iter().chain(item.explicits.iter()) {
        println!(
            "\nMod: {:?} ({:?})",
            m.header.name, m.header.slot
        );
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
    let armour_single = suggestions
        .iter()
        .find(|s| {
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
    let life_single = life_suggestions
        .iter()
        .find(|s| {
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
