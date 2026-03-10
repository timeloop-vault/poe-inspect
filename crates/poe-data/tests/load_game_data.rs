use poe_data::{GameData, StatSuggestionKind, load};

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
    assert!(
        gd.item_class_categories.len() > 3,
        "expected >3 item class categories"
    );
    assert!(gd.base_item_types.len() > 5_000, "expected >5k base items");
    assert!(gd.mods.len() > 30_000, "expected >30k mods");
    assert!(
        gd.rarities.len() >= 4,
        "expected >=4 rarities (Normal/Magic/Rare/Unique)"
    );

    println!(
        "Loaded: {} stats, {} tags, {} classes, {} categories, {} base items, {} mod families, {} mod types, {} mods, {} rarities",
        gd.stats.len(),
        gd.tags.len(),
        gd.item_classes.len(),
        gd.item_class_categories.len(),
        gd.base_item_types.len(),
        gd.mod_families.len(),
        gd.mod_types.len(),
        gd.mods.len(),
        gd.rarities.len()
    );
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

    // Rarity by id
    let rare = gd.rarity("Rare");
    assert!(rare.is_some(), "Rare rarity not found");
    let rare = rare.unwrap();
    assert!(rare.max_prefix > 0, "Rare should have max_prefix > 0");
    assert!(rare.max_suffix > 0, "Rare should have max_suffix > 0");
    println!(
        "Rare: max_prefix={}, max_suffix={}",
        rare.max_prefix, rare.max_suffix
    );

    // Max prefixes/suffixes helper
    assert!(gd.max_prefixes("Rare").is_some());
    assert!(gd.max_suffixes("Rare").is_some());
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
        let tag_names: Vec<_> = item.tags.iter().filter_map(|&fk| gd.tag_id(fk)).collect();
        println!("Iron Greaves tags: {:?}", tag_names);
        assert!(!tag_names.is_empty(), "should have tags");
    }
}

#[test]
fn stat_to_mod_index_works() {
    let Some(gd) = load_test_data() else { return };

    // base_maximum_life should appear in many mods (pure life + hybrid combos).
    let templates = gd.templates_for_stat("base_maximum_life");
    assert!(
        templates.is_some(),
        "base_maximum_life should have templates"
    );
    let templates = templates.unwrap();
    assert!(!templates.is_empty());
    println!(
        "base_maximum_life templates: {:?}",
        &templates[..templates.len().min(3)]
    );

    // Query "maximum Life" — should return single suggestions AND hybrid combos.
    let suggestions = gd.stat_suggestions_for_query("maximum Life");
    let singles: Vec<_> = suggestions
        .iter()
        .filter(|s| matches!(s.kind, StatSuggestionKind::Single))
        .collect();
    let hybrids: Vec<_> = suggestions
        .iter()
        .filter(|s| matches!(s.kind, StatSuggestionKind::Hybrid { .. }))
        .collect();

    println!(
        "Query 'maximum Life': {} singles, {} hybrids",
        singles.len(),
        hybrids.len()
    );
    assert!(
        !singles.is_empty(),
        "should have single suggestions for 'maximum Life'"
    );
    assert!(
        !hybrids.is_empty(),
        "should have hybrid suggestions for 'maximum Life'"
    );

    // Print first few hybrids for inspection.
    for h in hybrids.iter().take(5) {
        if let StatSuggestionKind::Hybrid {
            mod_name,
            generation_type,
            other_templates,
            other_stat_ids,
        } = &h.kind
        {
            let affix = if *generation_type == 1 {
                "prefix"
            } else {
                "suffix"
            };
            println!(
                "  Hybrid ({affix}) \"{mod_name}\": {} + {:?} (other_stat_ids: {:?})",
                h.template, other_templates, other_stat_ids
            );
        }
    }
}

#[test]
fn local_stat_template_fallback() {
    let Some(gd) = load_test_data() else { return };

    // Local defence stats used in hybrid mods should resolve to display templates
    // via the local→non-local fallback in set_reverse_index().
    assert!(
        gd.templates_for_stat("local_base_physical_damage_reduction_rating")
            .is_some(),
        "local armour stat should have template via prefix-strip fallback"
    );
    assert!(
        gd.templates_for_stat("local_base_evasion_rating").is_some(),
        "local evasion stat should have template via prefix-strip fallback"
    );
    assert!(
        gd.templates_for_stat("local_energy_shield").is_some(),
        "local ES stat should have template via hardcoded fallback"
    );

    // Hybrid suggestions for "maximum Life" should include armour+life hybrids
    // with resolved other_templates and real stat_ids from the Mods table.
    let suggestions = gd.stat_suggestions_for_query("# to maximum Life");
    let armour_life_hybrids: Vec<_> = suggestions
        .iter()
        .filter(|s| {
            matches!(&s.kind, StatSuggestionKind::Hybrid { other_stat_ids, .. }
            if other_stat_ids.iter().any(|id| id == "local_base_physical_damage_reduction_rating"))
        })
        .collect();
    assert!(
        !armour_life_hybrids.is_empty(),
        "should find armour+life hybrid mods (with real local stat_ids)"
    );
    for h in &armour_life_hybrids {
        if let StatSuggestionKind::Hybrid {
            other_templates,
            other_stat_ids,
            ..
        } = &h.kind
        {
            assert!(
                !other_templates.is_empty(),
                "armour+life hybrids should have other_templates"
            );
            assert!(
                other_templates.iter().any(|t| t.contains("Armour")),
                "other_templates should contain Armour template"
            );
            // Stat IDs come directly from the Mods table — should be real (local) IDs
            assert!(
                other_stat_ids.iter().any(|id| id.starts_with("local_")),
                "other_stat_ids should use real stat IDs from Mods table, got: {:?}",
                other_stat_ids
            );
        }
    }

    // Single suggestions for stats with local equivalents should include both
    // non-local and local stat_ids (so rules match items regardless of context).
    let armour_singles: Vec<_> = gd
        .stat_suggestions_for_query("to Armour")
        .into_iter()
        .filter(|s| {
            matches!(s.kind, StatSuggestionKind::Single)
                && s.stat_ids
                    .iter()
                    .any(|id| id == "base_physical_damage_reduction_rating")
        })
        .collect();
    assert!(
        !armour_singles.is_empty(),
        "should find a Single suggestion for armour stat"
    );
    let single = &armour_singles[0];
    assert!(
        single
            .stat_ids
            .iter()
            .any(|id| id == "base_physical_damage_reduction_rating"),
        "Single suggestion should include non-local stat_id, got: {:?}",
        single.stat_ids
    );
    assert!(
        single
            .stat_ids
            .iter()
            .any(|id| id == "local_base_physical_damage_reduction_rating"),
        "Single suggestion should include local stat_id, got: {:?}",
        single.stat_ids
    );
}
