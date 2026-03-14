# RQE Decision DAG — Indexed Matching Design

## Problem

At 200k users × 5-10 queries each = **1-2 million registered queries**, brute-force evaluation
is not viable. The benchmark shows cache pressure killing throughput at 50k queries (66M → 5M
evals/sec). At 1M queries, brute-force would be orders of magnitude slower.

We need a structure that evaluates shared conditions **once** regardless of how many queries
share them.

## Key Simplification: Not Full Rete

Classic Rete solves: "given N facts in working memory, find all matching rule activations,
including joins across multiple facts." That requires an alpha network (single-condition tests)
AND a beta network (multi-fact joins).

Our problem is simpler: **one item, many queries**. No joins. We only need the alpha network —
a DAG of shared condition tests. This eliminates the majority of Rete's implementation
complexity (join nodes, token memory, truth maintenance).

What we're building is a **decision DAG**: a directed acyclic graph where each node tests one
condition, edges lead to child nodes, and terminal nodes carry query IDs. Shared condition
prefixes are evaluated once.

## Data Model

```
item arrives
    ├─ item_category = "Crimson Jewel"?
    │   ├─ item_rarity_2 = "Non-Unique"?
    │   │   ├─ armor < 4?
    │   │   │   └─ armor > 20? → {Q_mod}
    │   │   ├─ item_rarity = "Rare"?
    │   │   │   └─ name = "_"? → {Q_rare}
    │   │   └─ ...
    │   └─ ...
    ├─ item_category = "Boots"?
    │   └─ ...
    └─ item_rarity_2 = "Non-Unique"?  ← queries without category
        └─ ...
```

If 500,000 queries check `item_category = "Crimson Jewel"` and the item is a Ring,
**one comparison** prunes all 500,000. With brute-force, that's 500,000 wasted comparisons.

## Data Structures

```rust
type NodeId = u32;  // arena index

/// A single node in the decision DAG.
struct DagNode {
    /// Condition to test. None for the root (always passes).
    condition: Option<Condition>,

    /// Queries fully satisfied at this node.
    /// (All conditions along the path from root to here have passed.)
    terminal_queries: SmallVec<[QueryId; 2]>,

    /// Child nodes — each with an additional condition to test.
    children: Vec<NodeId>,
}

/// The indexed query store. Drop-in replacement for QueryStore.
pub struct IndexedStore {
    /// Arena-allocated DAG nodes. Cache-friendly sequential memory.
    nodes: Vec<DagNode>,

    /// Root node ID (always 0).
    root: NodeId,

    /// Query metadata: original conditions + labels.
    /// Used for get(), removal (re-walking the path), and API responses.
    queries: HashMap<QueryId, StoredQuery>,

    /// Next auto-increment ID.
    next_id: QueryId,
}
```

### Why Arena Allocation

`Vec<DagNode>` with `NodeId = u32` indices instead of `Box<DagNode>` pointers:
- Sequential memory layout → cache-friendly traversal
- No pointer chasing → better prefetch behavior
- 4 bytes per reference instead of 8
- At 1M queries with ~5 conditions each, expect ~500k-2M nodes (sharing reduces this).
  At ~64 bytes per node, that's 32-128 MB — fits comfortably in memory.

## Condition Canonicalization

For the DAG to share effectively, all queries must order their conditions identically.
A canonical ordering ensures that queries checking the same conditions in any order
converge to the same DAG path.

### Priority Scheme

```
Priority 0: item_category (string eq)     — highest selectivity, most shared
Priority 1: item_rarity* keys (string eq)  — second most common discriminator
Priority 2: other string equality           — base_type, name, etc.
Priority 3: string wildcard                 — existence checks
Priority 4: boolean conditions              — rarity flags, corrupted, etc.
Priority 5: integer conditions              — stat thresholds
Priority 6: compound (NOT/OR/COUNT lists)   — most expensive, evaluated last
```

Within the same priority: sort by `key` alphabetically, then by value for determinism.

This ordering maximizes early pruning: the most selective, cheapest conditions come first.
Compound conditions (which may involve multiple sub-evaluations) come last, evaluated only
on the smallest surviving candidate set.

### AND Flattening

AND lists are semantically equivalent to additional top-level conditions (all must match).
Flattening them exposes their sub-conditions to the DAG for individual sharing:

```
Before: [category = "Crimson", AND([armor < 4, armor > 20])]
After:  [category = "Crimson", armor < 4, armor > 20]
```

If another query also checks `armor < 4`, they share that node. Without flattening,
the entire AND list would be an opaque node — no sharing of its internals.

**Only AND lists are flattened.** NOT, OR, and COUNT have different semantics and stay
as atomic nodes:
- `NOT([a, b])` = neither a nor b matches (≠ `!a AND !b` when keys are missing)
- `OR([a, b])` = any matches (≠ `a AND b`)
- `COUNT(n)([...])` = exactly n match (can't decompose)

Flattening is recursive: `AND([AND([a, b]), c])` → `[a, b, c]`.

## Algorithms

### Insert

```
fn insert(query_id, conditions, labels):
    1. Store query metadata in queries HashMap
    2. Flatten AND lists recursively
    3. Sort conditions by canonical order
    4. Walk DAG from root:
       for each condition in sorted order:
           find child edge with matching condition (linear scan)
           if found: follow it
           if not: allocate new node, add as child
    5. At terminal node: push query_id into terminal_queries
```

**Complexity:** O(D) where D = number of conditions after flattening (typically 3-8).
Edge scan is O(C) where C = children of current node, but C is small in practice.

### Remove

```
fn remove(query_id):
    1. Look up stored conditions in queries HashMap
    2. Flatten + canonicalize (same as insert)
    3. Walk DAG following the same path, collecting (parent, child_index) pairs
    4. At terminal: remove query_id from terminal_queries
    5. Prune: walk backward through collected path
       if node has no terminal_queries AND no children:
           remove from parent's children list
    6. Remove from queries HashMap
```

**Complexity:** O(D) for the walk + O(D) for pruning = O(D).

### Match

```
fn match_item(entry) -> Vec<QueryId>:
    let results = vec![]
    walk(root, entry, &mut results)
    return results

fn walk(node_id, entry, results):
    let node = &nodes[node_id]

    // Test this node's condition (root has None → always passes)
    if let Some(condition) = &node.condition:
        if !evaluate_one(condition, entry):
            return  // PRUNE entire subtree

    // Collect terminal queries
    results.extend(&node.terminal_queries)

    // Recurse into all children
    for &child_id in &node.children:
        walk(child_id, entry, results)
```

**Complexity:** O(V) where V = number of nodes visited. With good canonicalization,
this is a small fraction of total nodes — most subtrees are pruned at the first or
second level.

The `evaluate_one` function is reused directly from `eval.rs`. The DAG doesn't need
to understand condition semantics — it just controls evaluation order and sharing.

## Concrete Example

Given these three queries from the Erlang test data:

**Q_rare** (wanted_crimson_rare):
```
item_rarity_2 = "Non-Unique"
item_category = "Crimson Jewel"
item_rarity = "Rare"
name = "_"
```

**Q_mod** (wanted_crimson_mod):
```
item_category = "Crimson Jewel"
item_rarity_2 = "Non-Unique"
AND([armor < 4, armor > 20])
```

**Q_complex** (wanted_mod_and_not_count):
```
item_rarity_2 = "Non-Unique"
rarity_rare = true
item_category = "_"
name = "_"
NOT([armor < 4, armor > 20])
AND([fire_cold_res < 4, fire_cold_res > 20])
COUNT(1)([attack_speed <= 4, fire_cold_res >= 10])
```

### After Canonicalization

**Q_rare** (AND-flatten: nothing to flatten):
```
P0: item_category = "Crimson Jewel"
P1: item_rarity_2 = "Non-Unique"
P2: item_rarity = "Rare"
P3: name = "_"
```

**Q_mod** (AND-flatten: armor conditions extracted):
```
P0: item_category = "Crimson Jewel"
P1: item_rarity_2 = "Non-Unique"
P5: armor < 4
P5: armor > 20
```

**Q_complex** (AND-flatten: fire_cold_res extracted; NOT and COUNT stay opaque):
```
P0: item_category = "_"
P1: item_rarity_2 = "Non-Unique"
P3: name = "_"
P4: rarity_rare = true
P5: fire_cold_res < 4
P5: fire_cold_res > 20
P6: NOT([armor < 4, armor > 20])
P6: COUNT(1)([attack_speed <= 4, fire_cold_res >= 10])
```

### Resulting DAG

```
root
├─ item_category = "Crimson Jewel"
│   └─ item_rarity_2 = "Non-Unique"
│       ├─ item_rarity = "Rare"
│       │   └─ name = "_" → {Q_rare}
│       ├─ armor < 4
│       │   └─ armor > 20 → {Q_mod}
│       └─ ...
├─ item_category = "_"
│   └─ item_rarity_2 = "Non-Unique"
│       └─ name = "_"
│           └─ rarity_rare = true
│               └─ fire_cold_res < 4
│                   └─ fire_cold_res > 20
│                       └─ NOT([armor range])
│                           └─ COUNT(1)([...]) → {Q_complex}
└─ ...
```

**Sharing:** Q_rare and Q_mod share the `item_category = "Crimson Jewel"` →
`item_rarity_2 = "Non-Unique"` path. Those two condition evaluations happen once,
not twice (or 500,000 times if many queries share them).

### Match Walk-Through

Item: `crimson_w_mods_1.json` — Crimson Jewel, Rare, Non-Unique, armor=15

1. **Root**: always passes
2. **item_category = "Crimson Jewel"**: item has category "Crimson Jewel" → **PASS**
   - **item_rarity_2 = "Non-Unique"**: item has "Non-Unique" → **PASS**
     - **item_rarity = "Rare"**: item has "Rare" → **PASS**
       - **name = "_"**: item has name "Chimeric Spark", wildcard matches → **PASS**
         - Terminal: **Q_rare matches** ✓
     - **armor < 4**: Erlang semantics: 4 < 15 → **PASS**
       - **armor > 20**: 20 > 15 → **PASS**
         - Terminal: **Q_mod matches** ✓
3. **item_category = "_"**: item has category → **PASS**
   - **item_rarity_2 = "Non-Unique"**: **PASS**
     - **name = "_"**: **PASS**
       - **rarity_rare = true**: item has `rarity_rare = false` → **FAIL**
         - **Entire subtree pruned** — Q_complex skipped

Result: {Q_rare, Q_mod} ✓

## Scale Analysis

### 200k Users × 10 Queries = 2M Queries

**PoE query clustering:** Most players want similar things — life, resists, DPS stats on
rare armor/weapons/jewelry. Expect heavy overlap in conditions.

**Estimated DAG characteristics:**
- ~20 item categories account for 80%+ of queries → Level 1 has ~20-30 branches
- ~3 rarity values → Level 2 has ~3-5 branches per category
- Stat conditions diverge more, but common stats (life, resists) still cluster
- Estimated unique DAG nodes: **50k-200k** (vs 2M × 5 = 10M condition evaluations brute-force)
- Estimated depth: 3-8 levels per query
- Match time per item: walk ~100-500 nodes (vs 10M condition evaluations)

**That's a 20,000-100,000x reduction in work per item.**

### Memory

- 200k nodes × ~96 bytes = ~20 MB for the DAG
- 2M queries × ~200 bytes metadata = ~400 MB for the HashMap
- Total: ~420 MB — comfortable for a Cloud Run instance with 1-2 GB RAM

### Throughput

The brute-force benchmark shows 66M evals/sec for 100 queries (all in L1 cache).
With the DAG, each item walks ~100-500 nodes (all in L2/L3 cache at 20 MB).
Expected throughput: **1-10M items/sec** per core, even at 2M total queries.

## NOT/OR/COUNT Handling

These compound conditions stay as opaque DAG nodes. They can still participate in sharing
(two queries with identical NOT clauses share a node), but they're not decomposed.

They're placed last in canonical order (Priority 6) so they're evaluated only after
all simpler conditions have pruned the candidate set. At that point, the surviving
queries are few and the compound evaluation cost is negligible.

The existing `evaluate_one()` from `eval.rs` handles them correctly — the DAG node
just delegates to it like any other condition.

## Integration with Existing Code

### File Structure

```
crates/poe-rqe/src/
    predicate.rs  — unchanged
    eval.rs       — unchanged (evaluate_one reused by DAG)
    store.rs      — unchanged (brute-force, kept as reference for testing)
    index.rs      — NEW: IndexedStore + DagNode
    lib.rs        — add `pub mod index;`
```

### API Compatibility

`IndexedStore` exposes the same public API as `QueryStore`:

```rust
impl IndexedStore {
    pub fn new() -> Self;
    pub fn add(&mut self, conditions: Vec<Condition>, labels: Vec<String>) -> QueryId;
    pub fn remove(&mut self, id: QueryId) -> bool;
    pub fn get(&self, id: QueryId) -> Option<&StoredQuery>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn match_item(&self, entry: &Entry) -> Vec<QueryId>;
}
```

### Testing Strategy

1. **Same tests as QueryStore**: All 9 existing store tests should pass with IndexedStore
2. **Equivalence testing**: For any set of queries and entries,
   `IndexedStore::match_item()` must return the same results as `QueryStore::match_item()`
3. **Fuzz testing**: Random queries + entries, verify IndexedStore matches brute-force
4. **Benchmark**: Update `match_throughput.rs` to compare brute-force vs indexed at
   100, 1k, 10k, 50k, 100k, 500k, 1M queries

### DAG Diagnostics

```rust
impl IndexedStore {
    /// Total nodes in the DAG.
    pub fn node_count(&self) -> usize;

    /// Maximum depth of the DAG.
    pub fn max_depth(&self) -> usize;

    /// Average branching factor.
    pub fn avg_branching_factor(&self) -> f64;

    /// Number of shared nodes (nodes with >1 query path through them).
    pub fn shared_node_count(&self) -> usize;
}
```

## Future Optimizations (Not Now)

These are worth investigating once the baseline DAG is working and profiled:

1. **Hash-based edge lookup**: Replace linear child scan with `HashMap<ConditionKey, NodeId>`
   at nodes with many children. Only worth it if profiling shows edge scan is a hotspot.

2. **Sorted threshold nodes**: For integer conditions on the same key (e.g., many queries
   checking `life > 40`, `life > 70`, `life > 100`), sort thresholds and binary search.
   Prune impossible branches early.

3. **Batch matching**: Process multiple items per DAG walk, using SIMD for condition
   evaluation where possible.

4. **Incremental rebuild**: If query churn is high, periodically rebuild the DAG from
   scratch to reclaim fragmentation and rebalance.

5. **Partitioned DAGs**: Shard by item category — each category gets its own DAG.
   Eliminates the first level of the index and improves cache locality.
