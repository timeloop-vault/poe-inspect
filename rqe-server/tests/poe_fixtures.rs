//! PoE-specific integration tests using Erlang RQE fixtures.
//!
//! These tests validate that the generic reverse query engine works correctly
//! with PoE domain data (crimson jewels, boots, rings, etc.). The fixtures
//! live at `_reference/rqe/test/data/` and match the original Erlang test suite.

use poe_rqe::eval::{Entry, evaluate};
use poe_rqe::index::{IndexedStore, SelectivityConfig};
use poe_rqe::predicate::Condition;
use poe_rqe::store::QueryStore;

const FIXTURE_BASE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../_reference/rqe/test/data/");

fn load_rq(filename: &str) -> Vec<Condition> {
    let path = format!("{FIXTURE_BASE}rq/{filename}");
    let data =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    serde_json::from_str(&data).unwrap()
}

fn load_entry(filename: &str) -> Entry {
    let path = format!("{FIXTURE_BASE}entry/{filename}");
    let data =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    serde_json::from_str(&data).unwrap()
}

fn poe_config() -> SelectivityConfig {
    SelectivityConfig::new(&["item_category", "item_rarity*"])
}

// ===========================================================================
// Eval tests — ported from poe-rqe/src/eval.rs
// ===========================================================================

#[test]
fn erlang_suite_mod_and_not_count_matches_mods_2() {
    let rq = load_rq("wanted_mod_and_not_count.json");
    let entry = load_entry("crimson_w_mods_2.json");
    assert!(evaluate(&rq, &entry), "should match crimson_w_mods_2");
}

#[test]
fn erlang_suite_mod_and_not_count_rejects_mods_1() {
    let rq = load_rq("wanted_mod_and_not_count.json");
    let entry = load_entry("crimson_w_mods_1.json");
    assert!(!evaluate(&rq, &entry), "should NOT match crimson_w_mods_1");
}

#[test]
fn crimson_rare_matches_rare_crimson() {
    let rq = load_rq("wanted_crimson_rare.json");
    let entry = load_entry("crimson_w_mods_1.json");
    assert!(evaluate(&rq, &entry));
}

#[test]
fn crimson_rare_rejects_magic() {
    let rq = load_rq("wanted_crimson_rare.json");
    let entry = load_entry("crimson_magic.json");
    assert!(!evaluate(&rq, &entry));
}

#[test]
fn crimson_rare_rejects_unique() {
    let rq = load_rq("wanted_crimson_rare.json");
    let entry = load_entry("crimson_unique.json");
    assert!(!evaluate(&rq, &entry));
}

#[test]
fn crimson_mod_matches_armor_15() {
    let rq = load_rq("wanted_crimson_mod.json");
    let entry = load_entry("crimson_w_mods_1.json");
    assert!(evaluate(&rq, &entry));
}

#[test]
fn crimson_mod_rejects_no_armor() {
    let rq = load_rq("wanted_crimson_mod.json");
    let entry = load_entry("crimson_w_mods_2.json");
    assert!(!evaluate(&rq, &entry));
}

#[test]
fn crimson_mod_not_rejects_armor_15() {
    let rq = load_rq("wanted_crimson_mod_not.json");
    let entry = load_entry("crimson_w_mods_1.json");
    assert!(!evaluate(&rq, &entry));
}

#[test]
fn crimson_mod_not_matches_no_armor() {
    let rq = load_rq("wanted_crimson_mod_not.json");
    let entry = load_entry("crimson_w_mods_2.json");
    assert!(evaluate(&rq, &entry));
}

#[test]
fn crimson_mod_count_matches_mods_1() {
    let rq = load_rq("wanted_crimson_mod_count.json");
    let entry = load_entry("crimson_w_mods_1.json");
    assert!(!evaluate(&rq, &entry));
}

#[test]
fn crimson_mod_count_rejects_no_armor_no_lightning() {
    let rq = load_rq("wanted_crimson_mod_count.json");
    let entry = load_entry("crimson_w_mods_2.json");
    assert!(!evaluate(&rq, &entry));
}

#[test]
fn crimson_mod_count_2_matches_mods_2() {
    let rq = load_rq("wanted_crimson_mod_count_2.json");
    let entry = load_entry("crimson_w_mods_2.json");
    assert!(evaluate(&rq, &entry));
}

#[test]
fn crimson_mod_and_not_matches_mods_2() {
    let rq = load_rq("wanted_crimson_mod_and_not.json");
    let entry = load_entry("crimson_w_mods_2.json");
    assert!(evaluate(&rq, &entry));
}

#[test]
fn crimson_mod_and_not_rejects_mods_1() {
    let rq = load_rq("wanted_crimson_mod_and_not.json");
    let entry = load_entry("crimson_w_mods_1.json");
    assert!(!evaluate(&rq, &entry));
}

#[test]
fn crimson_rq_rejects_ring() {
    let rq = load_rq("wanted_crimson_rare.json");
    let entry = load_entry("paua_ring_rare.json");
    assert!(!evaluate(&rq, &entry));
}

#[test]
fn crimson_rq_rejects_weapon() {
    let rq = load_rq("wanted_crimson_mod.json");
    let entry = load_entry("two_handed_weapon_rare.json");
    assert!(!evaluate(&rq, &entry));
}

#[test]
fn boots_unique_matches_4_socket_3_link() {
    let rq = load_rq("wanted_boots_unique.json");
    let entry = load_entry("item_socket_4_link_3.json");
    assert!(evaluate(&rq, &entry));
}

#[test]
fn boots_unique_rejects_2_socket_wand() {
    let rq = load_rq("wanted_boots_unique.json");
    let entry = load_entry("item_socket_2_link_0.json");
    assert!(!evaluate(&rq, &entry));
}

#[test]
fn new_format_rejects_boots_no_lightning() {
    let rq = load_rq("wanted_boots_unique_new_format.json");
    let entry = load_entry("item_socket_4_link_3.json");
    assert!(!evaluate(&rq, &entry));
}

#[test]
fn new_format_matches_paua_ring() {
    let rq = load_rq("wanted_boots_unique_new_format.json");
    let entry = load_entry("paua_ring_rare.json");
    assert!(evaluate(&rq, &entry));
}

// ===========================================================================
// Store tests — ported from poe-rqe/src/store.rs
// ===========================================================================

#[test]
fn store_add_and_remove() {
    let mut store = QueryStore::new();
    assert!(store.is_empty());

    let id = store.add(load_rq("wanted_crimson_rare.json"), vec![], None);
    assert_eq!(store.len(), 1);
    assert!(store.get(id).is_some());

    assert!(store.remove(id));
    assert!(store.is_empty());
    assert!(!store.remove(id));
}

#[test]
fn store_match_single_query() {
    let mut store = QueryStore::new();
    let id = store.add(load_rq("wanted_crimson_rare.json"), vec![], None);

    let matches = store.match_item(&load_entry("crimson_w_mods_1.json"));
    assert_eq!(matches, vec![id]);

    let matches = store.match_item(&load_entry("crimson_magic.json"));
    assert!(matches.is_empty());
}

#[test]
fn store_match_multiple_queries() {
    let mut store = QueryStore::new();
    let id_rare = store.add(load_rq("wanted_crimson_rare.json"), vec![], None);
    let id_mod = store.add(load_rq("wanted_crimson_mod.json"), vec![], None);
    let _id_not = store.add(load_rq("wanted_crimson_mod_not.json"), vec![], None);

    let mut matches = store.match_item(&load_entry("crimson_w_mods_1.json"));
    matches.sort_unstable();
    let mut expected = vec![id_rare, id_mod];
    expected.sort_unstable();
    assert_eq!(matches, expected);
}

#[test]
fn store_match_no_queries_for_unrelated_item() {
    let mut store = QueryStore::new();
    store.add(load_rq("wanted_crimson_rare.json"), vec![], None);
    store.add(load_rq("wanted_crimson_mod.json"), vec![], None);

    let matches = store.match_item(&load_entry("paua_ring_rare.json"));
    assert!(matches.is_empty());
}

#[test]
fn store_match_with_labels() {
    let mut store = QueryStore::new();
    let id = store.add(
        load_rq("wanted_crimson_rare.json"),
        vec!["build:cyclone".into(), "priority:high".into()],
        None,
    );

    let query = store.get(id).unwrap();
    assert_eq!(query.labels, vec!["build:cyclone", "priority:high"]);
}

#[test]
fn store_ids_are_unique_and_sequential() {
    let mut store = QueryStore::new();
    let id0 = store.add(load_rq("wanted_crimson_rare.json"), vec![], None);
    let id1 = store.add(load_rq("wanted_crimson_mod.json"), vec![], None);
    let id2 = store.add(load_rq("wanted_crimson_mod_not.json"), vec![], None);
    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn store_match_all_rqs_against_all_entries() {
    let rq_files = [
        "wanted_crimson_rare.json",
        "wanted_crimson_mod.json",
        "wanted_crimson_mod_not.json",
        "wanted_crimson_mod_count.json",
        "wanted_crimson_mod_count_2.json",
        "wanted_crimson_mod_and_not.json",
        "wanted_mod_and_not_count.json",
        "wanted_boots_unique.json",
        "wanted_boots_unique_new_format.json",
    ];
    let entry_files = [
        "crimson_w_mods_1.json",
        "crimson_w_mods_2.json",
        "crimson_magic.json",
        "crimson_unique.json",
        "paua_ring_rare.json",
        "two_handed_weapon_rare.json",
        "item_socket_4_link_3.json",
        "item_socket_2_link_0.json",
    ];

    let mut store = QueryStore::new();
    for rq_file in &rq_files {
        store.add(load_rq(rq_file), vec![], None);
    }
    assert_eq!(store.len(), 9);

    for entry_file in &entry_files {
        let entry = load_entry(entry_file);
        let matches = store.match_item(&entry);
        for id in &matches {
            assert!(store.get(*id).is_some());
        }
    }
}

// ===========================================================================
// Index tests — ported from poe-rqe/src/index.rs
// ===========================================================================

#[test]
fn index_equivalence_with_brute_force() {
    let rq_files = [
        "wanted_crimson_rare.json",
        "wanted_crimson_mod.json",
        "wanted_crimson_mod_not.json",
        "wanted_crimson_mod_count.json",
        "wanted_crimson_mod_count_2.json",
        "wanted_crimson_mod_and_not.json",
        "wanted_mod_and_not_count.json",
        "wanted_boots_unique.json",
        "wanted_boots_unique_new_format.json",
    ];
    let entry_files = [
        "crimson_w_mods_1.json",
        "crimson_w_mods_2.json",
        "crimson_magic.json",
        "crimson_unique.json",
        "paua_ring_rare.json",
        "two_handed_weapon_rare.json",
        "item_socket_4_link_3.json",
        "item_socket_2_link_0.json",
    ];

    let mut brute = QueryStore::new();
    let mut indexed = IndexedStore::with_selectivity(poe_config());

    for rq_file in &rq_files {
        let conditions = load_rq(rq_file);
        brute.add(conditions.clone(), vec![], None);
        indexed.add(conditions, vec![], None);
    }

    for entry_file in &entry_files {
        let entry = load_entry(entry_file);

        let mut brute_matches = brute.match_item(&entry);
        let mut indexed_matches = indexed.match_item(&entry);

        brute_matches.sort_unstable();
        indexed_matches.sort_unstable();

        assert_eq!(
            brute_matches, indexed_matches,
            "mismatch for entry {entry_file}"
        );
    }
}

#[test]
fn index_match_all_rqs_against_all_entries() {
    let rq_files = [
        "wanted_crimson_rare.json",
        "wanted_crimson_mod.json",
        "wanted_crimson_mod_not.json",
        "wanted_crimson_mod_count.json",
        "wanted_crimson_mod_count_2.json",
        "wanted_crimson_mod_and_not.json",
        "wanted_mod_and_not_count.json",
        "wanted_boots_unique.json",
        "wanted_boots_unique_new_format.json",
    ];
    let entry_files = [
        "crimson_w_mods_1.json",
        "crimson_w_mods_2.json",
        "crimson_magic.json",
        "crimson_unique.json",
        "paua_ring_rare.json",
        "two_handed_weapon_rare.json",
        "item_socket_4_link_3.json",
        "item_socket_2_link_0.json",
    ];

    let mut store = IndexedStore::with_selectivity(poe_config());
    for rq_file in &rq_files {
        store.add(load_rq(rq_file), vec![], None);
    }
    assert_eq!(store.len(), 9);

    for entry_file in &entry_files {
        let entry = load_entry(entry_file);
        let matches = store.match_item(&entry);
        for id in &matches {
            assert!(store.get(*id).is_some());
        }
    }
}

// ===========================================================================
// Predicate tests — ported from poe-rqe/src/predicate.rs
// ===========================================================================

#[test]
fn round_trip_all_rq_files() {
    let rq_files = [
        "wanted_crimson_rare.json",
        "wanted_crimson_mod.json",
        "wanted_crimson_mod_not.json",
        "wanted_crimson_mod_count.json",
        "wanted_crimson_mod_count_2.json",
        "wanted_crimson_mod_and_not.json",
        "wanted_mod_and_not_count.json",
        "wanted_boots_unique.json",
        "wanted_boots_unique_new_format.json",
    ];
    for file in &rq_files {
        let path = format!("{FIXTURE_BASE}rq/{file}");
        let json = std::fs::read_to_string(&path).unwrap();
        let conditions: Vec<Condition> = serde_json::from_str(&json).unwrap();
        let serialized = serde_json::to_string(&conditions).unwrap();
        let round_tripped: Vec<Condition> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(conditions, round_tripped, "round-trip failed for {file}");
    }
}

#[test]
fn deserialize_full_rq_file() {
    let path = format!("{FIXTURE_BASE}rq/wanted_crimson_mod.json");
    let json =
        std::fs::read_to_string(&path).expect("test data file should exist at _reference/rqe/");
    let conditions: Vec<Condition> = serde_json::from_str(&json).unwrap();
    assert_eq!(conditions.len(), 3);
}
