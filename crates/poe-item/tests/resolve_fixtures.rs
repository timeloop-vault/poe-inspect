use poe_data::GameData;
use poe_dat::tables::BaseItemTypeRow;
use poe_item::types::{ModSlot, ModSource, Rarity, ValueRange};

fn fixture(name: &str) -> String {
    let path = format!("{}/../../fixtures/items/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

/// Minimal `GameData` with only base item types for testing.
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

fn resolve_fixture(name: &str, gd: &GameData) -> poe_item::types::ResolvedItem {
    let raw = poe_item::parse(&fixture(name)).unwrap();
    poe_item::resolve(&raw, gd)
}

// ─── Header resolution ──────────────────────────────────────────────────────

#[test]
fn rare_header_resolved() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-belt-crafted.txt", &gd);

    assert_eq!(item.header.rarity, Rarity::Rare);
    assert_eq!(item.header.name.as_deref(), Some("Doom Snare"));
    assert_eq!(item.header.base_type, "Leather Belt");
    assert_eq!(item.header.item_class, "Belts");
}

#[test]
fn unique_header_resolved() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("unique-ring-ventors-gamble.txt", &gd);

    assert_eq!(item.header.rarity, Rarity::Unique);
    assert_eq!(item.header.name.as_deref(), Some("Ventor's Gamble"));
    assert_eq!(item.header.base_type, "Gold Ring");
}

#[test]
fn normal_header_resolved() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("normal-staff-elder.txt", &gd);

    assert_eq!(item.header.rarity, Rarity::Normal);
    assert_eq!(item.header.name, None);
    assert_eq!(item.header.base_type, "Imperial Staff");
}

#[test]
fn magic_base_type_extracted() {
    let gd = test_game_data(&["Foul Staff", "Staff"]);
    let item = resolve_fixture("magic-axe-two-handed.txt", &gd);

    assert_eq!(item.header.rarity, Rarity::Magic);
    assert_eq!(item.header.name, None);
    // Should find "Foul Staff" (longest match), not "Staff"
    assert_eq!(item.header.base_type, "Foul Staff");
}

#[test]
fn magic_base_type_fallback() {
    // No matching base types in game data — falls back to full name
    let gd = test_game_data(&[]);
    let item = resolve_fixture("magic-axe-two-handed.txt", &gd);

    assert_eq!(item.header.base_type, "Smouldering Foul Staff");
}

#[test]
fn magic_jewel_base_extracted() {
    let gd = test_game_data(&["Cobalt Jewel"]);
    let item = resolve_fixture("magic-jewel-cobalt.txt", &gd);

    assert_eq!(item.header.base_type, "Cobalt Jewel");
}

#[test]
fn magic_flask_base_extracted() {
    let gd = test_game_data(&["Divine Life Flask"]);
    let item = resolve_fixture("magic-flask-life.txt", &gd);

    assert_eq!(item.header.base_type, "Divine Life Flask");
}

#[test]
fn gem_header_resolved() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("leap-slam.txt", &gd);

    assert_eq!(item.header.rarity, Rarity::Gem);
    assert_eq!(item.header.name, None);
    assert_eq!(item.header.base_type, "Leap Slam");
}

// ─── Section flattening ─────────────────────────────────────────────────────

#[test]
fn sections_flattened() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-belt-crafted.txt", &gd);

    assert_eq!(item.item_level, Some(50));
    assert_eq!(item.requirements.len(), 1);
    assert_eq!(item.requirements[0].key, "Level");
    assert!(item.sockets.is_none());
    assert!(item.monster_level.is_none());
}

#[test]
fn map_sections_flattened() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-map-shore.txt", &gd);

    assert_eq!(item.item_level, Some(75));
    assert_eq!(item.monster_level, Some(73));
    // Maps have property sections (Map Tier, IIQ, IIR, etc.)
    assert!(!item.properties.is_empty());
}

#[test]
fn influences_collected() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-boots-eater-exarch.txt", &gd);

    assert!(item.influences.len() >= 2);
}

#[test]
fn statuses_collected() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-map-city-square-delirium.txt", &gd);

    assert!(item.statuses.iter().any(|s| *s == poe_item::types::StatusKind::Corrupted));
    assert!(item.is_corrupted);
}

// ─── Mod resolution ─────────────────────────────────────────────────────────

#[test]
fn rare_belt_mods_resolved() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-belt-crafted.txt", &gd);

    // 1 implicit + 5 explicit mods (across two mod sections)
    let all_mods: Vec<_> = item.all_mods().collect();
    assert!(all_mods.len() >= 5);

    // Check implicit mod
    let implicit = item.implicits.iter().find(|m| m.header.slot == ModSlot::Implicit).unwrap();
    assert_eq!(implicit.stat_lines.len(), 1);
    assert_eq!(implicit.stat_lines[0].display_text, "+32 to maximum Life");
    assert_eq!(implicit.stat_lines[0].values.len(), 1);
    assert_eq!(
        implicit.stat_lines[0].values[0],
        ValueRange { current: 32, min: 25, max: 40 }
    );

    // Check a prefix mod
    let studded = item.explicits.iter().find(|m| {
        m.header.name.as_deref() == Some("Studded")
    }).unwrap();
    assert_eq!(studded.header.slot, ModSlot::Prefix);
    assert_eq!(studded.stat_lines[0].display_text, "+28 to Armour");
    assert_eq!(
        studded.stat_lines[0].values[0],
        ValueRange { current: 28, min: 11, max: 35 }
    );

    // Check master crafted mod
    let crafted = item.explicits.iter().find(|m| m.header.source == ModSource::MasterCrafted).unwrap();
    assert_eq!(crafted.stat_lines[0].display_text, "+10% to Cold and Lightning Resistances");
}

#[test]
fn unique_mods_negative_ranges() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("unique-ring-ventors-gamble.txt", &gd);

    let unique_mods: Vec<_> = item.explicits.iter().filter(|m| m.header.slot == ModSlot::Unique).collect();
    assert_eq!(unique_mods.len(), 6);

    // "+44(0-60) to maximum Life"
    let life_mod = &unique_mods[0];
    assert_eq!(life_mod.stat_lines[0].display_text, "+44 to maximum Life");
    assert_eq!(
        life_mod.stat_lines[0].values[0],
        ValueRange { current: 44, min: 0, max: 60 }
    );

    // "-9(-25-50)% to Cold Resistance"
    let cold_mod = &unique_mods[2];
    assert_eq!(cold_mod.stat_lines[0].display_text, "-9% to Cold Resistance");
    assert_eq!(
        cold_mod.stat_lines[0].values[0],
        ValueRange { current: -9, min: -25, max: 50 }
    );

    // "1(10--10)% reduced Quantity of Items found"
    let qty_mod = &unique_mods[4];
    assert_eq!(qty_mod.stat_lines[0].display_text, "1% reduced Quantity of Items found");
    assert_eq!(
        qty_mod.stat_lines[0].values[0],
        ValueRange { current: 1, min: 10, max: -10 }
    );
}

#[test]
fn implicit_suffix_stripped() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("unique-ring-ventors-gamble.txt", &gd);

    let implicit = item.implicits.first().unwrap();
    // Original: "15(6-15)% increased Rarity of Items found (implicit)"
    assert_eq!(
        implicit.stat_lines[0].display_text,
        "15% increased Rarity of Items found"
    );
}

#[test]
fn adds_damage_two_ranges() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("magic-axe-two-handed.txt", &gd);

    let fire_mod = item.explicits.iter().find(|m| {
        m.header.name.as_deref() == Some("Smouldering")
    }).unwrap();

    assert_eq!(fire_mod.stat_lines[0].display_text, "Adds 18 to 33 Fire Damage");
    assert_eq!(fire_mod.stat_lines[0].values.len(), 2);
    assert_eq!(
        fire_mod.stat_lines[0].values[0],
        ValueRange { current: 18, min: 14, max: 20 }
    );
    assert_eq!(
        fire_mod.stat_lines[0].values[1],
        ValueRange { current: 33, min: 29, max: 33 }
    );
}

#[test]
fn reminder_text_flagged() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-belt-crafted.txt", &gd);

    // "of the Pugilist" has reminder text about Stun Threshold
    let pugilist = item.explicits.iter().find(|m| {
        m.header.name.as_deref() == Some("of the Pugilist")
    }).unwrap();
    assert_eq!(pugilist.stat_lines.len(), 2);

    // First line is the actual stat
    assert!(!pugilist.stat_lines[0].is_reminder);
    assert_eq!(pugilist.stat_lines[0].display_text, "6% reduced Enemy Stun Threshold");

    // Second line is reminder text
    assert!(pugilist.stat_lines[1].is_reminder);
    assert!(pugilist.stat_lines[1].raw_text.starts_with('('));
}

#[test]
fn no_ranges_in_unscaled_line() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("magic-axe-two-handed.txt", &gd);

    let implicit = item.implicits.first().unwrap();
    // "+22% Chance to Block Attack Damage while wielding a Staff" — no range annotations
    assert!(implicit.stat_lines[0].values.is_empty());
    assert_eq!(
        implicit.stat_lines[0].display_text,
        "+22% Chance to Block Attack Damage while wielding a Staff"
    );
}

#[test]
fn stat_ids_none_without_reverse_index() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-belt-crafted.txt", &gd);

    // Without a ReverseIndex, stat_ids should be None
    for m in item.all_mods() {
        for line in &m.stat_lines {
            assert!(line.stat_ids.is_none());
            assert!(line.stat_values.is_none());
        }
    }
}

#[test]
fn multi_line_mod_resolved() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("magic-flask-life.txt", &gd);

    // "of Allaying" has two stat lines (immunity to Bleeding + Corrupted Blood)
    let allaying = item.explicits.iter().find(|m| {
        m.header.name.as_deref() == Some("of Allaying")
    }).unwrap();
    assert_eq!(allaying.stat_lines.len(), 2);
    assert!(!allaying.stat_lines[0].is_reminder);
    assert!(!allaying.stat_lines[1].is_reminder);
}

#[test]
fn properties_preserved() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("magic-flask-life.txt", &gd);

    // Flask has property sections (recovery, charges) and usage text
    // Some sections become properties, some become unclassified
    let total_sections = item.properties.len()
        + item.unclassified_sections.len()
        + if item.flavor_text.is_some() { 1 } else { 0 };
    assert!(total_sections >= 2);
}

// ─── New enriched fields ─────────────────────────────────────────────────────

#[test]
fn implicits_and_explicits_split() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-belt-crafted.txt", &gd);

    assert!(!item.implicits.is_empty(), "should have implicits");
    assert!(!item.explicits.is_empty(), "should have explicits");
    // All implicits should have implicit slot
    for m in &item.implicits {
        assert!(matches!(
            m.header.slot,
            ModSlot::Implicit | ModSlot::SearingExarchImplicit | ModSlot::EaterOfWorldsImplicit
        ));
    }
    // All explicits should have prefix/suffix/unique slot
    for m in &item.explicits {
        assert!(matches!(
            m.header.slot,
            ModSlot::Prefix | ModSlot::Suffix | ModSlot::Unique
        ));
    }
}

#[test]
fn is_corrupted_flag() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-map-city-square-delirium.txt", &gd);
    assert!(item.is_corrupted);

    let item2 = resolve_fixture("rare-belt-crafted.txt", &gd);
    assert!(!item2.is_corrupted);
}

#[test]
fn unique_has_flavor_text() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("unique-ring-ventors-gamble.txt", &gd);
    assert!(item.flavor_text.is_some(), "unique should have flavor text");
}
