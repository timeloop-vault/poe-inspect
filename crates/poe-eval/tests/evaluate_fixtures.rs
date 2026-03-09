use std::path::PathBuf;
use std::sync::OnceLock;

use poe_dat::tables::{BaseItemTypeRow, RarityRow};
use poe_data::GameData;
use poe_eval::affix;
use poe_eval::predicate::{Cmp, InfluenceValue, ModSlotKind, Predicate, RarityValue, StatusValue};
use poe_eval::tier;
use poe_eval::{Modifiability, TierQuality};
use poe_eval::profile::{Profile, ScoringRule};
use poe_eval::rule::Rule;
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
        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/poe-data/data");
        poe_data::load(&data_dir)
            .expect("full game data required — run poe-data extraction first")
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

    GameData::new(vec![], vec![], vec![], vec![], base_item_types, vec![], vec![], vec![], vec![])
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
        RarityRow { id: "Normal".into(), min_mods: 0, max_mods: 0, max_prefix: 0, max_suffix: 0, text: "Normal".into() },
        RarityRow { id: "Magic".into(), min_mods: 1, max_mods: 2, max_prefix: 1, max_suffix: 1, text: "Magic".into() },
        RarityRow { id: "Rare".into(), min_mods: 4, max_mods: 6, max_prefix: 3, max_suffix: 3, text: "Rare".into() },
        RarityRow { id: "Unique".into(), min_mods: 0, max_mods: 0, max_prefix: 0, max_suffix: 0, text: "Unique".into() },
    ];

    GameData::new(vec![], vec![], vec![], vec![], base_item_types, vec![], vec![], vec![], rarities)
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
    // StatValue checks all matching stats — any match satisfies the predicate.
    let rule = Rule::pred(Predicate::StatValue {
        text: None,
        stat_id: Some("base_maximum_life".to_string()),
        value_index: 0,
        op: Cmp::Ge,
        value: 30,
    });
    assert!(evaluate(&item, &rule, gd));

    // >= 40 should pass because the prefix has +49
    let rule = Rule::pred(Predicate::StatValue {
        text: None,
        stat_id: Some("base_maximum_life".to_string()),
        value_index: 0,
        op: Cmp::Ge,
        value: 40,
    });
    assert!(evaluate(&item, &rule, gd));

    // >= 55 should fail — neither 32 nor 49 reaches 55
    let rule = Rule::pred(Predicate::StatValue {
        text: None,
        stat_id: Some("base_maximum_life".to_string()),
        value_index: 0,
        op: Cmp::Ge,
        value: 55,
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
        text: None,
        stat_id: Some("base_maximum_life".to_string()),
        value_index: 0,
        op: Cmp::Gt,
        value: 100,
    });
    assert!(evaluate(&item, &rule, gd));

    // Neither +24 nor +139 exceeds 150
    let rule_high = Rule::pred(Predicate::StatValue {
        text: None,
        stat_id: Some("base_maximum_life".to_string()),
        value_index: 0,
        op: Cmp::Gt,
        value: 150,
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
        stat_id: Some("base_maximum_life".to_string()),
        value_index: 0,
        op: Cmp::Ge,
        value: 40,
    });
    assert!(evaluate(&item, &rule, gd));

    // >= 90 should fail — neither roll is that high
    let rule = Rule::pred(Predicate::RollPercent {
        text: None,
        stat_id: Some("base_maximum_life".to_string()),
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

    let summary = tier::analyze_tiers(&item);

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

    let summary = tier::analyze_tiers(&item);

    // Has T1, T1, T2, T4, T5 — best is Best (T1)
    assert_eq!(summary.best_explicit, TierQuality::Best);
    assert!(summary.quality_counts.best >= 2); // Two T1 mods
}

#[test]
fn tier_analysis_unique_has_no_tiers() {
    let gd = test_game_data(&[]);
    let item = resolve("unique-ring-ventors-gamble.txt", &gd);

    let summary = tier::analyze_tiers(&item);

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
