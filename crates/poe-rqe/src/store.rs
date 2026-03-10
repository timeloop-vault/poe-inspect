use std::collections::HashMap;

use crate::eval::{Entry, evaluate};
use crate::predicate::Condition;

/// Unique identifier for a stored reverse query.
pub type QueryId = u64;

/// A stored reverse query with its conditions and metadata.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StoredQuery {
    pub id: QueryId,
    pub conditions: Vec<Condition>,
    pub labels: Vec<String>,
}

/// In-memory store of reverse queries. Brute-force matching: every query is
/// evaluated against every item. This is the baseline implementation —
/// indexing will be layered on top later as an optimization.
#[derive(Debug, Default)]
pub struct QueryStore {
    queries: HashMap<QueryId, StoredQuery>,
    next_id: QueryId,
}

impl QueryStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a reverse query to the store. Returns its assigned ID.
    pub fn add(&mut self, conditions: Vec<Condition>, labels: Vec<String>) -> QueryId {
        let id = self.next_id;
        self.next_id += 1;
        self.queries.insert(
            id,
            StoredQuery {
                id,
                conditions,
                labels,
            },
        );
        id
    }

    /// Add a reverse query with a specific ID. Used when restoring from persistence.
    pub fn add_with_id(&mut self, id: QueryId, conditions: Vec<Condition>, labels: Vec<String>) {
        self.queries.insert(
            id,
            StoredQuery {
                id,
                conditions,
                labels,
            },
        );
    }

    /// Set the next auto-increment ID. Used when restoring from persistence.
    pub fn set_next_id(&mut self, id: QueryId) {
        self.next_id = id;
    }

    /// Remove a reverse query by ID. Returns `true` if it existed.
    pub fn remove(&mut self, id: QueryId) -> bool {
        self.queries.remove(&id).is_some()
    }

    /// Get a reverse query by ID.
    #[must_use]
    pub fn get(&self, id: QueryId) -> Option<&StoredQuery> {
        self.queries.get(&id)
    }

    /// Number of stored queries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.queries.len()
    }

    /// Whether the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queries.is_empty()
    }

    /// Match an item entry against all stored queries.
    /// Returns the IDs of all queries that match.
    ///
    /// This is brute-force: O(n) where n is the number of stored queries.
    #[must_use]
    pub fn match_item(&self, entry: &Entry) -> Vec<QueryId> {
        self.queries
            .values()
            .filter(|q| evaluate(&q.conditions, entry))
            .map(|q| q.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_rq(filename: &str) -> Vec<Condition> {
        let path = format!(
            "{}/_reference/rqe/test/data/rq/{filename}",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
        );
        let data =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
        serde_json::from_str(&data).unwrap()
    }

    fn load_entry(filename: &str) -> Entry {
        let path = format!(
            "{}/_reference/rqe/test/data/entry/{filename}",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
        );
        let data =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
        serde_json::from_str(&data).unwrap()
    }

    #[test]
    fn add_and_remove() {
        let mut store = QueryStore::new();
        assert!(store.is_empty());

        let id = store.add(load_rq("wanted_crimson_rare.json"), vec![]);
        assert_eq!(store.len(), 1);
        assert!(store.get(id).is_some());

        assert!(store.remove(id));
        assert!(store.is_empty());
        assert!(!store.remove(id));
    }

    #[test]
    fn match_single_query() {
        let mut store = QueryStore::new();
        let id = store.add(load_rq("wanted_crimson_rare.json"), vec![]);

        let matches = store.match_item(&load_entry("crimson_w_mods_1.json"));
        assert_eq!(matches, vec![id]);

        let matches = store.match_item(&load_entry("crimson_magic.json"));
        assert!(matches.is_empty());
    }

    #[test]
    fn match_multiple_queries() {
        let mut store = QueryStore::new();
        let id_rare = store.add(load_rq("wanted_crimson_rare.json"), vec![]);
        let id_mod = store.add(load_rq("wanted_crimson_mod.json"), vec![]);
        let _id_not = store.add(load_rq("wanted_crimson_mod_not.json"), vec![]);

        // crimson_w_mods_1: rare crimson with armor=15
        // - wanted_crimson_rare: matches (rare, crimson, non-unique)
        // - wanted_crimson_mod: matches (armor 4-20 AND range)
        // - wanted_crimson_mod_not: rejects (armor IS in NOT range)
        let mut matches = store.match_item(&load_entry("crimson_w_mods_1.json"));
        matches.sort_unstable();
        let mut expected = vec![id_rare, id_mod];
        expected.sort_unstable();
        assert_eq!(matches, expected);
    }

    #[test]
    fn match_no_queries_for_unrelated_item() {
        let mut store = QueryStore::new();
        store.add(load_rq("wanted_crimson_rare.json"), vec![]);
        store.add(load_rq("wanted_crimson_mod.json"), vec![]);

        let matches = store.match_item(&load_entry("paua_ring_rare.json"));
        assert!(matches.is_empty());
    }

    #[test]
    fn match_with_labels() {
        let mut store = QueryStore::new();
        let id = store.add(
            load_rq("wanted_crimson_rare.json"),
            vec!["build:cyclone".into(), "priority:high".into()],
        );

        let query = store.get(id).unwrap();
        assert_eq!(query.labels, vec!["build:cyclone", "priority:high"]);
    }

    #[test]
    fn ids_are_unique_and_sequential() {
        let mut store = QueryStore::new();
        let id0 = store.add(load_rq("wanted_crimson_rare.json"), vec![]);
        let id1 = store.add(load_rq("wanted_crimson_mod.json"), vec![]);
        let id2 = store.add(load_rq("wanted_crimson_mod_not.json"), vec![]);
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn match_all_rqs_against_all_entries() {
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
            store.add(load_rq(rq_file), vec![]);
        }
        assert_eq!(store.len(), 9);

        // Just verify it doesn't panic on any combination
        for entry_file in &entry_files {
            let entry = load_entry(entry_file);
            let matches = store.match_item(&entry);
            // All match IDs should be valid
            for id in &matches {
                assert!(store.get(*id).is_some());
            }
        }
    }
}
