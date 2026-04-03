use std::path::PathBuf;
use std::sync::OnceLock;

use poe_dat::tables::BaseItemTypeRow;
use poe_data::GameData;
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

    assert!(
        item.statuses
            .contains(&poe_item::types::StatusKind::Corrupted)
    );
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
    let implicit = item
        .implicits
        .iter()
        .find(|m| m.header.slot == ModSlot::Implicit)
        .unwrap();
    assert_eq!(implicit.stat_lines.len(), 1);
    assert_eq!(implicit.stat_lines[0].display_text, "+32 to maximum Life");
    assert_eq!(implicit.stat_lines[0].values.len(), 1);
    assert_eq!(
        implicit.stat_lines[0].values[0],
        ValueRange {
            current: 32,
            min: 25,
            max: 40
        }
    );

    // Check a prefix mod
    let studded = item
        .explicits
        .iter()
        .find(|m| m.header.name.as_deref() == Some("Studded"))
        .unwrap();
    assert_eq!(studded.header.slot, ModSlot::Prefix);
    assert_eq!(studded.stat_lines[0].display_text, "+28 to Armour");
    assert_eq!(
        studded.stat_lines[0].values[0],
        ValueRange {
            current: 28,
            min: 11,
            max: 35
        }
    );

    // Check master crafted mod
    let crafted = item
        .explicits
        .iter()
        .find(|m| m.header.source == ModSource::MasterCrafted)
        .unwrap();
    assert_eq!(
        crafted.stat_lines[0].display_text,
        "+10% to Cold and Lightning Resistances"
    );
}

#[test]
fn unique_mods_negative_ranges() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("unique-ring-ventors-gamble.txt", &gd);

    let unique_mods: Vec<_> = item
        .explicits
        .iter()
        .filter(|m| m.header.slot == ModSlot::Unique)
        .collect();
    assert_eq!(unique_mods.len(), 6);

    // "+44(0-60) to maximum Life"
    let life_mod = &unique_mods[0];
    assert_eq!(life_mod.stat_lines[0].display_text, "+44 to maximum Life");
    assert_eq!(
        life_mod.stat_lines[0].values[0],
        ValueRange {
            current: 44,
            min: 0,
            max: 60
        }
    );

    // "-9(-25-50)% to Cold Resistance"
    let cold_mod = &unique_mods[2];
    assert_eq!(
        cold_mod.stat_lines[0].display_text,
        "-9% to Cold Resistance"
    );
    assert_eq!(
        cold_mod.stat_lines[0].values[0],
        ValueRange {
            current: -9,
            min: -25,
            max: 50
        }
    );

    // "1(10--10)% reduced Quantity of Items found"
    let qty_mod = &unique_mods[4];
    assert_eq!(
        qty_mod.stat_lines[0].display_text,
        "1% reduced Quantity of Items found"
    );
    assert_eq!(
        qty_mod.stat_lines[0].values[0],
        ValueRange {
            current: 1,
            min: 10,
            max: -10
        }
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

    let fire_mod = item
        .explicits
        .iter()
        .find(|m| m.header.name.as_deref() == Some("Smouldering"))
        .unwrap();

    assert_eq!(
        fire_mod.stat_lines[0].display_text,
        "Adds 18 to 33 Fire Damage"
    );
    assert_eq!(fire_mod.stat_lines[0].values.len(), 2);
    assert_eq!(
        fire_mod.stat_lines[0].values[0],
        ValueRange {
            current: 18,
            min: 14,
            max: 20
        }
    );
    assert_eq!(
        fire_mod.stat_lines[0].values[1],
        ValueRange {
            current: 33,
            min: 29,
            max: 33
        }
    );
}

#[test]
fn reminder_text_flagged() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-belt-crafted.txt", &gd);

    // "of the Pugilist" has reminder text about Stun Threshold
    let pugilist = item
        .explicits
        .iter()
        .find(|m| m.header.name.as_deref() == Some("of the Pugilist"))
        .unwrap();
    assert_eq!(pugilist.stat_lines.len(), 2);

    // First line is the actual stat
    assert!(!pugilist.stat_lines[0].is_reminder);
    assert_eq!(
        pugilist.stat_lines[0].display_text,
        "6% reduced Enemy Stun Threshold"
    );

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
    let allaying = item
        .explicits
        .iter()
        .find(|m| m.header.name.as_deref() == Some("of Allaying"))
        .unwrap();
    assert_eq!(allaying.stat_lines.len(), 2);
    assert!(!allaying.stat_lines[0].is_reminder);
    assert!(!allaying.stat_lines[1].is_reminder);
}

#[test]
fn properties_preserved() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("magic-flask-life.txt", &gd);

    // Flask recovery section (Recovers/Consumes/Currently has) is unclassified
    // because the lines don't match "Key: Value" property format
    assert!(
        !item.unclassified_sections.is_empty(),
        "flask recovery lines should be unclassified"
    );
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

// ─── Enchant routing ─────────────────────────────────────────────────────────

#[test]
fn talisman_enchant_detected() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-amulet-talisman-corrupted.txt", &gd);
    assert_eq!(item.enchants.len(), 1, "talisman should have 1 enchant");
    assert!(
        item.enchants[0].stat_lines[0]
            .display_text
            .contains("Allocates Entropy"),
        "enchant should be Allocates Entropy"
    );
    assert_eq!(item.enchants[0].header.slot, ModSlot::Enchant);
}

#[test]
fn flask_enchant_detected() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("magic-flask-utility-enchanted.txt", &gd);
    assert_eq!(item.enchants.len(), 1, "flask should have 1 enchant");
    assert!(
        item.enchants[0].stat_lines[0]
            .display_text
            .contains("Charges reach full")
    );
}

#[test]
fn cluster_jewel_enchants_detected() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("magic-cluster-jewel-large.txt", &gd);
    assert_eq!(
        item.enchants.len(),
        6,
        "cluster jewel should have 6 enchant lines"
    );
}

#[test]
fn map_delirium_enchants_detected() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-map-tier5-delirium-enchant.txt", &gd);
    assert_eq!(
        item.enchants.len(),
        2,
        "map should have 2 delirium enchants"
    );
}

// ─── Description field ───────────────────────────────────────────────────────

#[test]
fn currency_has_description() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("currency-chaos-orb.txt", &gd);
    assert!(
        item.description.is_some(),
        "currency should have description"
    );
    assert!(item.description.as_ref().unwrap().contains("Reforges"));
}

#[test]
fn essence_has_description() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("currency-essence-screaming-greed.txt", &gd);
    assert!(
        item.description.is_some(),
        "essence should have description"
    );
    let desc = item.description.as_ref().unwrap();
    assert!(desc.contains("Upgrades"), "should contain upgrade text");
    assert!(desc.contains("Weapon:"), "should contain slot table");
}

#[test]
fn scarab_has_description() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("scarab-titanic.txt", &gd);
    assert!(item.description.is_some(), "scarab should have description");
    assert!(item.description.as_ref().unwrap().contains("Toughness"));
}

#[test]
fn scarab_has_flavor_text() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("scarab-titanic.txt", &gd);
    assert!(item.flavor_text.is_some(), "scarab should have flavor text");
    assert!(
        item.flavor_text
            .as_ref()
            .unwrap()
            .contains("power lies in a name")
    );
}

// ─── Note + statuses ─────────────────────────────────────────────────────────

#[test]
fn note_and_statuses_coexist() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-jewel-cobalt-mirrored-corrupted.txt", &gd);
    assert!(item.note.is_some(), "should have note");
    assert_eq!(item.note.as_deref(), Some("~b/o 35 chaos"));
    assert!(item.is_corrupted);
    // Usage instructions should NOT be flavor text
    assert!(
        item.flavor_text.is_none(),
        "jewel usage instructions should not be flavor text"
    );
}

// ─── Unidentified ────────────────────────────────────────────────────────────

#[test]
fn unidentified_flag() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("rare-axe-unidentified.txt", &gd);
    assert!(item.is_unidentified);
    assert!(
        item.explicits.is_empty(),
        "unidentified item should have no explicits"
    );
    // Unidentified rare: name1 is the base type, not the item name.
    assert_eq!(item.header.base_type, "Vaal Axe");
    assert!(
        item.header.name.is_none(),
        "unidentified item should have no name"
    );
}

#[test]
fn unidentified_unique_header() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("unique-belt-unidentified.txt", &gd);
    assert!(item.is_unidentified);
    assert_eq!(item.header.rarity, Rarity::Unique);
    // Unidentified unique: only the base type is visible, not the unique name.
    assert_eq!(item.header.base_type, "Leather Belt");
    assert!(
        item.header.name.is_none(),
        "unidentified unique should have no name"
    );
    assert!(
        item.explicits.is_empty(),
        "unidentified item should have no explicits"
    );
    // Implicit should still be parsed.
    assert_eq!(item.implicits.len(), 1);
}

// ─── Divination card ─────────────────────────────────────────────────────────

// ─── Gem data ────────────────────────────────────────────────────────────────

#[test]
fn gem_tags_extracted() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("gem-skill-shockwave-totem.txt", &gd);
    let gem = item.gem_data.as_ref().expect("should have gem_data");
    assert_eq!(gem.tags, vec!["Totem", "Spell", "AoE", "Physical", "Nova"]);
}

#[test]
fn gem_description_extracted() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("gem-skill-shockwave-totem.txt", &gd);
    let gem = item.gem_data.as_ref().expect("should have gem_data");
    assert!(
        gem.description
            .as_ref()
            .unwrap()
            .contains("shakes the earth")
    );
}

#[test]
fn gem_stats_and_quality() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("gem-skill-shockwave-totem.txt", &gd);
    let gem = item.gem_data.as_ref().expect("should have gem_data");
    assert!(!gem.stats.is_empty(), "should have stat lines");
    assert!(!gem.quality_stats.is_empty(), "should have quality stats");
    assert!(gem.stats.iter().any(|s| s.contains("Physical Damage")));
    assert!(gem.quality_stats.iter().any(|s| s.contains("radius")));
}

#[test]
fn vaal_gem_data() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("gem-vaal-ice-nova.txt", &gd);
    let gem = item.gem_data.as_ref().expect("should have gem_data");
    assert_eq!(gem.tags, vec!["Spell", "AoE", "Vaal", "Cold", "Nova"]);
    assert!(gem.description.as_ref().unwrap().contains("circle of ice"));

    let vaal = gem.vaal.as_ref().expect("should have vaal variant");
    assert_eq!(vaal.name, "Vaal Ice Nova");
    assert!(vaal.description.as_ref().unwrap().contains("repeating"));
    assert!(!vaal.stats.is_empty());
    assert!(!vaal.quality_stats.is_empty());
    // Vaal properties (Souls Per Use, etc.)
    assert!(vaal.properties.iter().any(|p| p.name == "Souls Per Use"));
}

#[test]
fn gem_no_unclassified() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("gem-skill-shockwave-totem.txt", &gd);
    assert!(
        item.unclassified_sections.is_empty(),
        "gem should have no unclassified sections: {:?}",
        item.unclassified_sections
    );
}

#[test]
fn divination_card_resolved() {
    let gd = test_game_data(&[]);
    let item = resolve_fixture("divination-card-hunters-resolve.txt", &gd);
    assert_eq!(item.header.rarity, Rarity::DivinationCard);
    assert!(
        item.flavor_text.is_some(),
        "div card should have flavor text"
    );
}

#[test]
fn superior_quality_prefix_stripped() {
    let gd = test_game_data(&["Ezomyte Tower Shield"]);
    let item = resolve_fixture("normal-shield-superior-quality.txt", &gd);

    assert_eq!(item.header.rarity, Rarity::Normal);
    assert_eq!(item.header.name, None);
    // "Superior" prefix must be stripped — trade API rejects it
    assert_eq!(item.header.base_type, "Ezomyte Tower Shield");
    assert_eq!(item.header.item_class, "Shields");
}

#[test]
fn fractured_mod_from_header_source() {
    let gd = test_game_data(&["Eternal Burgonet"]);
    let item = resolve_fixture("rare-helmet-fractured-dual-influence.txt", &gd);

    assert!(item.is_fractured, "item should be fractured");

    // The first explicit mod has { Fractured Prefix Modifier } in the header
    let encased = item
        .explicits
        .iter()
        .find(|m| m.header.name.as_deref() == Some("Encased"))
        .expect("should find Encased mod");
    assert!(
        encased.is_fractured,
        "Encased mod should be fractured (from header source)"
    );
    assert_eq!(encased.header.source, ModSource::Fractured);

    // Other mods should NOT be fractured
    let athletes = item
        .explicits
        .iter()
        .find(|m| m.header.name.as_deref() == Some("Athlete's"))
        .expect("should find Athlete's mod");
    assert!(
        !athletes.is_fractured,
        "Athlete's mod should not be fractured"
    );
}

// ─── Full game data tests (require extracted datc64 in poe-data/data/) ──────

/// Load full game data (with reverse index for `stat_id` resolution).
/// Cached via `OnceLock` so it's loaded at most once per test run.
fn full_game_data() -> &'static GameData {
    static GD: OnceLock<GameData> = OnceLock::new();
    GD.get_or_init(|| {
        let data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../poe-data/data");
        poe_data::load(&data_dir).expect("full game data required — run poe-data extraction first")
    })
}

fn resolve_full(name: &str) -> poe_item::types::ResolvedItem {
    let raw = poe_item::parse(&fixture(name)).unwrap();
    poe_item::resolve(&raw, full_game_data())
}

#[test]
fn weapon_flat_damage_stat_ids_resolved() {
    let item = resolve_full("rare-warstaff-flat-phys.txt");

    // "Adds 2 to 4 Physical Damage" (Glinting) should resolve to stat_ids.
    let phys_mod = item
        .explicits
        .iter()
        .find(|m| m.header.name.as_deref() == Some("Glinting"))
        .expect("should find Glinting mod (flat phys damage)");

    let stat_ids: Vec<&str> = phys_mod
        .stat_lines
        .iter()
        .filter_map(|sl| sl.stat_ids.as_ref())
        .flat_map(|ids| ids.iter().map(String::as_str))
        .collect();
    assert!(
        stat_ids
            .iter()
            .any(|id| id.contains("added_physical_damage")),
        "flat phys damage mod should have physical damage stat_id, got: {stat_ids:?}"
    );

    // "114% increased Physical Damage" (Bloodthirsty) — also local on weapons.
    let pct_mod = item
        .explicits
        .iter()
        .find(|m| m.header.name.as_deref() == Some("Bloodthirsty"))
        .expect("should find Bloodthirsty mod (% phys damage)");

    let pct_ids: Vec<&str> = pct_mod
        .stat_lines
        .iter()
        .filter_map(|sl| sl.stat_ids.as_ref())
        .flat_map(|ids| ids.iter().map(String::as_str))
        .collect();
    assert!(
        pct_ids.iter().any(|id| id.contains("physical_damage")),
        "% phys damage mod should have physical damage stat_id, got: {pct_ids:?}"
    );
}

#[test]
fn bow_triple_damage_has_local_stat_ids() {
    let item = resolve_full("rare-bow-triple-damage.txt");

    // Weapon flat damage mods should resolve to local_ stat_ids (not global_/attack_).
    // This tests the full pipeline: template fallback + inherited tags + apply_confirmed_stat_ids.
    for (mod_name, expected_id) in [
        ("Tempered", "local_minimum_added_physical_damage"),
        ("Carbonising", "local_minimum_added_fire_damage"),
        ("Malicious", "local_minimum_added_chaos_damage"),
    ] {
        let m = item
            .explicits
            .iter()
            .find(|m| m.header.name.as_deref() == Some(mod_name))
            .unwrap_or_else(|| panic!("should find {mod_name} mod"));

        let ids: Vec<&str> = m
            .stat_lines
            .iter()
            .filter_map(|sl| sl.stat_ids.as_ref())
            .flat_map(|ids| ids.iter().map(String::as_str))
            .collect();
        assert!(
            ids.contains(&expected_id),
            "{mod_name} should have local stat_id '{expected_id}', got: {ids:?}"
        );
    }
}

/// Multi-value "Adds # to # Damage to Attacks" on gloves should resolve to
/// distinct min and max attack stat IDs (not both minimum).
#[test]
fn gloves_attack_damage_has_distinct_min_max_stat_ids() {
    let item = resolve_full("rare-gloves-fractured-t1.txt");

    // "Scorching" = "Adds 13 to 27 Fire Damage to Attacks" (fractured T1)
    let scorching = item
        .explicits
        .iter()
        .find(|m| m.header.name.as_deref() == Some("Scorching"))
        .expect("should find Scorching mod");

    let stat_ids = scorching.stat_lines[0]
        .stat_ids
        .as_ref()
        .expect("should have stat_ids");

    assert_eq!(
        stat_ids.len(),
        2,
        "multi-value template should have 2 stat IDs"
    );
    assert_eq!(
        stat_ids[0], "attack_minimum_added_fire_damage",
        "first stat should be minimum"
    );
    assert_eq!(
        stat_ids[1], "attack_maximum_added_fire_damage",
        "second stat should be maximum"
    );

    // "Icy" = "Adds 5 to 12 Cold Damage to Attacks"
    let icy = item
        .explicits
        .iter()
        .find(|m| m.header.name.as_deref() == Some("Icy"))
        .expect("should find Icy mod");

    let cold_ids = icy.stat_lines[0]
        .stat_ids
        .as_ref()
        .expect("should have stat_ids");

    assert_eq!(cold_ids.len(), 2);
    assert_eq!(cold_ids[0], "attack_minimum_added_cold_damage");
    assert_eq!(cold_ids[1], "attack_maximum_added_cold_damage");
}

/// Multi-value "Adds # to # Damage" on weapons should resolve to
/// distinct local min and max stat IDs.
#[test]
fn weapon_flat_damage_has_distinct_min_max_stat_ids() {
    let item = resolve_full("rare-bow-triple-damage.txt");

    // "Tempered" = "Adds X to Y Physical Damage" on a weapon
    let tempered = item
        .explicits
        .iter()
        .find(|m| m.header.name.as_deref() == Some("Tempered"))
        .expect("should find Tempered mod");

    let stat_ids = tempered.stat_lines[0]
        .stat_ids
        .as_ref()
        .expect("should have stat_ids");

    assert_eq!(
        stat_ids.len(),
        2,
        "multi-value template should have 2 stat IDs"
    );
    assert_eq!(
        stat_ids[0], "local_minimum_added_physical_damage",
        "first stat should be local minimum"
    );
    assert_eq!(
        stat_ids[1], "local_maximum_added_physical_damage",
        "second stat should be local maximum"
    );
}
