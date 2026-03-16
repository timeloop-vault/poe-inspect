//! Probe GGPK tables to inventory all available data.
//!
//! Run with: cargo test -p poe-dat --test `probe_tables` -- --nocapture
//!
//! Requires all 911 tables extracted to _reference/ggpk-data-3.28/
//! via: cd crates/poe-query && cargo run --bin `extract_dat` -- -p <`poe_path`> -o ../../_reference/ggpk-data-3.28 --all

use poe_dat::dat_reader::DatFile;
use std::path::Path;

fn reference_dir() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../_reference/ggpk-data-3.28"
    ))
}

fn load_dat(name: &str) -> Option<DatFile> {
    let path = reference_dir().join(format!("{name}.datc64"));
    let bytes = std::fs::read(&path).ok()?;
    DatFile::from_bytes(bytes).ok()
}

fn load_tags() -> Vec<String> {
    let Some(dat) = load_dat("tags") else {
        return vec![];
    };
    (0..dat.row_count)
        .filter_map(|i| dat.read_string(i, 0))
        .collect()
}

fn load_item_classes() -> Vec<String> {
    let Some(dat) = load_dat("itemclasses") else {
        return vec![];
    };
    (0..dat.row_count)
        .filter_map(|i| dat.read_string(i, 0))
        .collect()
}

fn load_base_item_names() -> Vec<String> {
    let Some(dat) = load_dat("baseitemtypes") else {
        return vec![];
    };
    // Name is at a different offset — but we know from our extraction it works
    // BaseItemTypes: Id(8), ItemClass(16), Width(4), Height(4), Name(8), ...
    // Name offset = 8 + 16 + 4 + 4 = 32
    (0..dat.row_count)
        .filter_map(|i| dat.read_string(i, 32))
        .collect()
}

// ── Full inventory: row counts for all tables ────────────────────────────────

#[test]
fn inventory_all_tables() {
    let dir = reference_dir();
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("_reference/ggpk-data-3.28 not found")
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "datc64"))
        .collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);

    println!("=== GGPK Table Inventory ({} tables) ===\n", entries.len());
    println!(
        "{:<45} {:>8} {:>6} {:>8}",
        "Table", "Bytes", "Rows", "RowSize"
    );
    println!("{}", "-".repeat(75));

    for entry in &entries {
        let name = entry.file_name().to_string_lossy().replace(".datc64", "");
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

        if let Ok(bytes) = std::fs::read(entry.path()) {
            if let Ok(dat) = DatFile::from_bytes(bytes) {
                println!(
                    "{name:<45} {size:>8} {rows:>6} {row_size:>8}",
                    rows = dat.row_count,
                    row_size = dat.row_size
                );
            } else {
                println!("{name:<45} {size:>8}  (parse error)");
            }
        }
    }
}

// ── ClientStrings: full dump of ItemDisplay* and ItemPopup* entries ──────────

#[test]
fn inventory_client_strings() {
    let Some(dat) = load_dat("clientstrings") else {
        eprintln!("Skipping: clientstrings not found");
        return;
    };
    println!("=== ClientStrings ===");
    println!("Rows: {}, Row size: {}\n", dat.row_count, dat.row_size);

    // Dump ALL entries matching patterns relevant to item display
    let patterns = [
        "ItemDisplay",
        "ItemPopup",
        "ItemError",
        "Quality",
        "Rarity",
        "Synthesised",
        "Fractured",
        "Corrupted",
        "Mirrored",
        "Foulborn",
        "Mutated",
        "Influence",
        "Searing",
        "Eater",
        "Crucible",
        "Imbued",
        "Unmodifiable",
        "Unidentified",
        "Split",
        "Veiled",
        "Crafted",
        "Enchant",
        "Implicit",
        "Explicit",
        "Prefix",
        "Suffix",
        "Mod",
        "Tier",
        "Socket",
        "Link",
        "Ward",
        "Armour",
        "Evasion",
        "EnergyShield",
        "Block",
        "Weapon",
        "Damage",
        "Attack",
        "Critical",
        "Level",
        "Requirement",
        "Gem",
        "Flask",
        "Map",
        "Heist",
        "Sanctum",
        "Talisman",
        "Scourge",
        "Sentinel",
        "Essence",
        "Expedition",
    ];

    let mut results: Vec<(String, String)> = Vec::new();

    for i in 0..dat.row_count {
        if let Some(id) = dat.read_string(i, 0) {
            let matches = patterns.iter().any(|p| id.contains(p));
            if matches {
                let text = dat.read_string(i, 8).unwrap_or_default();
                // Skip very long entries and art paths
                if text.len() < 300 && !text.contains(".png") && !text.contains(".tgt") {
                    results.push((id, text));
                }
            }
        }
    }

    // Sort by ID for easy reading
    results.sort_by(|a, b| a.0.cmp(&b.0));

    println!("Found {} relevant entries:\n", results.len());
    for (id, text) in &results {
        println!("  {id:<55} = \"{text}\"");
    }
}

// ── InfluenceTags: complete dump ─────────────────────────────────────────────

#[test]
fn inventory_influence_tags() {
    let Some(dat) = load_dat("influencetags") else {
        eprintln!("Skipping: influencetags not found");
        return;
    };
    let tags = load_tags();
    let item_classes = load_item_classes();

    println!("=== InfluenceTags ===");
    println!("Rows: {}, Row size: {}\n", dat.row_count, dat.row_size);

    for i in 0..dat.row_count {
        let ic_idx = dat.read_fk(i, 0).unwrap_or(u64::MAX);
        let influence = dat.read_u32(i, 16).unwrap_or(u32::MAX);
        let tag_idx = dat.read_fk(i, 20).unwrap_or(u64::MAX);

        let ic_name = item_classes
            .get(ic_idx as usize)
            .map_or("?", String::as_str);
        let tag_name = tags.get(tag_idx as usize).map_or("?", String::as_str);
        let influence_name = match influence {
            0 => "Shaper",
            1 => "Elder",
            2 => "Crusader",
            3 => "Hunter",
            4 => "Redeemer",
            5 => "Warlord",
            6 => "None",
            _ => "Unknown",
        };

        println!("  {ic_name:25} + {influence_name:12} → {tag_name}");
    }
}

// ── ArmourTypes: base defence values ─────────────────────────────────────────

#[test]
#[allow(clippy::similar_names)] // ev_min/es_min are domain abbreviations (evasion/energy shield)
fn inventory_armour_types() {
    let Some(dat) = load_dat("armourtypes") else {
        eprintln!("Skipping: armourtypes not found");
        return;
    };
    let base_names = load_base_item_names();

    println!("=== ArmourTypes ===");
    println!("Rows: {}, Row size: {}\n", dat.row_count, dat.row_size);
    println!(
        "{:<30} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "Base", "AR min", "AR max", "EV min", "EV max", "ES min", "ES max"
    );

    for i in 0..dat.row_count.min(30) {
        let base_idx = dat.read_fk(i, 0).unwrap_or(u64::MAX);
        let ar_min = dat.read_i32(i, 16).unwrap_or(0);
        let ar_max = dat.read_i32(i, 20).unwrap_or(0);
        let ev_min = dat.read_i32(i, 24).unwrap_or(0);
        let ev_max = dat.read_i32(i, 28).unwrap_or(0);
        let es_min = dat.read_i32(i, 32).unwrap_or(0);
        let es_max = dat.read_i32(i, 36).unwrap_or(0);

        let name = base_names
            .get(base_idx as usize)
            .map_or("?", String::as_str);

        if ar_max > 0 || ev_max > 0 || es_max > 0 {
            println!(
                "  {name:<30} {ar_min:>8} {ar_max:>8} {ev_min:>8} {ev_max:>8} {es_min:>8} {es_max:>8}"
            );
        }
    }
}

// ── WeaponTypes: base weapon stats ───────────────────────────────────────────

#[test]
fn inventory_weapon_types() {
    let Some(dat) = load_dat("weapontypes") else {
        eprintln!("Skipping: weapontypes not found");
        return;
    };
    let base_names = load_base_item_names();

    println!("=== WeaponTypes ===");
    println!("Rows: {}, Row size: {}\n", dat.row_count, dat.row_size);
    println!(
        "{:<30} {:>8} {:>8} {:>8} {:>8}",
        "Base", "Crit", "Speed", "DmgMin", "DmgMax"
    );

    for i in 0..dat.row_count.min(30) {
        let base_idx = dat.read_fk(i, 0).unwrap_or(u64::MAX);
        let crit = dat.read_i32(i, 16).unwrap_or(0);
        let speed = dat.read_i32(i, 20).unwrap_or(0);
        let dmg_min = dat.read_i32(i, 24).unwrap_or(0);
        let dmg_max = dat.read_i32(i, 28).unwrap_or(0);

        let name = base_names
            .get(base_idx as usize)
            .map_or("?", String::as_str);
        let aps = if speed > 0 {
            1000.0 / f64::from(speed)
        } else {
            0.0
        };

        if dmg_max > 0 {
            println!("  {name:<30} {crit:>8} {speed:>5}({aps:.2}) {dmg_min:>8} {dmg_max:>8}");
        }
    }
}

// ── ItemClasses: capability flags ────────────────────────────────────────────

#[test]
fn inventory_item_class_flags() {
    let Some(dat) = load_dat("itemclasses") else {
        eprintln!("Skipping: itemclasses not found");
        return;
    };
    println!("=== ItemClasses (capability flags) ===");
    println!("Rows: {}, Row size: {}\n", dat.row_count, dat.row_size);

    // Schema: Id(8), Name(8), Category(16), RemovedIfLeavesArea(1),
    //   _(list=16), IdentifyAchievements(list=16), AllocateToMapOwner(1), AlwaysAllocate(1),
    //   CanHaveVeiledMods(1), PickedUpQuest(16), _(4), AlwaysShow(1),
    //   CanBeCorrupted(1), CanHaveIncubators(1), CanHaveInfluence(1),
    //   CanBeDoubleCorrupted(1), CanHaveAspects(1), CanTransferSkin(1),
    //   ItemStance(?), CanScourge(1), CanUpgradeRarity(1), _(1), _(1),
    //   MaxInventoryDimensions(list=16), Unmodifiable(1), CanBeFractured(1), ...

    // Let's compute offsets:
    // 0: Id (8)
    // 8: Name (8)
    // 16: Category FK (16)
    // 32: RemovedIfLeavesArea (1)
    // 33: _ list (16)
    // 49: IdentifyAchievements list (16)
    // 65: AllocateToMapOwner (1)
    // 66: AlwaysAllocate (1)
    // 67: CanHaveVeiledMods (1)
    // 68: PickedUpQuest FK (16) — or rid (8)?
    // Let's try both and see which gives sensible bool values

    // Try PickedUpQuest as FK (16 bytes):
    // 84: _(i32=4)
    // 88: AlwaysShow (1)
    // 89: CanBeCorrupted (1)
    // 90: CanHaveIncubators (1)
    // 91: CanHaveInfluence (1)
    // 92: CanBeDoubleCorrupted (1)
    // 93: CanHaveAspects (1)
    // 94: CanTransferSkin (1)

    // But PickedUpQuest might be rid (8 bytes):
    // 76: _(i32=4)
    // 80: AlwaysShow (1)
    // 81: CanBeCorrupted (1)
    // 82: CanHaveIncubators (1)
    // 83: CanHaveInfluence (1)
    // ...

    // Let's just try different offsets and find the one that makes sense
    // We know "Body Armour" can be corrupted, influenced, fractured
    // We know "Stackable Currency" cannot

    println!("Probing bool fields at various offsets...\n");
    println!(
        "{:<25} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}",
        "Class", "veiled", "corr", "incub", "infl", "dblcor", "fract"
    );

    // Try with PickedUpQuest as FK(16): bools at 88-94
    // Try with PickedUpQuest as rid(8): bools at 80-86
    // The QuestFlags type might be an enum (4 bytes) too

    // Let's try multiple candidate offsets for CanBeCorrupted
    // and check which gives consistent results
    for i in 0..dat.row_count.min(30) {
        let id = dat.read_string(i, 0).unwrap_or_default();
        let veiled = dat.read_bool(i, 67).unwrap_or(false);

        // Try offset 89 for CanBeCorrupted (assuming FK16 for PickedUpQuest)
        let corr_89 = dat.read_bool(i, 89).unwrap_or(false);
        let incub_89 = dat.read_bool(i, 90).unwrap_or(false);
        let infl_89 = dat.read_bool(i, 91).unwrap_or(false);
        let dblcor_89 = dat.read_bool(i, 92).unwrap_or(false);

        // Look for CanBeFractured further down
        // After CanTransferSkin(94), ItemStance(?), CanScourge(1), CanUpgradeRarity(1), _(1), _(1),
        // MaxInventoryDimensions(list=16), Unmodifiable(1), CanBeFractured(1)
        // ItemStance could be enum(4), FK(16), or rid(8)
        // Let's just scan a range

        // Only print interesting classes
        if [
            "Body Armour",
            "Boots",
            "Ring",
            "Amulet",
            "Wand",
            "Map",
            "Stackable Currency",
            "Skill Gem",
            "Jewel",
            "Flask",
            "Shield",
            "Divination Card",
            "Support Gem",
        ]
        .contains(&id.as_str())
        {
            println!(
                "  {id:<25} {veiled:>6} {corr_89:>6} {incub_89:>6} {infl_89:>6} {dblcor_89:>6}"
            );
        }
    }

    // Also dump raw bytes around the bool region for Body Armour (row 0 probably)
    println!("\nRaw bytes for first class at offsets 65-100:");
    for offset in 65..dat.row_size.min(120) {
        let val = dat.read_bool(0, offset).unwrap_or(false);
        if val {
            print!(" {offset}:T");
        }
    }
    println!();

    println!("\nRaw bytes for 'Stackable Currency' (should have mostly false):");
    // Find Stackable Currency row
    for i in 0..dat.row_count {
        if dat.read_string(i, 0).as_deref() == Some("Stackable Currency") {
            for offset in 65..dat.row_size.min(120) {
                let val = dat.read_bool(i, offset).unwrap_or(false);
                if val {
                    print!(" {offset}:T");
                }
            }
            println!();
            break;
        }
    }
}

// ── InfluenceExalts: what exalts exist per influence ─────────────────────────

#[test]
fn inventory_influence_exalts() {
    let Some(dat) = load_dat("influenceexalts") else {
        eprintln!("Skipping: influenceexalts not found");
        return;
    };
    let base_names = load_base_item_names();

    println!("=== InfluenceExalts ===");
    println!("Rows: {}, Row size: {}\n", dat.row_count, dat.row_size);

    for i in 0..dat.row_count {
        let base_idx = dat.read_fk(i, 0).unwrap_or(u64::MAX);
        let influence = dat.read_u32(i, 16).unwrap_or(u32::MAX);
        let name = base_names
            .get(base_idx as usize)
            .map_or("?", String::as_str);
        let influence_name = match influence {
            0 => "Shaper",
            1 => "Elder",
            2 => "Crusader",
            3 => "Hunter",
            4 => "Redeemer",
            5 => "Warlord",
            _ => "Unknown",
        };
        println!("  {name:<40} → {influence_name}");
    }
}

// ── CraftingBenchOptions: sample of bench crafts ─────────────────────────────

#[test]
fn inventory_crafting_bench_sample() {
    let Some(dat) = load_dat("craftingbenchoptions") else {
        eprintln!("Skipping: craftingbenchoptions not found");
        return;
    };
    println!("=== CraftingBenchOptions ===");
    println!("Rows: {}, Row size: {}\n", dat.row_count, dat.row_size);
    println!("(Full schema exploration deferred — just row count for now)");
}

// ── ComponentAttributeRequirements: str/dex/int per base ─────────────────────

#[test]
fn inventory_attribute_requirements() {
    let Some(dat) = load_dat("componentattributerequirements") else {
        eprintln!("Skipping: componentattributerequirements not found");
        return;
    };
    let base_names = load_base_item_names();

    println!("=== ComponentAttributeRequirements ===");
    println!("Rows: {}, Row size: {}\n", dat.row_count, dat.row_size);
    // Schema: BaseItemTypesKey(FK=16), ReqStr(i32), ReqDex(i32), ReqInt(i32)

    println!("{:<30} {:>6} {:>6} {:>6}", "Base", "Str", "Dex", "Int");
    for i in 0..dat.row_count.min(20) {
        let base_idx = dat.read_fk(i, 0).unwrap_or(u64::MAX);
        let str_req = dat.read_i32(i, 16).unwrap_or(0);
        let dex_req = dat.read_i32(i, 20).unwrap_or(0);
        let int_req = dat.read_i32(i, 24).unwrap_or(0);
        let name = base_names
            .get(base_idx as usize)
            .map_or("?", String::as_str);

        if str_req > 0 || dex_req > 0 || int_req > 0 {
            println!("  {name:<30} {str_req:>6} {dex_req:>6} {int_req:>6}");
        }
    }
}

// ── ShieldTypes: base shield values ──────────────────────────────────────────

#[test]
fn inventory_shield_types() {
    let Some(dat) = load_dat("shieldtypes") else {
        eprintln!("Skipping: shieldtypes not found");
        return;
    };
    let base_names = load_base_item_names();

    println!("=== ShieldTypes ===");
    println!("Rows: {}, Row size: {}\n", dat.row_count, dat.row_size);
    // Schema: BaseItemTypesKey(FK=16), Block(i32)

    for i in 0..dat.row_count.min(20) {
        let base_idx = dat.read_fk(i, 0).unwrap_or(u64::MAX);
        let block = dat.read_i32(i, 16).unwrap_or(0);
        let name = base_names
            .get(base_idx as usize)
            .map_or("?", String::as_str);
        println!("  {name:<30} block={block}");
    }
}
