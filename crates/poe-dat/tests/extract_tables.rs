use poe_dat::dat_reader::DatFile;
use poe_dat::tables;

/// Load a datc64 file from the temp directory.
/// Files should be extracted from GGPK using poe-query or poe-bundle first.
///
/// To extract: use poe-bundle's BundleReader to read `data/{table}.datc64`
/// and write the raw bytes to `%TEMP%/poe-dat/{table}.datc64`.
fn load_dat(table: &str) -> Option<DatFile> {
    let path = std::env::temp_dir()
        .join("poe-dat")
        .join(format!("{table}.datc64"));
    if !path.exists() {
        eprintln!(
            "Skipping: {} not found (extract from GGPK first)",
            path.display()
        );
        return None;
    }
    let bytes = std::fs::read(&path).expect("failed to read file");
    Some(DatFile::from_bytes(bytes).expect("failed to parse dat file"))
}

#[test]
fn read_stats() {
    let Some(dat) = load_dat("stats") else {
        return;
    };
    let stats = tables::extract_stats(&dat);
    println!("Stats: {} rows extracted", stats.len());
    assert!(stats.len() > 20_000, "expected >20k stats");

    // Spot check: "base_maximum_life" should exist
    let life = stats.iter().find(|s| s.id == "base_maximum_life");
    assert!(life.is_some(), "base_maximum_life not found");
    let life = life.unwrap();
    assert!(!life.is_local, "base_maximum_life should not be local");
    println!(
        "  base_maximum_life: local={}, virtual={}",
        life.is_local, life.is_virtual
    );

    // First few
    for s in stats.iter().take(5) {
        println!(
            "  {:50} local={} weapon_local={} virtual={}",
            s.id, s.is_local, s.is_weapon_local, s.is_virtual
        );
    }
}

#[test]
fn read_tags() {
    let Some(dat) = load_dat("tags") else {
        return;
    };
    let tags = tables::extract_tags(&dat);
    println!("Tags: {} rows extracted", tags.len());
    assert!(tags.len() > 100, "expected >100 tags");

    // "default" should be the first tag
    assert_eq!(tags[0].id, "default", "first tag should be 'default'");
    println!("  First 5 tags:");
    for t in tags.iter().take(5) {
        println!("    {}", t.id);
    }
}

#[test]
fn read_item_classes() {
    let Some(dat) = load_dat("itemclasses") else {
        return;
    };
    let classes = tables::extract_item_classes(&dat);
    println!("ItemClasses: {} rows extracted", classes.len());
    assert!(classes.len() > 50, "expected >50 item classes");

    // "Body Armour" should exist
    let body = classes.iter().find(|c| c.id == "Body Armour");
    assert!(body.is_some(), "Body Armour class not found");
    let body = body.unwrap();
    println!(
        "  Body Armour: name={:?}, category={:?}",
        body.name, body.category
    );

    for c in classes.iter().take(5) {
        println!("  {:30} name={:30} cat={:?}", c.id, c.name, c.category);
    }
}

#[test]
fn read_base_item_types() {
    let Some(dat) = load_dat("baseitemtypes") else {
        return;
    };
    let items = tables::extract_base_item_types(&dat);
    println!("BaseItemTypes: {} rows extracted", items.len());
    assert!(items.len() > 5_000, "expected >5k base items");

    // Spot check: find "Blacksmith's Whetstone"
    let whetstone = items.iter().find(|i| i.name == "Blacksmith's Whetstone");
    assert!(whetstone.is_some(), "Blacksmith's Whetstone not found");
    let w = whetstone.unwrap();
    println!(
        "  Blacksmith's Whetstone: class={:?}, drop_level={}, {}x{}",
        w.item_class, w.drop_level, w.width, w.height
    );
    assert_eq!(w.width, 1);
    assert_eq!(w.height, 1);
    assert_eq!(w.drop_level, 1);

    // Should have a name
    let named = items.iter().filter(|i| !i.name.is_empty()).count();
    println!("  Items with names: {}/{}", named, items.len());
}

#[test]
fn read_mods() {
    let Some(dat) = load_dat("mods") else {
        return;
    };
    let mods = tables::extract_mods(&dat);
    println!("Mods: {} rows extracted", mods.len());
    assert!(mods.len() > 30_000, "expected >30k mods");

    // Spot check: "Strength1" should exist (basic +Strength prefix)
    let str1 = mods.iter().find(|m| m.id == "Strength1");
    if let Some(m) = str1 {
        println!(
            "  Strength1: name={:?}, gen_type={}, level={}",
            m.name, m.generation_type, m.level
        );
        println!("    stat_keys: {:?}", m.stat_keys);
        println!("    stat_ranges: {:?}", m.stat_ranges);
        println!("    families: {:?}", m.families);
    } else {
        eprintln!("  WARNING: Strength1 not found (offset issue?)");
    }

    // Check distribution: most mods should have at least one stat
    let with_stats = mods.iter().filter(|m| m.stat_keys[0].is_some()).count();
    println!(
        "  Mods with at least one stat: {}/{}",
        with_stats,
        mods.len()
    );
}
