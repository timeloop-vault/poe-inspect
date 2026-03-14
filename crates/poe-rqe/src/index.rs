use std::cmp::Ordering;
use std::collections::HashMap;

use crate::eval::{Entry, evaluate_one};
use crate::predicate::{Condition, ListOp, Value};
use crate::store::{QueryId, StoredQuery};

type NodeId = u32;

/// A single node in the decision DAG.
struct DagNode {
    /// Condition to test. `None` for the root (always passes).
    condition: Option<Condition>,

    /// Queries fully satisfied at this node — all ancestor conditions passed.
    terminal_queries: Vec<QueryId>,

    /// Child node IDs, each with an additional condition to test.
    children: Vec<NodeId>,
}

/// Indexed query store using a decision DAG for shared condition evaluation.
///
/// Drop-in replacement for [`crate::store::QueryStore`]. Internally builds a DAG
/// of shared condition nodes so that common prefixes (e.g., `item_category = "Crimson Jewel"`)
/// are evaluated once regardless of how many queries share them.
#[derive(Default)]
pub struct IndexedStore {
    /// Arena-allocated DAG nodes.
    nodes: Vec<DagNode>,

    /// Root node ID (always 0 after first `ensure_root`).
    root: NodeId,

    /// Query metadata — original conditions + labels.
    queries: HashMap<QueryId, StoredQuery>,

    /// Next auto-increment ID.
    next_id: QueryId,
}

impl IndexedStore {
    #[must_use]
    pub fn new() -> Self {
        let mut store = Self::default();
        store.ensure_root();
        store
    }

    /// Add a reverse query. Returns its assigned ID.
    pub fn add(&mut self, conditions: Vec<Condition>, labels: Vec<String>) -> QueryId {
        let id = self.next_id;
        self.next_id += 1;
        self.insert_query(id, conditions, labels);
        id
    }

    /// Add a reverse query with a specific ID. Used when restoring from persistence.
    pub fn add_with_id(&mut self, id: QueryId, conditions: Vec<Condition>, labels: Vec<String>) {
        self.insert_query(id, conditions, labels);
    }

    /// Set the next auto-increment ID. Used when restoring from persistence.
    pub fn set_next_id(&mut self, id: QueryId) {
        self.next_id = id;
    }

    /// Remove a reverse query by ID. Returns `true` if it existed.
    pub fn remove(&mut self, id: QueryId) -> bool {
        let Some(query) = self.queries.remove(&id) else {
            return false;
        };

        // Re-walk the DAG using the stored conditions to find the terminal node.
        let canonical = canonicalize(&query.conditions);
        self.remove_from_dag(id, &canonical);
        true
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

    /// Match an item entry against all stored queries using the decision DAG.
    #[must_use]
    pub fn match_item(&self, entry: &Entry) -> Vec<QueryId> {
        let mut results = Vec::new();
        if !self.nodes.is_empty() {
            self.walk(self.root, entry, &mut results);
        }
        results
    }

    /// Total number of nodes in the DAG (including root).
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Maximum depth of the DAG (0 = root only).
    #[must_use]
    pub fn max_depth(&self) -> usize {
        if self.nodes.is_empty() {
            return 0;
        }
        self.depth_of(self.root)
    }

    /// Average branching factor across non-leaf nodes.
    #[must_use]
    pub fn avg_branching_factor(&self) -> f64 {
        let non_leaf: Vec<_> = self
            .nodes
            .iter()
            .filter(|n| !n.children.is_empty())
            .collect();
        if non_leaf.is_empty() {
            return 0.0;
        }
        let total_children: usize = non_leaf.iter().map(|n| n.children.len()).sum();
        total_children as f64 / non_leaf.len() as f64
    }

    // --- Internals ---

    fn ensure_root(&mut self) {
        if self.nodes.is_empty() {
            self.nodes.push(DagNode {
                condition: None,
                terminal_queries: Vec::new(),
                children: Vec::new(),
            });
            self.root = 0;
        }
    }

    fn insert_query(&mut self, id: QueryId, conditions: Vec<Condition>, labels: Vec<String>) {
        self.ensure_root();

        let canonical = canonicalize(&conditions);

        // Walk/extend the DAG.
        let mut current = self.root;
        for condition in &canonical {
            // Look for an existing child with this condition.
            let existing = self.nodes[current as usize]
                .children
                .iter()
                .find(|&&child_id| {
                    self.nodes[child_id as usize].condition.as_ref() == Some(condition)
                })
                .copied();

            if let Some(child_id) = existing {
                current = child_id;
            } else {
                // Allocate a new node.
                let new_id = self.nodes.len() as NodeId;
                self.nodes.push(DagNode {
                    condition: Some(condition.clone()),
                    terminal_queries: Vec::new(),
                    children: Vec::new(),
                });
                self.nodes[current as usize].children.push(new_id);
                current = new_id;
            }
        }

        // Mark terminal.
        self.nodes[current as usize].terminal_queries.push(id);

        // Store metadata.
        self.queries.insert(
            id,
            StoredQuery {
                id,
                conditions,
                labels,
            },
        );
    }

    fn remove_from_dag(&mut self, id: QueryId, canonical: &[Condition]) {
        // Collect the path: sequence of node IDs from root to terminal.
        let mut path = vec![self.root];
        let mut current = self.root;

        for condition in canonical {
            let child = self.nodes[current as usize]
                .children
                .iter()
                .find(|&&child_id| {
                    self.nodes[child_id as usize].condition.as_ref() == Some(condition)
                })
                .copied();

            if let Some(child_id) = child {
                path.push(child_id);
                current = child_id;
            } else {
                // Path doesn't exist — query wasn't in the DAG (shouldn't happen).
                return;
            }
        }

        // Remove query ID from terminal node.
        let terminal = *path.last().expect("path is non-empty");
        self.nodes[terminal as usize]
            .terminal_queries
            .retain(|&q| q != id);

        // Prune empty nodes bottom-up.
        // Walk backward, skipping the root (can't prune root).
        for i in (1..path.len()).rev() {
            let node_id = path[i];
            let node = &self.nodes[node_id as usize];
            if node.terminal_queries.is_empty() && node.children.is_empty() {
                // Remove this node from its parent's children list.
                let parent_id = path[i - 1];
                self.nodes[parent_id as usize]
                    .children
                    .retain(|&c| c != node_id);
                // Note: we don't deallocate from the arena (it's append-only).
                // The node becomes unreachable. For long-running systems with
                // high churn, periodic rebuild would reclaim this space.
            } else {
                // Node still has content — stop pruning.
                break;
            }
        }
    }

    fn walk(&self, node_id: NodeId, entry: &Entry, results: &mut Vec<QueryId>) {
        let node = &self.nodes[node_id as usize];

        // Test this node's condition (root has None → always passes).
        if let Some(condition) = &node.condition {
            if !evaluate_one(condition, entry) {
                return; // Prune entire subtree.
            }
        }

        // Collect terminal queries.
        results.extend_from_slice(&node.terminal_queries);

        // Recurse into all children.
        for &child_id in &node.children {
            self.walk(child_id, entry, results);
        }
    }

    fn depth_of(&self, node_id: NodeId) -> usize {
        let node = &self.nodes[node_id as usize];
        if node.children.is_empty() {
            return 0;
        }
        1 + node
            .children
            .iter()
            .map(|&c| self.depth_of(c))
            .max()
            .unwrap_or(0)
    }
}

// --- Canonicalization ---

/// Flatten AND lists and sort conditions into canonical order for maximum DAG sharing.
fn canonicalize(conditions: &[Condition]) -> Vec<Condition> {
    let mut flat = Vec::new();
    flatten_and(conditions, &mut flat);
    flat.sort_by(condition_ordering);
    flat
}

/// Recursively flatten AND lists into individual conditions.
/// NOT, OR, and COUNT lists are kept as opaque units.
fn flatten_and(conditions: &[Condition], out: &mut Vec<Condition>) {
    for cond in conditions {
        if let Value::List {
            op: ListOp::And,
            conditions: inner,
        } = &cond.value
        {
            // Recurse: AND([AND([a, b]), c]) → [a, b, c]
            flatten_and(inner, out);
        } else {
            out.push(cond.clone());
        }
    }
}

/// Canonical sort key for conditions.
///
/// Priority scheme (lower = evaluated first):
///   0: `item_category` string equality (highest selectivity)
///   1: `item_rarity*` string equality
///   2: other string equality
///   3: string wildcard (existence checks)
///   4: boolean conditions
///   5: integer conditions
///   6: compound (NOT/OR/COUNT lists)
fn condition_priority(cond: &Condition) -> u8 {
    match &cond.value {
        Value::String(_) if cond.key == "item_category" => 0,
        Value::String(_) if cond.key.starts_with("item_rarity") => 1,
        Value::String(crate::predicate::StringMatch::Exact(_)) => 2,
        Value::String(crate::predicate::StringMatch::Wildcard) => 3,
        Value::Boolean(_) => 4,
        Value::Integer { .. } => 5,
        Value::List { .. } => 6,
    }
}

/// Ordering function for canonical condition sort.
fn condition_ordering(a: &Condition, b: &Condition) -> Ordering {
    let pa = condition_priority(a);
    let pb = condition_priority(b);
    pa.cmp(&pb).then_with(|| a.key.cmp(&b.key)).then_with(|| {
        // Within same priority and key, use a stable tiebreaker.
        // Compare serialized values for determinism.
        value_sort_key(&a.value).cmp(&value_sort_key(&b.value))
    })
}

/// Produce a sortable key for a Value. Used only for deterministic ordering
/// within conditions that share the same priority and key.
fn value_sort_key(value: &Value) -> (u8, i64, String) {
    match value {
        Value::Boolean(b) => (0, i64::from(*b), String::new()),
        Value::String(crate::predicate::StringMatch::Wildcard) => (1, 0, "_".to_owned()),
        Value::String(crate::predicate::StringMatch::Exact(s)) => (1, 0, s.clone()),
        Value::Integer { value: v, op } => (2, *v, format!("{op:?}")),
        Value::List { op, .. } => (3, 0, format!("{op:?}")),
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

    // --- Same tests as QueryStore ---

    #[test]
    fn add_and_remove() {
        let mut store = IndexedStore::new();
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
        let mut store = IndexedStore::new();
        let id = store.add(load_rq("wanted_crimson_rare.json"), vec![]);

        let matches = store.match_item(&load_entry("crimson_w_mods_1.json"));
        assert_eq!(matches, vec![id]);

        let matches = store.match_item(&load_entry("crimson_magic.json"));
        assert!(matches.is_empty());
    }

    #[test]
    fn match_multiple_queries() {
        let mut store = IndexedStore::new();
        let id_rare = store.add(load_rq("wanted_crimson_rare.json"), vec![]);
        let id_mod = store.add(load_rq("wanted_crimson_mod.json"), vec![]);
        let _id_not = store.add(load_rq("wanted_crimson_mod_not.json"), vec![]);

        let mut matches = store.match_item(&load_entry("crimson_w_mods_1.json"));
        matches.sort_unstable();
        let mut expected = vec![id_rare, id_mod];
        expected.sort_unstable();
        assert_eq!(matches, expected);
    }

    #[test]
    fn match_no_queries_for_unrelated_item() {
        let mut store = IndexedStore::new();
        store.add(load_rq("wanted_crimson_rare.json"), vec![]);
        store.add(load_rq("wanted_crimson_mod.json"), vec![]);

        let matches = store.match_item(&load_entry("paua_ring_rare.json"));
        assert!(matches.is_empty());
    }

    #[test]
    fn match_with_labels() {
        let mut store = IndexedStore::new();
        let id = store.add(
            load_rq("wanted_crimson_rare.json"),
            vec!["build:cyclone".into(), "priority:high".into()],
        );

        let query = store.get(id).unwrap();
        assert_eq!(query.labels, vec!["build:cyclone", "priority:high"]);
    }

    #[test]
    fn ids_are_unique_and_sequential() {
        let mut store = IndexedStore::new();
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

        let mut store = IndexedStore::new();
        for rq_file in &rq_files {
            store.add(load_rq(rq_file), vec![]);
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

    // --- Equivalence: IndexedStore must match QueryStore exactly ---

    #[test]
    fn equivalence_with_brute_force() {
        use crate::store::QueryStore;

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
        let mut indexed = IndexedStore::new();

        for rq_file in &rq_files {
            let conditions = load_rq(rq_file);
            brute.add(conditions.clone(), vec![]);
            indexed.add(conditions, vec![]);
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

    // --- DAG diagnostics ---

    #[test]
    fn dag_diagnostics() {
        let mut store = IndexedStore::new();
        assert_eq!(store.node_count(), 1); // root only
        assert_eq!(store.max_depth(), 0);

        store.add(load_rq("wanted_crimson_rare.json"), vec![]);
        store.add(load_rq("wanted_crimson_mod.json"), vec![]);

        // Both share item_category="Crimson Jewel" and item_rarity_2="Non-Unique",
        // so the DAG should have shared prefix nodes.
        assert!(store.node_count() > 1);
        assert!(store.max_depth() >= 2);
        assert!(store.avg_branching_factor() > 0.0);

        println!(
            "DAG: {} nodes, depth {}, avg branching {:.2}",
            store.node_count(),
            store.max_depth(),
            store.avg_branching_factor(),
        );
    }

    // --- Removal prunes empty nodes ---

    #[test]
    fn remove_prunes_dag() {
        let mut store = IndexedStore::new();
        let id = store.add(load_rq("wanted_crimson_rare.json"), vec![]);

        let nodes_before = store.node_count();
        assert!(nodes_before > 1);

        store.remove(id);

        // After removal, pruning should have removed all nodes except root.
        // (Arena doesn't deallocate, but children links are cleaned up.)
        // The root should have no children.
        assert!(store.nodes[store.root as usize].children.is_empty());
    }

    // --- AND flattening ---

    #[test]
    fn and_flattening_shares_conditions() {
        // wanted_crimson_mod has an AND list with two armor conditions.
        // After flattening, those become individual DAG nodes that can be shared.
        let mut store = IndexedStore::new();
        store.add(load_rq("wanted_crimson_mod.json"), vec![]);

        // The AND list [armor < 4, armor > 20] should be flattened into two nodes.
        // Total path: item_category → item_rarity_2 → armor < 4 → armor > 20
        // Plus root = 5 nodes.
        assert_eq!(store.node_count(), 5);
        assert_eq!(store.max_depth(), 4);
    }

    // --- Canonicalization ---

    #[test]
    fn canonicalize_sorts_by_priority() {
        use crate::predicate::{CompareOp, StringMatch};

        let conditions = vec![
            // Integer (priority 5)
            Condition {
                key: "armor".into(),
                value: Value::Integer {
                    value: 50,
                    op: CompareOp::Gt,
                },
            },
            // Boolean (priority 4)
            Condition {
                key: "corrupted".into(),
                value: Value::Boolean(true),
            },
            // item_category string (priority 0)
            Condition {
                key: "item_category".into(),
                value: Value::String(StringMatch::Exact("Ring".into())),
            },
            // item_rarity string (priority 1)
            Condition {
                key: "item_rarity".into(),
                value: Value::String(StringMatch::Exact("Rare".into())),
            },
        ];

        let result = canonicalize(&conditions);
        assert_eq!(result[0].key, "item_category");
        assert_eq!(result[1].key, "item_rarity");
        assert_eq!(result[2].key, "corrupted");
        assert_eq!(result[3].key, "armor");
    }

    #[test]
    fn canonicalize_flattens_nested_and() {
        use crate::predicate::CompareOp;

        let conditions = vec![Condition {
            key: "list".into(),
            value: Value::List {
                op: ListOp::And,
                conditions: vec![
                    Condition {
                        key: "list".into(),
                        value: Value::List {
                            op: ListOp::And,
                            conditions: vec![
                                Condition {
                                    key: "a".into(),
                                    value: Value::Integer {
                                        value: 1,
                                        op: CompareOp::Gt,
                                    },
                                },
                                Condition {
                                    key: "b".into(),
                                    value: Value::Integer {
                                        value: 2,
                                        op: CompareOp::Lt,
                                    },
                                },
                            ],
                        },
                    },
                    Condition {
                        key: "c".into(),
                        value: Value::Integer {
                            value: 3,
                            op: CompareOp::Eq,
                        },
                    },
                ],
            },
        }];

        let result = canonicalize(&conditions);
        // AND([AND([a, b]), c]) → [a, b, c]
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].key, "a");
        assert_eq!(result[1].key, "b");
        assert_eq!(result[2].key, "c");
    }

    #[test]
    fn canonicalize_preserves_not_list() {
        use crate::predicate::CompareOp;

        let not_list = Condition {
            key: "list".into(),
            value: Value::List {
                op: ListOp::Not,
                conditions: vec![Condition {
                    key: "armor".into(),
                    value: Value::Integer {
                        value: 10,
                        op: CompareOp::Gt,
                    },
                }],
            },
        };

        let not_vec = vec![not_list.clone()];
        let result = canonicalize(&not_vec);
        // NOT should not be flattened — stays as one opaque node.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], not_list);
    }
}
