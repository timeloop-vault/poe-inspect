//! Benchmark: measure brute-force matching throughput.
//!
//! Run with: `cargo bench -p poe-rqe`
//!
//! Generates N synthetic queries and matches a single item against all of them,
//! measuring queries-per-second throughput. This establishes the baseline for
//! the brute-force approach before indexing is added.

use std::hint::black_box;
use std::time::Instant;

use poe_rqe::eval::Entry;
use poe_rqe::predicate::Condition;
use poe_rqe::store::QueryStore;

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

const STAT_NAMES: &[&str] = &[
    "explicit stat_1 % increased Armour",
    "explicit stat_1 +% to Fire and Cold Resistances",
    "explicit stat_1 % increased Attack Speed with Axes",
    "explicit stat_1 +#% total to lightning resistance",
    "explicit stat_1 maximum Life",
    "explicit stat_1 maximum Mana",
    "explicit stat_1 % increased Physical Damage",
    "explicit stat_1 Adds # to # Fire Damage",
    "explicit stat_1 % increased Critical Strike Chance",
    "explicit stat_1 +% to Chaos Resistance",
];

/// Generate a synthetic RQ as JSON, then deserialize it.
fn make_rq(category_idx: usize, stat_idx: usize, min_val: i64, max_val: i64) -> Vec<Condition> {
    let category = ITEM_CATEGORIES[category_idx % ITEM_CATEGORIES.len()];
    let stat = STAT_NAMES[stat_idx % STAT_NAMES.len()];
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

fn make_entry() -> Entry {
    let json = r#"{
        "item_category": "Crimson Jewel",
        "item_level": 75,
        "item_rarity": "Rare",
        "item_rarity_2": "Non-Unique",
        "name": "Benchmark Item",
        "explicit stat_1 % increased Armour": 15,
        "explicit stat_1 +% to Fire and Cold Resistances": 11,
        "explicit stat_1 % increased Attack Speed with Axes": 6,
        "explicit stat_1 maximum Life": 40,
        "explicit stat_1 maximum Mana": 30
    }"#;
    serde_json::from_str(json).unwrap()
}

fn bench_match(query_count: usize) {
    let mut store = QueryStore::new();
    for i in 0..query_count {
        #[allow(clippy::cast_possible_wrap)]
        let rq = make_rq(i, i, (i % 20) as i64, ((i % 20) + 30) as i64);
        store.add(rq, vec![]);
    }

    let entry = make_entry();
    let iterations = 100;

    let start = Instant::now();
    for _ in 0..iterations {
        black_box(store.match_item(&entry));
    }
    let elapsed = start.elapsed();

    let total_evaluations = query_count as u64 * iterations;
    let evals_per_sec = total_evaluations as f64 / elapsed.as_secs_f64();
    let matches = store.match_item(&entry);

    println!(
        "  {query_count:>7} queries | {iterations} iterations | {:.2}ms total | {:.0} evals/sec | {} matches found",
        elapsed.as_secs_f64() * 1000.0,
        evals_per_sec,
        matches.len(),
    );
}

fn main() {
    println!("RQE brute-force matching benchmark");
    println!("===================================");
    println!();

    for &count in &[100, 1_000, 10_000, 50_000, 100_000] {
        bench_match(count);
    }

    println!();
    println!("This is the brute-force baseline. Indexing should improve this significantly.");
}
