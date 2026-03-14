//! Benchmark: brute-force vs decision DAG matching throughput.
//!
//! Run with: `cargo bench -p poe-rqe`
//!
//! Two benchmark suites:
//! 1. **Synthetic** — uniform queries, maximum sharing (best-case for DAG)
//! 2. **Realistic** — models real PoE player behavior with diverse query shapes,
//!    stat distributions, NOT/COUNT/boolean conditions, and varying complexity

use std::hint::black_box;
use std::time::Instant;

use poe_rqe::eval::Entry;
use poe_rqe::index::IndexedStore;
use poe_rqe::predicate::Condition;
use poe_rqe::store::QueryStore;

// ---------------------------------------------------------------------------
// Shared data pools
// ---------------------------------------------------------------------------

const ITEM_CATEGORIES: &[&str] = &[
    "Crimson Jewel",
    "Cobalt Jewel",
    "Viridian Jewel",
    "Boots",
    "Gloves",
    "Helmet",
    "Body Armour",
    "Ring",
    "Amulet",
    "Belt",
    "Wand",
    "Dagger",
    "Sword",
    "Axe",
    "Mace",
    "Staff",
    "Bow",
    "Quiver",
    "Shield",
    "Flask",
];

/// Weighted category distribution — armor/weapons/jewelry are far more popular
/// than flasks/quivers. Each entry is (category_index, relative_weight).
const CATEGORY_WEIGHTS: &[(usize, u32)] = &[
    (0, 3),  // Crimson Jewel
    (1, 3),  // Cobalt Jewel
    (2, 3),  // Viridian Jewel
    (3, 10), // Boots
    (4, 8),  // Gloves
    (5, 10), // Helmet
    (6, 15), // Body Armour
    (7, 12), // Ring
    (8, 10), // Amulet
    (9, 8),  // Belt
    (10, 5), // Wand
    (11, 4), // Dagger
    (12, 6), // Sword
    (13, 3), // Axe
    (14, 3), // Mace
    (15, 2), // Staff
    (16, 5), // Bow
    (17, 1), // Quiver
    (18, 6), // Shield
    (19, 1), // Flask
];

/// 40 realistic stat template strings — 4x the original pool
const STAT_POOL: &[&str] = &[
    // Defences
    "+# to maximum Life",
    "+# to maximum Mana",
    "+# to maximum Energy Shield",
    "% increased maximum Life",
    "% increased Armour",
    "% increased Evasion Rating",
    "+# to Armour",
    "+# to Evasion Rating",
    // Resistances
    "+#% to Fire Resistance",
    "+#% to Cold Resistance",
    "+#% to Lightning Resistance",
    "+#% to Chaos Resistance",
    "+#% to all Elemental Resistances",
    "+#% to Fire and Cold Resistances",
    "+#% to Fire and Lightning Resistances",
    "+#% to Cold and Lightning Resistances",
    // Offence — physical
    "% increased Physical Damage",
    "Adds # to # Physical Damage",
    "% increased Attack Speed",
    "% increased Critical Strike Chance",
    "+#% to Critical Strike Multiplier",
    "#% increased Accuracy Rating",
    // Offence — elemental
    "Adds # to # Fire Damage",
    "Adds # to # Cold Damage",
    "Adds # to # Lightning Damage",
    "% increased Elemental Damage",
    "% increased Spell Damage",
    "% increased Cast Speed",
    "+# to Level of all Spell Skill Gems",
    // Utility
    "% increased Movement Speed",
    "% increased Rarity of Items found",
    "#% reduced Mana Cost of Skills",
    "+# Mana gained on Kill",
    "+# Life gained on Kill",
    "% increased Stun Duration on Enemies",
    "#% chance to Block",
    "Regenerate # Life per second",
    "Regenerate #% of Life per second",
    "+# to Strength",
    "+# to Dexterity",
    "+# to Intelligence",
    "+# to all Attributes",
];

/// Rarity values (Non-Unique is most common in searches).
const RARITIES: &[&str] = &["Non-Unique", "Rare", "Magic"];

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

    /// Pick from a weighted distribution. Returns the index from the weights table.
    fn weighted_pick(&mut self, weights: &[(usize, u32)]) -> usize {
        let total: u32 = weights.iter().map(|(_, w)| w).sum();
        let mut roll = (self.next_u64() % u64::from(total)) as u32;
        for &(idx, weight) in weights {
            if roll < weight {
                return idx;
            }
            roll -= weight;
        }
        weights.last().unwrap().0
    }
}

// ---------------------------------------------------------------------------
// Realistic query generator
// ---------------------------------------------------------------------------

/// Query archetype — models different player search behaviors
#[derive(Clone, Copy)]
enum Archetype {
    /// Casual shopper: category + rarity + 1-2 loose stat checks
    Simple,
    /// Gearing a build: category + rarity + 3-4 stat requirements
    Moderate,
    /// Min-maxer: category + rarity + 4-6 stats + NOT conditions + booleans
    Complex,
    /// Crafter: category + rarity + NOT(bad mods) + COUNT conditions
    Crafter,
    /// Broad hunter: rarity only (no category) or wildcard category + stats
    Broad,
}

fn pick_archetype(rng: &mut Rng) -> Archetype {
    match rng.next_usize(100) {
        0..30 => Archetype::Simple,
        30..60 => Archetype::Moderate,
        60..80 => Archetype::Complex,
        80..90 => Archetype::Crafter,
        _ => Archetype::Broad,
    }
}

/// Build a condition JSON string for a single stat range check.
fn stat_range_json(stat: &str, min: i64, max: i64) -> String {
    format!(
        r#"{{"key": "list", "value": [
            {{"key": "{stat}", "value": {min}, "type": "integer", "typeOptions": {{"operator": "<"}}}},
            {{"key": "{stat}", "value": {max}, "type": "integer", "typeOptions": {{"operator": ">"}}}}
        ], "type": "list", "typeOptions": {{"operator": "and"}}}}"#
    )
}

/// Build a condition JSON string for a simple integer threshold.
fn stat_threshold_json(stat: &str, value: i64, op: &str) -> String {
    format!(
        r#"{{"key": "{stat}", "value": {value}, "type": "integer", "typeOptions": {{"operator": "{op}"}}}}"#
    )
}

/// Build a NOT list condition.
fn not_list_json(inner: &[String]) -> String {
    let joined = inner.join(",");
    format!(
        r#"{{"key": "list", "value": [{joined}], "type": "list", "typeOptions": {{"operator": "not"}}}}"#
    )
}

/// Build a COUNT list condition.
fn count_list_json(inner: &[String], count: u32) -> String {
    let joined = inner.join(",");
    format!(
        r#"{{"key": "list", "value": [{joined}], "type": "list", "typeOptions": {{"operator": "count", "count": {count}}}}}"#
    )
}

fn category_json(cat: &str) -> String {
    format!(
        r#"{{"key": "item_category", "value": "{cat}", "type": "string", "typeOptions": null}}"#
    )
}

fn rarity_json(rarity: &str) -> String {
    format!(
        r#"{{"key": "item_rarity_2", "value": "{rarity}", "type": "string", "typeOptions": null}}"#
    )
}

fn boolean_json(key: &str, value: bool) -> String {
    format!(
        r#"{{"key": "{key}", "value": {value}, "type": "boolean", "typeOptions": null}}"#
    )
}

fn wildcard_json(key: &str) -> String {
    format!(
        r#"{{"key": "{key}", "value": "_", "type": "string", "typeOptions": null}}"#
    )
}

/// Pick N unique stats from the pool.
fn pick_stats(rng: &mut Rng, count: usize) -> Vec<usize> {
    let mut chosen = Vec::with_capacity(count);
    while chosen.len() < count {
        let idx = rng.next_usize(STAT_POOL.len());
        if !chosen.contains(&idx) {
            chosen.push(idx);
        }
    }
    chosen
}

/// Generate a single realistic query based on archetype.
fn generate_realistic_query(rng: &mut Rng) -> Vec<Condition> {
    let archetype = pick_archetype(rng);
    let mut parts: Vec<String> = Vec::new();

    match archetype {
        Archetype::Simple => {
            // Category + rarity + 1-2 stat thresholds
            let cat_idx = rng.weighted_pick(CATEGORY_WEIGHTS);
            parts.push(category_json(ITEM_CATEGORIES[cat_idx]));
            parts.push(rarity_json(RARITIES[rng.next_usize(RARITIES.len())]));

            let stat_count = 1 + rng.next_usize(2); // 1-2
            for &si in &pick_stats(rng, stat_count) {
                let threshold = rng.next_range(10, 80);
                parts.push(stat_threshold_json(STAT_POOL[si], threshold, "<"));
            }
        }
        Archetype::Moderate => {
            // Category + rarity + 3-4 stat ranges
            let cat_idx = rng.weighted_pick(CATEGORY_WEIGHTS);
            parts.push(category_json(ITEM_CATEGORIES[cat_idx]));
            parts.push(rarity_json("Non-Unique"));

            let stat_count = 3 + rng.next_usize(2); // 3-4
            for &si in &pick_stats(rng, stat_count) {
                let min = rng.next_range(5, 40);
                let max = min + rng.next_range(20, 60);
                parts.push(stat_range_json(STAT_POOL[si], min, max));
            }
        }
        Archetype::Complex => {
            // Category + rarity + 4-6 stats + boolean + NOT
            let cat_idx = rng.weighted_pick(CATEGORY_WEIGHTS);
            parts.push(category_json(ITEM_CATEGORIES[cat_idx]));
            parts.push(rarity_json("Non-Unique"));

            // Boolean condition (corrupted, identified, etc.)
            if rng.next_usize(2) == 0 {
                parts.push(boolean_json("corrupted", false));
            } else {
                parts.push(boolean_json("identified", true));
            }

            // 4-6 stat requirements
            let stat_count = 4 + rng.next_usize(3); // 4-6
            let stats = pick_stats(rng, stat_count);
            for &si in &stats[..stat_count.min(stats.len())] {
                let min = rng.next_range(10, 50);
                let max = min + rng.next_range(30, 80);
                parts.push(stat_range_json(STAT_POOL[si], min, max));
            }

            // NOT condition — exclude a bad mod
            let bad_stat = STAT_POOL[rng.next_usize(STAT_POOL.len())];
            let not_inner = vec![stat_threshold_json(bad_stat, 5, "<")];
            parts.push(not_list_json(&not_inner));
        }
        Archetype::Crafter => {
            // Category + rarity + NOT(bad mods) + COUNT(n of stats)
            let cat_idx = rng.weighted_pick(CATEGORY_WEIGHTS);
            parts.push(category_json(ITEM_CATEGORIES[cat_idx]));
            parts.push(rarity_json("Non-Unique"));

            // NOT: exclude 2 bad mods
            let bad_stats = pick_stats(rng, 2);
            let not_inner: Vec<String> = bad_stats
                .iter()
                .map(|&si| stat_threshold_json(STAT_POOL[si], rng.next_range(5, 20), "<"))
                .collect();
            parts.push(not_list_json(&not_inner));

            // COUNT: at least 2 of these 4 desired stats
            let desired = pick_stats(rng, 4);
            let count_inner: Vec<String> = desired
                .iter()
                .map(|&si| stat_threshold_json(STAT_POOL[si], rng.next_range(20, 60), "<"))
                .collect();
            let need = 1 + rng.next_usize(3) as u32; // COUNT 1-3
            parts.push(count_list_json(&count_inner, need));

            // 1-2 hard requirements
            let hard_count = 1 + rng.next_usize(2);
            for &si in &pick_stats(rng, hard_count) {
                let threshold = rng.next_range(30, 70);
                parts.push(stat_threshold_json(STAT_POOL[si], threshold, "<"));
            }
        }
        Archetype::Broad => {
            // No category or wildcard + rarity + 1-3 stats
            if rng.next_usize(2) == 0 {
                parts.push(wildcard_json("item_category"));
            }
            // else: no category at all
            parts.push(rarity_json("Non-Unique"));

            let stat_count = 1 + rng.next_usize(3); // 1-3
            for &si in &pick_stats(rng, stat_count) {
                let threshold = rng.next_range(20, 80);
                parts.push(stat_threshold_json(STAT_POOL[si], threshold, "<"));
            }
        }
    }

    let json = format!("[{}]", parts.join(","));
    serde_json::from_str(&json).unwrap()
}

/// Generate a realistic item entry with multiple stats.
fn make_realistic_entry(rng: &mut Rng) -> Entry {
    let cat_idx = rng.weighted_pick(CATEGORY_WEIGHTS);
    let category = ITEM_CATEGORIES[cat_idx];

    // Pick 4-8 random stats with realistic values
    let stat_count = 4 + rng.next_usize(5);
    let stats = pick_stats(rng, stat_count);

    let mut parts = vec![
        format!(r#""item_category": "{category}""#),
        format!(r#""item_level": {}"#, rng.next_range(60, 85)),
        r#""item_rarity": "Rare""#.to_owned(),
        r#""item_rarity_2": "Non-Unique""#.to_owned(),
        r#""name": "Benchmark Rare Item""#.to_owned(),
        r#""identified": true"#.to_owned(),
        r#""corrupted": false"#.to_owned(),
    ];

    for &si in &stats {
        let value = rng.next_range(5, 100);
        parts.push(format!(r#""{}": {value}"#, STAT_POOL[si]));
    }

    let json = format!("{{{}}}", parts.join(","));
    serde_json::from_str(&json).unwrap()
}

// ---------------------------------------------------------------------------
// Benchmark runners
// ---------------------------------------------------------------------------

fn bench_brute_force(queries: &[Vec<Condition>], entry: &Entry, iterations: u64) {
    let mut store = QueryStore::new();
    for rq in queries {
        store.add(rq.clone(), vec![]);
    }

    let start = Instant::now();
    for _ in 0..iterations {
        black_box(store.match_item(entry));
    }
    let elapsed = start.elapsed();

    let query_count = queries.len();
    let us_per_match = (elapsed.as_secs_f64() * 1_000_000.0) / iterations as f64;
    let matches = store.match_item(entry);

    println!(
        "  brute-force | {:>7} queries | {:>9.1}μs/match | {} matches",
        query_count, us_per_match, matches.len(),
    );
}

fn bench_indexed(queries: &[Vec<Condition>], entry: &Entry, iterations: u64) {
    let mut store = IndexedStore::new();
    for rq in queries {
        store.add(rq.clone(), vec![]);
    }

    let start = Instant::now();
    for _ in 0..iterations {
        black_box(store.match_item(entry));
    }
    let elapsed = start.elapsed();

    let query_count = queries.len();
    let us_per_match = (elapsed.as_secs_f64() * 1_000_000.0) / iterations as f64;
    let matches = store.match_item(entry);

    println!(
        "  indexed     | {:>7} queries | {:>9.1}μs/match | {} matches | {:>6} nodes, depth {}",
        query_count,
        us_per_match,
        matches.len(),
        store.node_count(),
        store.max_depth(),
    );
}

fn iterations_for(query_count: usize) -> u64 {
    match query_count {
        n if n <= 1_000 => 10_000,
        n if n <= 10_000 => 1_000,
        n if n <= 100_000 => 100,
        _ => 10,
    }
}

// ---------------------------------------------------------------------------
// Synthetic benchmark (original — uniform queries, max sharing)
// ---------------------------------------------------------------------------

fn make_synthetic_rq(
    category_idx: usize,
    stat_idx: usize,
    min_val: i64,
    max_val: i64,
) -> Vec<Condition> {
    let category = ITEM_CATEGORIES[category_idx % ITEM_CATEGORIES.len()];
    let stat = STAT_POOL[stat_idx % STAT_POOL.len()];
    let json = format!(
        r#"[
            {{"key": "item_category", "value": "{category}", "type": "string", "typeOptions": null}},
            {{"key": "item_rarity_2", "value": "Non-Unique", "type": "string", "typeOptions": null}},
            {{"key": "list", "value": [
                {{"key": "{stat}", "value": {min_val}, "type": "integer", "typeOptions": {{"operator": "<"}}}},
                {{"key": "{stat}", "value": {max_val}, "type": "integer", "typeOptions": {{"operator": ">"}}}}
            ], "type": "list", "typeOptions": {{"operator": "and"}}}}
        ]"#
    );
    serde_json::from_str(&json).unwrap()
}

fn make_synthetic_entry() -> Entry {
    let json = r#"{
        "item_category": "Crimson Jewel",
        "item_level": 75,
        "item_rarity": "Rare",
        "item_rarity_2": "Non-Unique",
        "name": "Benchmark Item",
        "+# to maximum Life": 40,
        "+# to maximum Mana": 30,
        "% increased Armour": 15,
        "+#% to Fire and Cold Resistances": 11,
        "% increased Attack Speed": 6,
        "identified": true,
        "corrupted": false
    }"#;
    serde_json::from_str(json).unwrap()
}

fn generate_synthetic(count: usize) -> Vec<Vec<Condition>> {
    (0..count)
        .map(|i| {
            #[allow(clippy::cast_possible_wrap)]
            make_synthetic_rq(i, i, (i % 20) as i64, ((i % 20) + 30) as i64)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let counts = &[100, 1_000, 10_000, 50_000, 100_000, 500_000, 1_000_000];

    // --- Synthetic (uniform queries, best-case sharing) ---
    println!("=== SYNTHETIC (uniform queries, best-case sharing) ===");
    println!();

    let entry = make_synthetic_entry();
    for &count in counts {
        let queries = generate_synthetic(count);
        let iters = iterations_for(count);
        bench_brute_force(&queries, &entry, iters);
        bench_indexed(&queries, &entry, iters);
        println!();
    }

    // --- Realistic (diverse queries, real PoE behavior) ---
    println!("=== REALISTIC (diverse archetypes, 42 stats, NOT/COUNT/boolean) ===");
    println!();

    for &count in counts {
        let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234); // fixed seed for reproducibility
        let queries: Vec<Vec<Condition>> =
            (0..count).map(|_| generate_realistic_query(&mut rng)).collect();

        // Generate a realistic item to match against
        let mut entry_rng = Rng::new(0x1234_5678_9ABC_DEF0);
        let realistic_entry = make_realistic_entry(&mut entry_rng);

        let iters = iterations_for(count);
        bench_brute_force(&queries, &realistic_entry, iters);
        bench_indexed(&queries, &realistic_entry, iters);
        println!();
    }
}
