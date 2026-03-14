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

    // --- Product marketplace entries ---

    fn electronics_entry() -> Entry {
        serde_json::from_str(
            r#"{"category": "Electronics", "in_stock": true, "on_sale": false, "price": 299, "weight": 2, "rating": 4, "color": "Black"}"#,
        )
        .unwrap()
    }

    fn clothing_entry() -> Entry {
        serde_json::from_str(
            r#"{"category": "Clothing", "in_stock": true, "on_sale": true, "price": 49, "weight": 1, "rating": 5, "color": "Red"}"#,
        )
        .unwrap()
    }

    fn book_entry() -> Entry {
        serde_json::from_str(
            r#"{"category": "Books", "in_stock": false, "on_sale": false, "price": 15, "weight": 1, "rating": 3}"#,
        )
        .unwrap()
    }

    fn want_electronics_in_stock() -> Vec<Condition> {
        serde_json::from_str(
            r#"[
                {"key": "category", "value": "Electronics", "type": "string", "typeOptions": null},
                {"key": "in_stock", "value": true, "type": "boolean", "typeOptions": null}
            ]"#,
        )
        .unwrap()
    }

    fn want_cheap_electronics() -> Vec<Condition> {
        serde_json::from_str(
            r#"[
                {"key": "category", "value": "Electronics", "type": "string", "typeOptions": null},
                {"key": "price", "value": 500, "type": "integer", "typeOptions": {"operator": ">"}}
            ]"#,
        )
        .unwrap()
    }

    fn want_clothing_on_sale() -> Vec<Condition> {
        serde_json::from_str(
            r#"[
                {"key": "category", "value": "Clothing", "type": "string", "typeOptions": null},
                {"key": "on_sale", "value": true, "type": "boolean", "typeOptions": null}
            ]"#,
        )
        .unwrap()
    }

    #[test]
    fn add_and_remove() {
        let mut store = QueryStore::new();
        assert!(store.is_empty());

        let id = store.add(want_electronics_in_stock(), vec![]);
        assert_eq!(store.len(), 1);
        assert!(store.get(id).is_some());

        assert!(store.remove(id));
        assert!(store.is_empty());
        assert!(!store.remove(id));
    }

    #[test]
    fn match_single_query() {
        let mut store = QueryStore::new();
        let id = store.add(want_electronics_in_stock(), vec![]);

        let matches = store.match_item(&electronics_entry());
        assert_eq!(matches, vec![id]);

        // Book is not electronics
        let matches = store.match_item(&book_entry());
        assert!(matches.is_empty());
    }

    #[test]
    fn match_multiple_queries() {
        let mut store = QueryStore::new();
        let id_stock = store.add(want_electronics_in_stock(), vec![]);
        let id_cheap = store.add(want_cheap_electronics(), vec![]);
        let _id_clothing = store.add(want_clothing_on_sale(), vec![]);

        // Electronics entry matches both electronics queries but not clothing
        let mut matches = store.match_item(&electronics_entry());
        matches.sort_unstable();
        let mut expected = vec![id_stock, id_cheap];
        expected.sort_unstable();
        assert_eq!(matches, expected);
    }

    #[test]
    fn match_no_queries_for_unrelated_item() {
        let mut store = QueryStore::new();
        store.add(want_electronics_in_stock(), vec![]);
        store.add(want_cheap_electronics(), vec![]);

        let matches = store.match_item(&book_entry());
        assert!(matches.is_empty());
    }

    #[test]
    fn match_with_labels() {
        let mut store = QueryStore::new();
        let id = store.add(
            want_electronics_in_stock(),
            vec!["wishlist:gaming".into(), "priority:high".into()],
        );

        let query = store.get(id).unwrap();
        assert_eq!(query.labels, vec!["wishlist:gaming", "priority:high"]);
    }

    #[test]
    fn ids_are_unique_and_sequential() {
        let mut store = QueryStore::new();
        let id0 = store.add(want_electronics_in_stock(), vec![]);
        let id1 = store.add(want_cheap_electronics(), vec![]);
        let id2 = store.add(want_clothing_on_sale(), vec![]);
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn match_all_combinations() {
        let queries: Vec<Vec<Condition>> = vec![
            want_electronics_in_stock(),
            want_cheap_electronics(),
            want_clothing_on_sale(),
        ];
        let entries = vec![electronics_entry(), clothing_entry(), book_entry()];

        let mut store = QueryStore::new();
        for q in &queries {
            store.add(q.clone(), vec![]);
        }
        assert_eq!(store.len(), 3);

        for entry in &entries {
            let matches = store.match_item(entry);
            for id in &matches {
                assert!(store.get(*id).is_some());
            }
        }
    }
}
