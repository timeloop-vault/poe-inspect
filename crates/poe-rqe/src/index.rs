use std::cmp::Ordering;
use std::collections::HashMap;

use crate::eval::{Entry, EntryValue, evaluate_one};
use crate::predicate::{CompareOp, Condition, ListOp, Value};
use crate::store::{QueryId, StoredQuery};

type NodeId = u32;

/// User-supplied selectivity ranking for condition keys.
/// Keys listed earlier are evaluated first (higher selectivity = more pruning).
/// Keys not in the list use default type-based priority.
#[derive(Clone, Default)]
pub struct SelectivityConfig {
    /// Keys ranked by selectivity, highest first.
    /// Entries can be exact matches or prefix patterns (trailing `*`).
    entries: Vec<SelectivityPattern>,
}

#[derive(Clone)]
enum SelectivityPattern {
    Exact(String),
    Prefix(String),
}

impl SelectivityConfig {
    /// Create a config with the given high-selectivity keys.
    /// Keys are evaluated in order — first key gets highest priority.
    /// Append `*` for prefix matching (e.g., `"item_rarity*"`).
    #[must_use]
    pub fn new(keys: &[&str]) -> Self {
        let entries = keys
            .iter()
            .map(|k| {
                if let Some(prefix) = k.strip_suffix('*') {
                    SelectivityPattern::Prefix(prefix.to_owned())
                } else {
                    SelectivityPattern::Exact((*k).to_owned())
                }
            })
            .collect();
        Self { entries }
    }

    /// Returns the priority for a key if it matches a ranked entry.
    /// Lower number = higher priority (evaluated first).
    fn priority_for(&self, key: &str) -> Option<u8> {
        for (i, entry) in self.entries.iter().enumerate() {
            let matches = match entry {
                SelectivityPattern::Exact(k) => key == k,
                SelectivityPattern::Prefix(p) => key.starts_with(p),
            };
            if matches {
                return Some(i as u8);
            }
        }
        None
    }
}

/// A single node in the decision DAG.
struct DagNode {
    /// Condition to test. `None` for the root and for threshold-group children
    /// (whose integer condition is checked by the parent's threshold group).
    condition: Option<Condition>,

    /// Queries fully satisfied at this node — all ancestor conditions passed.
    terminal_queries: Vec<QueryId>,

    /// Child node IDs for non-integer conditions.
    children: Vec<NodeId>,

    /// Grouped integer conditions: sorted by threshold for binary-search pruning.
    /// Each group shares the same `(key, op)` — one `entry.get()` per group
    /// instead of one per threshold.
    threshold_groups: Vec<ThresholdGroup>,
}

/// A group of integer conditions sharing the same key and comparison operator.
/// Thresholds are sorted ascending for binary-search pruning.
struct ThresholdGroup {
    key: String,
    op: CompareOp,
    /// Sorted by threshold value ascending.
    entries: Vec<ThresholdEntry>,
}

struct ThresholdEntry {
    threshold: i64,
    target: NodeId,
}

/// Indexed query store using a decision DAG for shared condition evaluation.
///
/// Drop-in replacement for [`crate::store::QueryStore`]. Internally builds a DAG
/// of shared condition nodes so that common prefixes are evaluated once regardless
/// of how many queries share them.
///
/// Integer conditions are further optimized: conditions sharing the same `(key, operator)`
/// are grouped into sorted threshold arrays. A single `entry.get()` + binary search
/// replaces N individual node evaluations.
///
/// Use [`SelectivityConfig`] to control condition evaluation order based on
/// domain-specific knowledge of key selectivity.
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

    /// Domain-specific selectivity ranking for condition keys.
    selectivity: SelectivityConfig,
}

impl IndexedStore {
    #[must_use]
    pub fn new() -> Self {
        let mut store = Self::default();
        store.ensure_root();
        store
    }

    /// Create an indexed store with a domain-specific selectivity config.
    /// Keys listed in the config are evaluated first for maximum pruning.
    #[must_use]
    pub fn with_selectivity(config: SelectivityConfig) -> Self {
        let mut store = Self {
            selectivity: config,
            ..Self::default()
        };
        store.ensure_root();
        store
    }

    /// Add a reverse query. Returns its assigned ID.
    pub fn add(
        &mut self,
        conditions: Vec<Condition>,
        labels: Vec<String>,
        owner: Option<String>,
    ) -> QueryId {
        let id = self.next_id;
        self.next_id += 1;
        self.insert_query(id, conditions, labels, owner);
        id
    }

    /// Add a reverse query with a specific ID. Used when restoring from persistence.
    pub fn add_with_id(
        &mut self,
        id: QueryId,
        conditions: Vec<Condition>,
        labels: Vec<String>,
        owner: Option<String>,
    ) {
        self.insert_query(id, conditions, labels, owner);
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

        let canonical = canonicalize(&query.conditions, &self.selectivity);
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
            .filter(|n| !n.children.is_empty() || !n.threshold_groups.is_empty())
            .collect();
        if non_leaf.is_empty() {
            return 0.0;
        }
        let total: usize = non_leaf
            .iter()
            .map(|n| {
                n.children.len()
                    + n.threshold_groups
                        .iter()
                        .map(|g| g.entries.len())
                        .sum::<usize>()
            })
            .sum();
        total as f64 / non_leaf.len() as f64
    }

    /// Number of threshold groups across all nodes.
    #[must_use]
    pub fn threshold_group_count(&self) -> usize {
        self.nodes.iter().map(|n| n.threshold_groups.len()).sum()
    }

    // --- Internals ---

    fn ensure_root(&mut self) {
        if self.nodes.is_empty() {
            self.nodes.push(DagNode {
                condition: None,
                terminal_queries: Vec::new(),
                children: Vec::new(),
                threshold_groups: Vec::new(),
            });
            self.root = 0;
        }
    }

    fn alloc_node(&mut self, condition: Option<Condition>) -> NodeId {
        let id = self.nodes.len() as NodeId;
        self.nodes.push(DagNode {
            condition,
            terminal_queries: Vec::new(),
            children: Vec::new(),
            threshold_groups: Vec::new(),
        });
        id
    }

    fn insert_query(
        &mut self,
        id: QueryId,
        conditions: Vec<Condition>,
        labels: Vec<String>,
        owner: Option<String>,
    ) {
        self.ensure_root();

        let canonical = canonicalize(&conditions, &self.selectivity);

        // Walk/extend the DAG.
        let mut current = self.root;
        for condition in &canonical {
            current = self.insert_child(current, condition);
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
                owner,
            },
        );
    }

    /// Insert a condition as a child of `parent`, returning the child node ID.
    /// Integer conditions go into threshold groups; others into regular children.
    fn insert_child(&mut self, parent: NodeId, condition: &Condition) -> NodeId {
        // Integer conditions → threshold groups
        if let Value::Integer { value, op } = &condition.value {
            return self.insert_threshold(parent, &condition.key, *op, *value);
        }

        // Non-integer: look for existing child with same condition
        let existing = self.nodes[parent as usize]
            .children
            .iter()
            .find(|&&child_id| self.nodes[child_id as usize].condition.as_ref() == Some(condition))
            .copied();

        if let Some(child_id) = existing {
            child_id
        } else {
            let new_id = self.alloc_node(Some(condition.clone()));
            self.nodes[parent as usize].children.push(new_id);
            new_id
        }
    }

    /// Insert an integer threshold into a threshold group on `parent`.
    /// Creates the group if it doesn't exist. Returns the child node ID.
    fn insert_threshold(
        &mut self,
        parent: NodeId,
        key: &str,
        op: CompareOp,
        threshold: i64,
    ) -> NodeId {
        // Find or create the threshold group for (key, op)
        let group_idx = self.nodes[parent as usize]
            .threshold_groups
            .iter()
            .position(|g| g.key == key && g.op == op);

        let group_idx = group_idx.unwrap_or_else(|| {
            let idx = self.nodes[parent as usize].threshold_groups.len();
            self.nodes[parent as usize]
                .threshold_groups
                .push(ThresholdGroup {
                    key: key.to_owned(),
                    op,
                    entries: Vec::new(),
                });
            idx
        });

        // Find existing entry with this threshold, or insert new
        let entries = &self.nodes[parent as usize].threshold_groups[group_idx].entries;
        let pos = entries.partition_point(|e| e.threshold < threshold);

        if pos < entries.len()
            && self.nodes[parent as usize].threshold_groups[group_idx].entries[pos].threshold
                == threshold
        {
            // Existing threshold — reuse its target node
            self.nodes[parent as usize].threshold_groups[group_idx].entries[pos].target
        } else {
            // New threshold — allocate node (condition = None, group handles the check)
            let new_id = self.alloc_node(None);
            self.nodes[parent as usize].threshold_groups[group_idx]
                .entries
                .insert(
                    pos,
                    ThresholdEntry {
                        threshold,
                        target: new_id,
                    },
                );
            new_id
        }
    }

    fn remove_from_dag(&mut self, id: QueryId, canonical: &[Condition]) {
        // Collect the path: sequence of (node_id, is_threshold) from root to terminal.
        let mut path: Vec<(NodeId, bool)> = vec![(self.root, false)];
        let mut current = self.root;

        for condition in canonical {
            if let Value::Integer { value, op } = &condition.value {
                // Look in threshold groups
                let child = self.find_threshold_child(current, &condition.key, *op, *value);
                if let Some(child_id) = child {
                    path.push((child_id, true));
                    current = child_id;
                } else {
                    return;
                }
            } else {
                // Look in regular children
                let child = self.nodes[current as usize]
                    .children
                    .iter()
                    .find(|&&child_id| {
                        self.nodes[child_id as usize].condition.as_ref() == Some(condition)
                    })
                    .copied();

                if let Some(child_id) = child {
                    path.push((child_id, false));
                    current = child_id;
                } else {
                    return;
                }
            }
        }

        // Remove query ID from terminal node.
        let (terminal, _) = *path.last().expect("path is non-empty");
        self.nodes[terminal as usize]
            .terminal_queries
            .retain(|&q| q != id);

        // Prune empty nodes bottom-up.
        for i in (1..path.len()).rev() {
            let (node_id, is_threshold) = path[i];
            let node = &self.nodes[node_id as usize];
            if !node.terminal_queries.is_empty()
                || !node.children.is_empty()
                || !node.threshold_groups.is_empty()
            {
                break; // Node still has content
            }

            let (parent_id, _) = path[i - 1];
            if is_threshold {
                // Remove from parent's threshold group
                for group in &mut self.nodes[parent_id as usize].threshold_groups {
                    group.entries.retain(|e| e.target != node_id);
                }
                // Remove empty groups
                self.nodes[parent_id as usize]
                    .threshold_groups
                    .retain(|g| !g.entries.is_empty());
            } else {
                self.nodes[parent_id as usize]
                    .children
                    .retain(|&c| c != node_id);
            }
        }
    }

    /// Find a child node in a threshold group matching (key, op, threshold).
    fn find_threshold_child(
        &self,
        parent: NodeId,
        key: &str,
        op: CompareOp,
        threshold: i64,
    ) -> Option<NodeId> {
        for group in &self.nodes[parent as usize].threshold_groups {
            if group.key == key && group.op == op {
                let pos = group.entries.partition_point(|e| e.threshold < threshold);
                if pos < group.entries.len() && group.entries[pos].threshold == threshold {
                    return Some(group.entries[pos].target);
                }
            }
        }
        None
    }

    fn walk(&self, node_id: NodeId, entry: &Entry, results: &mut Vec<QueryId>) {
        let node = &self.nodes[node_id as usize];

        // Test this node's condition (root and threshold children have None → always pass).
        if let Some(condition) = &node.condition {
            if !evaluate_one(condition, entry) {
                return; // Prune entire subtree.
            }
        }

        // Collect terminal queries.
        results.extend_from_slice(&node.terminal_queries);

        // Recurse into regular children.
        for &child_id in &node.children {
            self.walk(child_id, entry, results);
        }

        // Process threshold groups: one entry.get() + binary search per group.
        for group in &node.threshold_groups {
            let Some(EntryValue::Integer(entry_val)) = entry.get(&group.key) else {
                // Key missing or not an integer → no thresholds pass
                continue;
            };

            let passing = passing_range(&group.entries, *entry_val, group.op);
            for te in passing {
                self.walk(te.target, entry, results);
            }
        }
    }

    fn depth_of(&self, node_id: NodeId) -> usize {
        let node = &self.nodes[node_id as usize];
        let child_max = node
            .children
            .iter()
            .map(|&c| self.depth_of(c))
            .max()
            .unwrap_or(0);
        let threshold_max = node
            .threshold_groups
            .iter()
            .flat_map(|g| g.entries.iter().map(|e| self.depth_of(e.target)))
            .max()
            .unwrap_or(0);

        let max_child = child_max.max(threshold_max);
        if max_child == 0 && node.children.is_empty() && node.threshold_groups.is_empty() {
            0
        } else {
            1 + max_child
        }
    }
}

// --- Threshold binary search ---

/// Return the slice of threshold entries that pass for the given entry value and operator.
///
/// Entries are sorted by threshold ascending. The comparison semantics follow the
/// Erlang convention: `rq_value <op> entry_value`.
///
/// - `Lt`: `threshold < entry_value` → all thresholds below `entry_value`
/// - `Lte`: `threshold <= entry_value` → all thresholds at or below
/// - `Gt`: `threshold > entry_value` → all thresholds above
/// - `Gte`: `threshold >= entry_value` → all thresholds at or above
/// - `Eq`: `threshold == entry_value` → only exact match
fn passing_range(entries: &[ThresholdEntry], entry_val: i64, op: CompareOp) -> &[ThresholdEntry] {
    match op {
        CompareOp::Lt => {
            // threshold < entry_val: all entries where threshold < entry_val
            let end = entries.partition_point(|e| e.threshold < entry_val);
            &entries[..end]
        }
        CompareOp::Lte => {
            let end = entries.partition_point(|e| e.threshold <= entry_val);
            &entries[..end]
        }
        CompareOp::Gt => {
            // threshold > entry_val: all entries where threshold > entry_val
            let start = entries.partition_point(|e| e.threshold <= entry_val);
            &entries[start..]
        }
        CompareOp::Gte => {
            let start = entries.partition_point(|e| e.threshold < entry_val);
            &entries[start..]
        }
        CompareOp::Eq => {
            let pos = entries.partition_point(|e| e.threshold < entry_val);
            if pos < entries.len() && entries[pos].threshold == entry_val {
                &entries[pos..=pos]
            } else {
                &[]
            }
        }
    }
}

// --- Canonicalization ---

/// Flatten AND lists and sort conditions into canonical order for maximum DAG sharing.
fn canonicalize(conditions: &[Condition], selectivity: &SelectivityConfig) -> Vec<Condition> {
    let mut flat = Vec::new();
    flatten_and(conditions, &mut flat);
    flat.sort_by(|a, b| condition_ordering(a, b, selectivity));
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
            flatten_and(inner, out);
        } else {
            out.push(cond.clone());
        }
    }
}

/// Condition priority using selectivity config.
///
/// If the condition's key matches a ranked entry in the config, that rank is used.
/// Otherwise, default type-based priorities apply (offset past all config entries):
///   - string exact match
///   - string wildcard (existence check)
///   - boolean
///   - integer
///   - compound (NOT/OR/COUNT lists)
fn condition_priority(cond: &Condition, selectivity: &SelectivityConfig) -> u8 {
    // Check selectivity config first
    if let Some(p) = selectivity.priority_for(&cond.key) {
        return p;
    }
    // Default type-based priorities (offset by max possible config entries)
    let base = selectivity.entries.len() as u8;
    match &cond.value {
        Value::String(crate::predicate::StringMatch::Exact(_)) => base,
        Value::String(crate::predicate::StringMatch::Wildcard) => base + 1,
        Value::Boolean(_) => base + 2,
        Value::Integer { .. } => base + 3,
        Value::List { .. } => base + 4,
    }
}

/// Ordering function for canonical condition sort.
fn condition_ordering(a: &Condition, b: &Condition, selectivity: &SelectivityConfig) -> Ordering {
    let pa = condition_priority(a, selectivity);
    let pb = condition_priority(b, selectivity);
    pa.cmp(&pb)
        .then_with(|| a.key.cmp(&b.key))
        .then_with(|| value_sort_key(&a.value).cmp(&value_sort_key(&b.value)))
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
    use crate::predicate::{CompareOp, StringMatch};
    use crate::store::QueryStore;

    // --- Product marketplace helpers ---

    /// Cheap electronics product: category=Electronics, `in_stock=true`, price=299, weight=2, rating=4
    fn electronics_entry() -> Entry {
        serde_json::from_str(
            r#"{"category": "Electronics", "in_stock": true, "on_sale": false, "price": 299, "weight": 2, "rating": 4, "color": "Black"}"#,
        )
        .unwrap()
    }

    /// Clothing product: category=Clothing, `in_stock=true`, `on_sale=true`, price=49, weight=1, rating=5
    fn clothing_entry() -> Entry {
        serde_json::from_str(
            r#"{"category": "Clothing", "in_stock": true, "on_sale": true, "price": 49, "weight": 1, "rating": 5, "color": "Red"}"#,
        )
        .unwrap()
    }

    /// Book product: category=Books, `in_stock=false`, price=15, weight=1, rating=3
    fn book_entry() -> Entry {
        serde_json::from_str(
            r#"{"category": "Books", "in_stock": false, "on_sale": false, "price": 15, "weight": 1, "rating": 3}"#,
        )
        .unwrap()
    }

    /// Query: category=Electronics, `in_stock=true`
    fn want_electronics_in_stock() -> Vec<Condition> {
        serde_json::from_str(
            r#"[
                {"key": "category", "value": "Electronics", "type": "string", "typeOptions": null},
                {"key": "in_stock", "value": true, "type": "boolean", "typeOptions": null}
            ]"#,
        )
        .unwrap()
    }

    /// Query: category=Electronics, price < 500 (Erlang: 500 < `entry_price`, i.e. price under 500)
    fn want_cheap_electronics() -> Vec<Condition> {
        serde_json::from_str(
            r#"[
                {"key": "category", "value": "Electronics", "type": "string", "typeOptions": null},
                {"key": "price", "value": 500, "type": "integer", "typeOptions": {"operator": ">"}}
            ]"#,
        )
        .unwrap()
    }

    /// Query: category=Clothing, `on_sale=true`
    fn want_clothing_on_sale() -> Vec<Condition> {
        serde_json::from_str(
            r#"[
                {"key": "category", "value": "Clothing", "type": "string", "typeOptions": null},
                {"key": "on_sale", "value": true, "type": "boolean", "typeOptions": null}
            ]"#,
        )
        .unwrap()
    }

    /// Query: category=Clothing, `NOT(on_sale=true)` — wants non-sale clothing
    fn want_clothing_not_on_sale() -> Vec<Condition> {
        serde_json::from_str(
            r#"[
                {"key": "category", "value": "Clothing", "type": "string", "typeOptions": null},
                {"key": "list", "value": [
                    {"key": "on_sale", "value": true, "type": "boolean", "typeOptions": null}
                ], "type": "list", "typeOptions": {"operator": "not"}}
            ]"#,
        )
        .unwrap()
    }

    // --- CRUD tests ---

    #[test]
    fn add_and_remove() {
        let mut store = IndexedStore::new();
        assert!(store.is_empty());

        let id = store.add(want_electronics_in_stock(), vec![], None);
        assert_eq!(store.len(), 1);
        assert!(store.get(id).is_some());

        assert!(store.remove(id));
        assert!(store.is_empty());
        assert!(!store.remove(id));
    }

    #[test]
    fn match_single_query() {
        let mut store = IndexedStore::new();
        let id = store.add(want_electronics_in_stock(), vec![], None);

        let matches = store.match_item(&electronics_entry());
        assert_eq!(matches, vec![id]);

        // Book is not electronics
        let matches = store.match_item(&book_entry());
        assert!(matches.is_empty());
    }

    #[test]
    fn match_multiple_queries() {
        let mut store = IndexedStore::new();
        let id_stock = store.add(want_electronics_in_stock(), vec![], None);
        let id_cheap = store.add(want_cheap_electronics(), vec![], None);
        let _id_clothing = store.add(want_clothing_on_sale(), vec![], None);

        // Electronics entry matches both electronics queries but not clothing
        let mut matches = store.match_item(&electronics_entry());
        matches.sort_unstable();
        let mut expected = vec![id_stock, id_cheap];
        expected.sort_unstable();
        assert_eq!(matches, expected);
    }

    #[test]
    fn match_no_queries_for_unrelated_item() {
        let mut store = IndexedStore::new();
        store.add(want_electronics_in_stock(), vec![], None);
        store.add(want_cheap_electronics(), vec![], None);

        // Book is not electronics
        let matches = store.match_item(&book_entry());
        assert!(matches.is_empty());
    }

    #[test]
    fn match_with_labels() {
        let mut store = IndexedStore::new();
        let id = store.add(
            want_electronics_in_stock(),
            vec!["wishlist:gaming".into(), "priority:high".into()],
            None,
        );

        let query = store.get(id).unwrap();
        assert_eq!(query.labels, vec!["wishlist:gaming", "priority:high"]);
    }

    #[test]
    fn ids_are_unique_and_sequential() {
        let mut store = IndexedStore::new();
        let id0 = store.add(want_electronics_in_stock(), vec![], None);
        let id1 = store.add(want_cheap_electronics(), vec![], None);
        let id2 = store.add(want_clothing_on_sale(), vec![], None);
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
            want_clothing_not_on_sale(),
        ];
        let entries = vec![electronics_entry(), clothing_entry(), book_entry()];

        let mut store = IndexedStore::new();
        for q in &queries {
            store.add(q.clone(), vec![], None);
        }
        assert_eq!(store.len(), 4);

        for entry in &entries {
            let matches = store.match_item(entry);
            for id in &matches {
                assert!(store.get(*id).is_some());
            }
        }
    }

    // --- Equivalence: IndexedStore must match QueryStore exactly ---

    #[test]
    fn equivalence_with_brute_force() {
        let queries: Vec<Vec<Condition>> = vec![
            want_electronics_in_stock(),
            want_cheap_electronics(),
            want_clothing_on_sale(),
            want_clothing_not_on_sale(),
            // Additional queries for broader coverage
            serde_json::from_str(
                r#"[{"key": "rating", "value": 3, "type": "integer", "typeOptions": {"operator": "<"}}]"#,
            ).unwrap(),
            serde_json::from_str(
                r#"[{"key": "price", "value": 20, "type": "integer", "typeOptions": {"operator": "<"}},
                    {"key": "weight", "value": 2, "type": "integer", "typeOptions": {"operator": ">"}}]"#,
            ).unwrap(),
            serde_json::from_str(
                r#"[{"key": "color", "value": "_", "type": "string", "typeOptions": null}]"#,
            ).unwrap(),
            serde_json::from_str(
                r#"[{"key": "list", "value": [
                        {"key": "price", "value": 100, "type": "integer", "typeOptions": {"operator": "<"}},
                        {"key": "rating", "value": 4, "type": "integer", "typeOptions": {"operator": "<"}}
                    ], "type": "list", "typeOptions": {"operator": "count", "count": 1}}]"#,
            ).unwrap(),
        ];
        let entries = [electronics_entry(), clothing_entry(), book_entry()];

        let mut brute = QueryStore::new();
        let mut indexed = IndexedStore::new();

        for q in &queries {
            brute.add(q.clone(), vec![], None);
            indexed.add(q.clone(), vec![], None);
        }

        for (i, entry) in entries.iter().enumerate() {
            let mut brute_matches = brute.match_item(entry);
            let mut indexed_matches = indexed.match_item(entry);

            brute_matches.sort_unstable();
            indexed_matches.sort_unstable();

            assert_eq!(
                brute_matches, indexed_matches,
                "mismatch for entry index {i}"
            );
        }
    }

    // --- DAG diagnostics ---

    #[test]
    fn dag_diagnostics() {
        let mut store = IndexedStore::new();
        assert_eq!(store.node_count(), 1); // root only
        assert_eq!(store.max_depth(), 0);

        store.add(want_electronics_in_stock(), vec![], None);
        store.add(want_cheap_electronics(), vec![], None);

        assert!(store.node_count() > 1);
        assert!(store.max_depth() >= 2);
        assert!(store.avg_branching_factor() > 0.0);

        println!(
            "DAG: {} nodes, depth {}, avg branching {:.2}, {} threshold groups",
            store.node_count(),
            store.max_depth(),
            store.avg_branching_factor(),
            store.threshold_group_count(),
        );
    }

    // --- Removal prunes empty nodes ---

    #[test]
    fn remove_prunes_dag() {
        let mut store = IndexedStore::new();
        let id = store.add(want_electronics_in_stock(), vec![], None);

        let nodes_before = store.node_count();
        assert!(nodes_before > 1);

        store.remove(id);

        // Root should have no children or threshold groups after full removal.
        let root = &store.nodes[store.root as usize];
        assert!(root.children.is_empty());
        assert!(root.threshold_groups.is_empty());
    }

    // --- Threshold groups ---

    #[test]
    fn threshold_grouping_reduces_nodes() {
        // Two queries with same (key, op) but different thresholds should share a group.
        let mut store = IndexedStore::new();

        let q1: Vec<Condition> = serde_json::from_str(
            r#"[
            {"key": "price", "value": 40, "type": "integer", "typeOptions": {"operator": "<"}},
            {"key": "weight", "value": 20, "type": "integer", "typeOptions": {"operator": "<"}}
        ]"#,
        )
        .unwrap();

        let q2: Vec<Condition> = serde_json::from_str(
            r#"[
            {"key": "price", "value": 60, "type": "integer", "typeOptions": {"operator": "<"}},
            {"key": "weight", "value": 30, "type": "integer", "typeOptions": {"operator": "<"}}
        ]"#,
        )
        .unwrap();

        store.add(q1, vec![], None);
        store.add(q2, vec![], None);

        assert!(store.threshold_group_count() > 0);
        println!(
            "Threshold test: {} nodes, {} groups",
            store.node_count(),
            store.threshold_group_count(),
        );
    }

    #[test]
    fn threshold_binary_search_correctness() {
        let mut store = IndexedStore::new();

        // Add queries with various price thresholds
        for threshold in [10, 20, 30, 40, 50, 60, 70, 80] {
            let q: Vec<Condition> = serde_json::from_str(&format!(
                r#"[{{"key": "price", "value": {threshold}, "type": "integer", "typeOptions": {{"operator": "<"}}}}]"#
            )).unwrap();
            store.add(q, vec![], None);
        }

        // Entry with price=45 should match thresholds < 45: [10, 20, 30, 40]
        let entry: Entry = serde_json::from_str(r#"{"price": 45}"#).unwrap();
        let matches = store.match_item(&entry);
        assert_eq!(matches.len(), 4, "should match thresholds 10,20,30,40");

        // Entry with price=100 should match all 8
        let entry: Entry = serde_json::from_str(r#"{"price": 100}"#).unwrap();
        let matches = store.match_item(&entry);
        assert_eq!(matches.len(), 8, "should match all thresholds");

        // Entry with price=5 should match none (no threshold < 5)
        let entry: Entry = serde_json::from_str(r#"{"price": 5}"#).unwrap();
        let matches = store.match_item(&entry);
        assert_eq!(matches.len(), 0, "should match no thresholds");
    }

    // --- AND flattening ---

    #[test]
    fn and_flattening_shares_conditions() {
        // AND list with two price conditions should be flattened into threshold groups
        let q: Vec<Condition> = serde_json::from_str(
            r#"[
                {"key": "category", "value": "Electronics", "type": "string", "typeOptions": null},
                {"key": "list", "value": [
                    {"key": "price", "value": 100, "type": "integer", "typeOptions": {"operator": "<"}},
                    {"key": "price", "value": 500, "type": "integer", "typeOptions": {"operator": ">"}}
                ], "type": "list", "typeOptions": {"operator": "and"}}
            ]"#,
        )
        .unwrap();

        let mut store = IndexedStore::new();
        store.add(q, vec![], None);

        // The AND list [price < 100, price > 500] is flattened.
        // Integer conditions go into threshold groups, not regular children.
        assert!(store.threshold_group_count() > 0);
    }

    // --- Canonicalization ---

    #[test]
    fn canonicalize_sorts_by_priority() {
        // With SelectivityConfig, ranked keys come first, then type-based defaults.
        let config = SelectivityConfig::new(&["category", "color"]);

        let conditions = vec![
            Condition {
                key: "price".into(),
                value: Value::Integer {
                    value: 50,
                    op: CompareOp::Gt,
                },
            },
            Condition {
                key: "in_stock".into(),
                value: Value::Boolean(true),
            },
            Condition {
                key: "category".into(),
                value: Value::String(StringMatch::Exact("Electronics".into())),
            },
            Condition {
                key: "color".into(),
                value: Value::String(StringMatch::Exact("Red".into())),
            },
        ];

        let result = canonicalize(&conditions, &config);
        // category has config priority 0, color has config priority 1
        assert_eq!(result[0].key, "category");
        assert_eq!(result[1].key, "color");
        // Then type-based: boolean before integer
        assert_eq!(result[2].key, "in_stock");
        assert_eq!(result[3].key, "price");
    }

    #[test]
    fn canonicalize_default_sorts_by_type() {
        // With empty SelectivityConfig, all keys use type-based defaults.
        let config = SelectivityConfig::default();

        let conditions = vec![
            Condition {
                key: "price".into(),
                value: Value::Integer {
                    value: 50,
                    op: CompareOp::Gt,
                },
            },
            Condition {
                key: "in_stock".into(),
                value: Value::Boolean(true),
            },
            Condition {
                key: "category".into(),
                value: Value::String(StringMatch::Exact("Electronics".into())),
            },
        ];

        let result = canonicalize(&conditions, &config);
        // Type-based: string exact (0) → boolean (2) → integer (3)
        assert_eq!(result[0].key, "category");
        assert_eq!(result[1].key, "in_stock");
        assert_eq!(result[2].key, "price");
    }

    #[test]
    fn canonicalize_flattens_nested_and() {
        let config = SelectivityConfig::default();

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

        let result = canonicalize(&conditions, &config);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].key, "a");
        assert_eq!(result[1].key, "b");
        assert_eq!(result[2].key, "c");
    }

    #[test]
    fn canonicalize_preserves_not_list() {
        let config = SelectivityConfig::default();

        let not_list = Condition {
            key: "list".into(),
            value: Value::List {
                op: ListOp::Not,
                conditions: vec![Condition {
                    key: "weight".into(),
                    value: Value::Integer {
                        value: 10,
                        op: CompareOp::Gt,
                    },
                }],
            },
        };

        let not_vec = vec![not_list.clone()];
        let result = canonicalize(&not_vec, &config);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], not_list);
    }

    #[test]
    fn selectivity_config_prefix_matching() {
        let config = SelectivityConfig::new(&["category", "color*"]);
        // Exact match
        assert_eq!(config.priority_for("category"), Some(0));
        // Prefix match
        assert_eq!(config.priority_for("color"), Some(1));
        assert_eq!(config.priority_for("color_primary"), Some(1));
        // No match
        assert!(config.priority_for("price").is_none());
    }

    #[test]
    fn with_selectivity_constructor() {
        let config = SelectivityConfig::new(&["category"]);
        let store = IndexedStore::with_selectivity(config);
        assert!(store.is_empty());
        assert_eq!(store.node_count(), 1); // root allocated
    }
}
