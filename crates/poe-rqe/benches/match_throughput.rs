//! Benchmark: brute-force vs decision DAG matching throughput.
//!
//! Run with: `cargo bench -p poe-rqe`
//!
//! Two benchmark suites:
//! 1. **Synthetic** — generated queries with realistic archetypes
//! 2. **Fixtures** — real PoE items parsed from `fixtures/items/*.txt`

use std::collections::HashMap;
use std::hint::black_box;
use std::time::Instant;

use poe_rqe::eval::Entry;
use poe_rqe::index::IndexedStore;
use poe_rqe::predicate::Condition;
use poe_rqe::store::QueryStore;

// ---------------------------------------------------------------------------
// Fixture parser — lightweight Ctrl+Alt+C text → Entry converter
// ---------------------------------------------------------------------------

/// Parse a Ctrl+Alt+C fixture file into a flat Entry for RQE matching.
/// Returns None for items that don't have meaningful stats (currency, fragments, etc.)
fn parse_fixture_to_entry(text: &str) -> Option<Entry> {
    let mut map: HashMap<String, serde_json::Value> = HashMap::new();
    let sections: Vec<&str> = text.split("--------\n").collect();

    if sections.is_empty() {
        return None;
    }

    // --- Header section (always first) ---
    let header_lines: Vec<&str> = sections[0].lines().collect();
    let mut item_class = None;
    let mut rarity = None;

    for line in &header_lines {
        if let Some(rest) = line.strip_prefix("Item Class: ") {
            item_class = Some(rest.trim().to_owned());
        }
        if let Some(rest) = line.strip_prefix("Rarity: ") {
            rarity = Some(rest.trim().to_owned());
        }
    }

    let item_class = item_class?;
    let rarity = rarity?;

    // Skip non-equipment items
    let dominated_categories = [
        "Currency",
        "Stackable Currency",
        "Divination Cards",
        "Map Fragments",
        "Hideout Doodads",
        "Microtransactions",
        "Quest Items",
        "Labyrinth",
    ];
    if dominated_categories.iter().any(|c| item_class.contains(c)) {
        return None;
    }

    map.insert("item_class".into(), serde_json::json!(item_class));
    map.insert("rarity".into(), serde_json::json!(rarity));

    // Rarity classification for matching
    let is_unique = rarity == "Unique";
    map.insert(
        "rarity_class".into(),
        serde_json::json!(if is_unique { "Unique" } else { "Non-Unique" }),
    );

    // Name and base type from remaining header lines (after Item Class + Rarity)
    let name_lines: Vec<&&str> = header_lines
        .iter()
        .filter(|l| !l.starts_with("Item Class:") && !l.starts_with("Rarity:") && !l.is_empty())
        .collect();

    if name_lines.len() >= 2 {
        // Rare/Unique: name + base_type
        map.insert("name".into(), serde_json::json!(name_lines[0]));
        map.insert("base_type".into(), serde_json::json!(name_lines[1]));
    } else if name_lines.len() == 1 {
        // Normal/Magic: just base_type (or name for magic)
        map.insert("base_type".into(), serde_json::json!(name_lines[0]));
    }

    // --- Parse remaining sections ---
    for section in &sections[1..] {
        let lines: Vec<&str> = section.lines().collect();
        if lines.is_empty() {
            continue;
        }

        // Item Level
        for line in &lines {
            if let Some(rest) = line.strip_prefix("Item Level: ") {
                if let Ok(ilvl) = rest.trim().parse::<i64>() {
                    map.insert("item_level".into(), serde_json::json!(ilvl));
                }
            }
        }

        // Sockets
        for line in &lines {
            if let Some(rest) = line.strip_prefix("Sockets: ") {
                let socket_count = rest.chars().filter(|c| c.is_alphabetic()).count();
                let link_groups: Vec<&str> = rest.split(' ').collect();
                let max_link = link_groups
                    .iter()
                    .map(|g| g.chars().filter(|c| c.is_alphabetic()).count())
                    .max()
                    .unwrap_or(0);
                map.insert(
                    "socket_count".into(),
                    serde_json::json!(socket_count as i64),
                );
                map.insert("max_link".into(), serde_json::json!(max_link as i64));
            }
        }

        // Mod sections — lines starting with { ... } are mod headers
        let mut current_source = "explicit"; // default
        for line in &lines {
            let trimmed = line.trim();

            // Mod header: { Prefix Modifier ... } or { Implicit Modifier ... }
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                if trimmed.contains("Implicit") {
                    current_source = "implicit";
                } else if trimmed.contains("Enchant") || trimmed.contains("enchant") {
                    current_source = "enchant";
                } else if trimmed.contains("Crafted") {
                    current_source = "crafted";
                } else {
                    current_source = "explicit";
                }
                continue;
            }

            // Skip reminder text (parenthesized)
            if trimmed.starts_with('(') && trimmed.ends_with(')') {
                continue;
            }

            // Skip non-stat lines
            if trimmed.is_empty()
                || trimmed.starts_with("Requirements:")
                || trimmed.starts_with("Level:")
                || trimmed.starts_with("Str:")
                || trimmed.starts_with("Dex:")
                || trimmed.starts_with("Int:")
                || trimmed.starts_with("Quality:")
                || trimmed.starts_with("Sockets:")
                || trimmed.starts_with("Item Level:")
                || trimmed.starts_with("Place into")
                || trimmed.starts_with("Right click")
                || trimmed.ends_with("Item") // "Shaper Item", "Elder Item", etc.
                || trimmed.starts_with("In a blaze")
            // flavor text start
            {
                continue;
            }

            // Try to extract stat value and template from this line
            if let Some((template, value)) = extract_stat_template(trimmed) {
                let key = format!("{current_source}.{template}");
                map.insert(key, serde_json::json!(value));
            }
        }
    }

    // Only return entries that have at least one stat
    let has_stats = map.keys().any(|k| k.contains('.'));
    if !has_stats {
        return None;
    }

    let json = serde_json::to_string(&map).ok()?;
    serde_json::from_str(&json).ok()
}

/// Extract a stat template and numeric value from a stat line.
///
/// Examples:
///   "+87(80-130) to Accuracy Rating" → ("+# to Accuracy Rating", 87)
///   "9(8-12)% increased Spell Damage" → ("#% increased Spell Damage", 9)
///   "+1 to Level of all Physical Spell Skill Gems" → ("+# to Level of all Physical Spell Skill Gems", 1)
///   "Regenerate 41.1(32.1-48) Life per second" → ("Regenerate # Life per second", 41)
///   "30% increased Movement Speed" → ("#% increased Movement Speed", 30)
fn extract_stat_template(line: &str) -> Option<(String, i64)> {
    // Strip common suffixes that aren't part of the stat template
    let line = line
        .trim_end_matches(" (implicit)")
        .trim_end_matches(" (crafted)")
        .trim_end_matches(" (enchant)");

    // Find all numbers (with optional leading +/-, optional decimal, optional range)
    // Pattern: optional sign, digits, optional decimal, optional (min-max) range
    let mut template = String::with_capacity(line.len());
    let mut first_value: Option<i64> = None;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check if we're at the start of a number
        let is_sign = (chars[i] == '+' || chars[i] == '-')
            && i + 1 < chars.len()
            && chars[i + 1].is_ascii_digit();
        let is_digit = chars[i].is_ascii_digit();

        if is_sign || is_digit {
            let num_start = i;
            let sign = if chars[i] == '-' {
                i += 1;
                -1i64
            } else if chars[i] == '+' {
                template.push('+');
                i += 1;
                1i64
            } else {
                1i64
            };

            // Read digits (integer part)
            let digit_start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }

            if i == digit_start {
                // Sign with no digits following — just push the sign
                if num_start < chars.len() {
                    template.push(chars[num_start]);
                }
                continue;
            }

            let int_str: String = chars[digit_start..i].iter().collect();
            let int_val: i64 = int_str.parse().unwrap_or(0) * sign;

            // Skip decimal part if present
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }

            // Skip range annotation (min-max) if present
            if i < chars.len() && chars[i] == '(' {
                let paren_start = i;
                i += 1;
                // Find matching )
                while i < chars.len() && chars[i] != ')' {
                    i += 1;
                }
                if i < chars.len() {
                    i += 1; // skip ')'
                }
                // Verify this looks like a range (contains digits and -)
                let paren_content: String =
                    chars[paren_start + 1..i.saturating_sub(1)].iter().collect();
                if !paren_content.chars().any(|c| c.is_ascii_digit()) {
                    // Not a range — put back
                    i = paren_start;
                }
            }

            template.push('#');
            if first_value.is_none() {
                first_value = Some(int_val);
            }
        } else {
            template.push(chars[i]);
            i += 1;
        }
    }

    let value = first_value?;
    let template = template.trim().to_owned();
    if template.is_empty() || template == "#" {
        return None;
    }
    Some((template, value))
}

/// Load all fixture files that can be parsed into Entry.
fn load_fixture_entries() -> Vec<Entry> {
    let fixtures_dir = format!(
        "{}/fixtures/items",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
    );

    let mut entries = Vec::new();
    if let Ok(dir) = std::fs::read_dir(&fixtures_dir) {
        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "txt") {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    if let Some(entry) = parse_fixture_to_entry(&text) {
                        entries.push(entry);
                    }
                }
            }
        }
    }
    entries
}

/// Collect all stat keys that appear in a set of entries.
fn collect_stat_keys(_entries: &[Entry]) -> Vec<String> {
    let mut keys = std::collections::HashSet::new();

    // Collect stat keys by re-parsing fixtures and tracking emitted keys.
    let fixtures_dir = format!(
        "{}/fixtures/items",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
    );

    if let Ok(dir) = std::fs::read_dir(&fixtures_dir) {
        for dir_entry in dir.flatten() {
            let path = dir_entry.path();
            if path.extension().is_some_and(|e| e == "txt") {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    let sections: Vec<&str> = text.split("--------\n").collect();
                    for section in &sections {
                        for line in section.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with('{') || trimmed.starts_with('(') {
                                continue;
                            }
                            if let Some((template, _)) = extract_stat_template(trimmed) {
                                // Determine source from context (simplified)
                                keys.insert(format!("explicit.{template}"));
                            }
                        }
                    }
                }
            }
        }
    }

    keys.into_iter().collect()
}

// ---------------------------------------------------------------------------
// Deterministic PRNG — simple xorshift for reproducible benchmarks
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn next_usize(&mut self, max: usize) -> usize {
        (self.next_u64() % max as u64) as usize
    }

    fn next_range(&mut self, min: i64, max: i64) -> i64 {
        min + (self.next_u64() % (max - min + 1) as u64) as i64
    }
}

// ---------------------------------------------------------------------------
// Query generators using real stat keys
// ---------------------------------------------------------------------------

const ITEM_CLASSES: &[&str] = &[
    "Boots",
    "Gloves",
    "Helmets",
    "Body Armours",
    "Rings",
    "Amulets",
    "Belts",
    "Wands",
    "Daggers",
    "One Hand Swords",
    "Two Hand Swords",
    "Bows",
    "Quivers",
    "Shields",
    "Sceptres",
    "Staves",
    "Jewels",
];

fn category_json(cat: &str) -> String {
    format!(r#"{{"key": "item_class", "value": "{cat}", "type": "string", "typeOptions": null}}"#)
}

fn rarity_class_json(rarity: &str) -> String {
    format!(
        r#"{{"key": "rarity_class", "value": "{rarity}", "type": "string", "typeOptions": null}}"#
    )
}

fn stat_threshold_json(stat: &str, value: i64, op: &str) -> String {
    // Escape any quotes in stat keys
    let escaped = stat.replace('"', r#"\""#);
    format!(
        r#"{{"key": "{escaped}", "value": {value}, "type": "integer", "typeOptions": {{"operator": "{op}"}}}}"#
    )
}

fn stat_range_json(stat: &str, min: i64, max: i64) -> String {
    let lo = stat_threshold_json(stat, min, "<");
    let hi = stat_threshold_json(stat, max, ">");
    format!(
        r#"{{"key": "list", "value": [{lo},{hi}], "type": "list", "typeOptions": {{"operator": "and"}}}}"#
    )
}

fn not_list_json(inner: &[String]) -> String {
    let joined = inner.join(",");
    format!(
        r#"{{"key": "list", "value": [{joined}], "type": "list", "typeOptions": {{"operator": "not"}}}}"#
    )
}

fn boolean_json(key: &str, value: bool) -> String {
    format!(r#"{{"key": "{key}", "value": {value}, "type": "boolean", "typeOptions": null}}"#)
}

/// Pick N unique items from a slice.
fn pick_unique<'a>(rng: &mut Rng, items: &'a [String], count: usize) -> Vec<&'a String> {
    let count = count.min(items.len());
    let mut chosen = Vec::with_capacity(count);
    let mut indices = Vec::with_capacity(count);
    while chosen.len() < count {
        let idx = rng.next_usize(items.len());
        if !indices.contains(&idx) {
            indices.push(idx);
            chosen.push(&items[idx]);
        }
    }
    chosen
}

/// Generate a query using real stat keys extracted from fixtures.
fn generate_query_from_real_stats(rng: &mut Rng, stat_keys: &[String]) -> Vec<Condition> {
    if stat_keys.is_empty() {
        return Vec::new();
    }

    let mut parts: Vec<String> = Vec::new();

    // 80% have a category filter
    if rng.next_usize(100) < 80 {
        let cat = ITEM_CLASSES[rng.next_usize(ITEM_CLASSES.len())];
        parts.push(category_json(cat));
    }

    // 90% have a rarity filter
    if rng.next_usize(100) < 90 {
        parts.push(rarity_class_json("Non-Unique"));
    }

    // Archetype determines stat count and complexity
    match rng.next_usize(100) {
        0..30 => {
            // Simple: 1-2 stat thresholds
            let count = 1 + rng.next_usize(2);
            for stat in pick_unique(rng, stat_keys, count) {
                let threshold = rng.next_range(10, 60);
                parts.push(stat_threshold_json(stat, threshold, "<"));
            }
        }
        30..60 => {
            // Moderate: 2-4 stat ranges
            let count = 2 + rng.next_usize(3);
            for stat in pick_unique(rng, stat_keys, count) {
                let min = rng.next_range(5, 30);
                let max = min + rng.next_range(20, 50);
                parts.push(stat_range_json(stat, min, max));
            }
        }
        60..80 => {
            // Complex: 3-5 stats + boolean
            let count = 3 + rng.next_usize(3);
            for stat in pick_unique(rng, stat_keys, count) {
                let min = rng.next_range(10, 40);
                let max = min + rng.next_range(25, 60);
                parts.push(stat_range_json(stat, min, max));
            }
            if rng.next_usize(2) == 0 {
                parts.push(boolean_json("corrupted", false));
            }
        }
        80..90 => {
            // NOT pattern: want stats, exclude bad stats
            let count = 2 + rng.next_usize(2);
            for stat in pick_unique(rng, stat_keys, count) {
                let threshold = rng.next_range(15, 50);
                parts.push(stat_threshold_json(stat, threshold, "<"));
            }
            // NOT condition
            let bad_count = 1 + rng.next_usize(2);
            let bad_stats = pick_unique(rng, stat_keys, bad_count);
            let not_inner: Vec<String> = bad_stats
                .iter()
                .map(|s| stat_threshold_json(s, rng.next_range(5, 20), "<"))
                .collect();
            parts.push(not_list_json(&not_inner));
        }
        _ => {
            // Broad: just 1-2 stats, no category
            parts.clear();
            parts.push(rarity_class_json("Non-Unique"));
            let count = 1 + rng.next_usize(2);
            for stat in pick_unique(rng, stat_keys, count) {
                let threshold = rng.next_range(20, 70);
                parts.push(stat_threshold_json(stat, threshold, "<"));
            }
        }
    }

    let json = format!("[{}]", parts.join(","));
    serde_json::from_str(&json).unwrap()
}

// ---------------------------------------------------------------------------
// Benchmark runners
// ---------------------------------------------------------------------------

fn bench_brute_force(queries: &[Vec<Condition>], entries: &[Entry], iterations: u64) {
    let mut store = QueryStore::new();
    for rq in queries {
        store.add(rq.clone(), vec![]);
    }

    let start = Instant::now();
    for _ in 0..iterations {
        for entry in entries {
            black_box(store.match_item(entry));
        }
    }
    let elapsed = start.elapsed();

    let total_matches = entries.len() as u64 * iterations;
    let us_per_match = (elapsed.as_secs_f64() * 1_000_000.0) / total_matches as f64;
    let total_hits: usize = entries.iter().map(|e| store.match_item(e).len()).sum();

    println!(
        "  brute-force | {:>7} queries | {:>9.1}μs/match | {} total hits across {} items",
        queries.len(),
        us_per_match,
        total_hits,
        entries.len(),
    );
}

fn bench_indexed(queries: &[Vec<Condition>], entries: &[Entry], iterations: u64) {
    let mut store = IndexedStore::new();
    for rq in queries {
        store.add(rq.clone(), vec![]);
    }

    let start = Instant::now();
    for _ in 0..iterations {
        for entry in entries {
            black_box(store.match_item(entry));
        }
    }
    let elapsed = start.elapsed();

    let total_matches = entries.len() as u64 * iterations;
    let us_per_match = (elapsed.as_secs_f64() * 1_000_000.0) / total_matches as f64;
    let total_hits: usize = entries.iter().map(|e| store.match_item(e).len()).sum();

    println!(
        "  indexed     | {:>7} queries | {:>9.1}μs/match | {} total hits across {} items | {} nodes, depth {}",
        queries.len(),
        us_per_match,
        total_hits,
        entries.len(),
        store.node_count(),
        store.max_depth(),
    );
}

fn iterations_for(query_count: usize) -> u64 {
    match query_count {
        n if n <= 1_000 => 1_000,
        n if n <= 10_000 => 100,
        n if n <= 100_000 => 10,
        _ => 2,
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    // Load real item fixtures
    let fixture_entries = load_fixture_entries();
    let stat_keys = collect_stat_keys(&fixture_entries);

    println!(
        "Loaded {} real items from fixtures, found {} unique stat templates",
        fixture_entries.len(),
        stat_keys.len(),
    );
    println!();

    if fixture_entries.is_empty() || stat_keys.is_empty() {
        println!("ERROR: No fixtures found. Run from workspace root.");
        return;
    }

    let counts = &[100, 1_000, 10_000, 50_000, 100_000, 500_000, 1_000_000];

    println!("=== REAL FIXTURES + REAL STAT KEYS ===");
    println!("  Queries use stat templates extracted from fixture items.");
    println!("  Items are real Ctrl+Alt+C copies from PoE.");
    println!();

    for &count in counts {
        let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);
        let queries: Vec<Vec<Condition>> = (0..count)
            .map(|_| generate_query_from_real_stats(&mut rng, &stat_keys))
            .collect();

        let iters = iterations_for(count);
        bench_brute_force(&queries, &fixture_entries, iters);
        bench_indexed(&queries, &fixture_entries, iters);
        println!();
    }
}
