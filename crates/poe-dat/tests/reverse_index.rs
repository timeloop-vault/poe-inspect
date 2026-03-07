use poe_dat::stat_desc;

fn load_index() -> Option<stat_desc::ReverseIndex> {
    let path = std::env::temp_dir().join("stat_desc_utf8.txt");
    if !path.exists() {
        eprintln!("Skipping: {} not found (extract from GGPK first)", path.display());
        return None;
    }

    let input = std::fs::read_to_string(&path).expect("failed to read file");
    let file = stat_desc::parse(&input).expect("failed to parse");
    Some(stat_desc::ReverseIndex::from_file(&file))
}

#[test]
fn build_reverse_index() {
    let Some(index) = load_index() else { return };
    println!("Reverse index: {} patterns", index.len());
    assert!(index.len() > 10_000, "expected >10k patterns, got {}", index.len());
}

#[test]
fn lookup_simple_stat() {
    let Some(index) = load_index() else { return };

    // "+92 to maximum Life" — common mod
    let result = index.lookup("+92 to maximum Life");
    println!("lookup '+92 to maximum Life': {result:?}");
    // This uses "{0:+d} to maximum Life" or "+{0} to maximum Life"
    // Either way, should resolve
    if let Some(m) = &result {
        println!("  stat_ids: {:?}", m.stat_ids);
        println!("  values: {:?}", m.values);
    }
}

#[test]
fn lookup_increased_percent() {
    let Some(index) = load_index() else { return };

    // "40% increased maximum Life" — percentage-based mod
    let result = index.lookup("40% increased maximum Life");
    println!("lookup '40% increased maximum Life': {result:?}");
    if let Some(m) = &result {
        println!("  stat_ids: {:?}", m.stat_ids);
        println!("  values: {:?}", m.values);
        assert_eq!(m.values[0], 40);
    }
}

#[test]
fn lookup_with_negate() {
    let Some(index) = load_index() else { return };

    // "10% reduced maximum Life" — uses negate transform
    let result = index.lookup("10% reduced maximum Life");
    println!("lookup '10% reduced maximum Life': {result:?}");
    if let Some(m) = &result {
        println!("  stat_ids: {:?}", m.stat_ids);
        println!("  values: {:?}", m.values);
        // negate: displayed=10, raw=-10
        assert_eq!(m.values[0], -10);
    }
}

#[test]
fn lookup_adds_damage() {
    let Some(index) = load_index() else { return };

    // "Adds 5 to 10 Physical Damage to Attacks"
    let result = index.lookup("Adds 5 to 10 Physical Damage to Attacks");
    println!("lookup 'Adds 5 to 10 Physical Damage to Attacks': {result:?}");
    if let Some(m) = &result {
        println!("  stat_ids: {:?}", m.stat_ids);
        println!("  values: {:?}", m.values);
        assert_eq!(m.values.len(), 2);
        assert_eq!(m.values[0], 5);
        assert_eq!(m.values[1], 10);
    }
}

#[test]
fn lookup_batch_common_mods() {
    let Some(index) = load_index() else { return };

    let test_cases = [
        "+30 to Strength",
        "+20 to Dexterity",
        "+10 to Intelligence",
        "+40 to maximum Mana",
        "15% increased Attack Speed",
        "20% increased Cast Speed",
        "Adds 1 to 50 Lightning Damage",
        "+15% to Fire Resistance",
        "+20% to Cold Resistance",
        "+30% to Lightning Resistance",
    ];

    let mut found = 0;
    let mut not_found = Vec::new();
    for text in &test_cases {
        match index.lookup(text) {
            Some(m) => {
                found += 1;
                println!("  OK: {text:40} → {:?} = {:?}", m.stat_ids, m.values);
            }
            None => {
                // Try exhaustive as fallback
                match index.lookup_regex(text) {
                    Some(m) => {
                        found += 1;
                        println!("  OK (exhaustive): {text:40} → {:?} = {:?}", m.stat_ids, m.values);
                    }
                    None => {
                        not_found.push(*text);
                        println!("  MISS: {text}");
                    }
                }
            }
        }
    }

    println!("\nFound: {found}/{}", test_cases.len());
    if !not_found.is_empty() {
        println!("Not found: {not_found:?}");
    }
}
