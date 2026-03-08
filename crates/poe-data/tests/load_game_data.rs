use poe_data::{load, GameData};

fn load_test_data() -> Option<GameData> {
    let dir = std::env::temp_dir().join("poe-dat");
    if !dir.exists() {
        eprintln!(
            "Skipping: {} not found (run extract_dat first)",
            dir.display()
        );
        return None;
    }
    Some(load(&dir).expect("failed to load game data"))
}

#[test]
fn loads_all_tables() {
    let Some(gd) = load_test_data() else { return };

    assert!(gd.stats.len() > 20_000, "expected >20k stats");
    assert!(gd.tags.len() > 100, "expected >100 tags");
    assert!(gd.item_classes.len() > 50, "expected >50 item classes");
    assert!(gd.base_item_types.len() > 5_000, "expected >5k base items");
    assert!(gd.mods.len() > 30_000, "expected >30k mods");

    println!("Loaded: {} stats, {} tags, {} classes, {} base items, {} mod families, {} mod types, {} mods",
        gd.stats.len(), gd.tags.len(), gd.item_classes.len(),
        gd.base_item_types.len(), gd.mod_families.len(),
        gd.mod_types.len(), gd.mods.len());
}

#[test]
fn id_lookups_work() {
    let Some(gd) = load_test_data() else { return };

    // Stat by id
    let life = gd.stat("base_maximum_life");
    assert!(life.is_some(), "base_maximum_life not found");
    assert!(!life.unwrap().is_local);

    // Mod by id
    let str1 = gd.mod_by_id("Strength1");
    assert!(str1.is_some(), "Strength1 not found");
    assert_eq!(str1.unwrap().name, "of the Brute");

    // Base item by name
    let whetstone = gd.base_item_by_name("Blacksmith's Whetstone");
    assert!(whetstone.is_some(), "Blacksmith's Whetstone not found");
    assert_eq!(whetstone.unwrap().width, 1);

    // Item class by id
    let body = gd.item_class("Body Armour");
    assert!(body.is_some(), "Body Armour class not found");

    // Tag by id
    let default = gd.tag("default");
    assert!(default.is_some(), "default tag not found");
}

#[test]
fn fk_resolution_works() {
    let Some(gd) = load_test_data() else { return };

    // Strength1 has stat_keys[0] pointing to a Stats row
    let str1 = gd.mod_by_id("Strength1").unwrap();
    let stat_fk = str1.stat_keys[0].expect("Strength1 should have stat_key[0]");
    let stat_id = gd.stat_id(stat_fk);
    assert!(stat_id.is_some(), "stat FK should resolve");
    println!("Strength1 stat[0] = {} (FK {})", stat_id.unwrap(), stat_fk);

    // Mod type resolution
    if let Some(mt_fk) = str1.mod_type {
        let mt_name = gd.mod_type_name(mt_fk);
        println!("Strength1 mod_type = {:?} (FK {})", mt_name, mt_fk);
    }

    // Family resolution
    for &fam_fk in &str1.families {
        let fam_id = gd.mod_family_id(fam_fk);
        println!("Strength1 family = {:?} (FK {})", fam_id, fam_fk);
    }

    // Base item tag resolution
    let greaves = gd.base_item_by_name("Iron Greaves");
    if let Some(item) = greaves {
        let tag_names: Vec<_> = item.tags.iter()
            .filter_map(|&fk| gd.tag_id(fk))
            .collect();
        println!("Iron Greaves tags: {:?}", tag_names);
        assert!(!tag_names.is_empty(), "should have tags");
    }
}
