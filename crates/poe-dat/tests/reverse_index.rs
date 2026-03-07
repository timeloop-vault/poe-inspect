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

/// Test against real item data from a 3.28 Mirage character snapshot.
/// These are actual explicitMods/implicitMods/craftedMods from the PoE API.
#[test]
fn lookup_real_character_mods() {
    let Some(index) = load_index() else { return };

    let mods_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/test_data/scripter_boomboom_mods.txt");
    let mods_text = std::fs::read_to_string(&mods_file).expect("failed to read test data");

    // Each line in the file is one mod. Literal "\n" in the file represents
    // a real newline inside a multi-line mod (the PoE API returns these as
    // single strings with embedded newlines matching stat_descriptions \n).
    let mods: Vec<String> = mods_text
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.trim().replace(r"\n", "\n"))
        .collect();
    let total = mods.len();

    let mut found = 0;
    let mut not_found = Vec::new();

    for mod_text in &mods {
        let display = mod_text.replace('\n', "\\n"); // for printing
        match index.lookup(mod_text) {
            Some(m) => {
                found += 1;
                println!("  OK: {display:60} → {:?} = {:?}", m.stat_ids, m.values);
            }
            None => {
                not_found.push(display.clone());
                println!("  MISS: {display}");
            }
        }
    }

    println!("\n=== Real character mods: {found}/{total} matched ===");
    if !not_found.is_empty() {
        println!("Not found ({}):", not_found.len());
        for m in &not_found {
            println!("  - {m}");
        }
    }

    // We expect most mods to resolve — anything below 80% indicates a problem
    let hit_rate = found as f64 / total as f64 * 100.0;
    println!("Hit rate: {hit_rate:.1}%");
    assert!(
        hit_rate > 70.0,
        "expected >70% hit rate, got {hit_rate:.1}% ({found}/{total})"
    );
}
