# poe-rqe — Reverse Query Engine

## Purpose

Match items against registered reverse queries. Instead of "find items matching a query,"
RQE answers "which queries match this item?" — enabling a demand marketplace where players
register what they're looking for and get notified when matching items appear.

## Status

Steps 1-5 complete: Predicate types, evaluation, Erlang test ports, brute-force QueryStore,
and **decision DAG indexed matching** (IndexedStore).

## Architecture

Currently self-contained. When poe-eval matures, the predicate model (`predicate.rs`)
and evaluation logic (`eval.rs`) will be extracted into poe-eval as the shared core
for both local item evaluation (poe-inspect overlay) and remote matching (RQE service).

```
predicate.rs  — Condition, Value, CompareOp, ListOp types + serde
eval.rs       — evaluate() + evaluate_one(), Entry type
store.rs      — QueryStore: brute-force baseline (kept for testing)
index.rs      — IndexedStore: decision DAG with canonical ordering + AND flattening
```

## Design Decisions

- **Predicate model ported from Erlang RQE** (`_reference/rqe/`), adapted for Rust idioms
- **JSON wire format** matches Erlang's: conditions use `key`/`value`/`type`/`typeOptions`
- **Entry** is a flat `HashMap<String, EntryValue>` — same as Erlang's flat map approach
- **Evaluation** is recursive with short-circuit, mirroring `rqe_lib:eval_rq/2` exactly
- **Template-keyed**: keys are stat description text, not numeric IDs (shared principle with poe-inspect)

## Erlang Behavior Reference

Key semantics ported from `rqe_lib.erl`:
- Missing key in entry → condition fails (returns false)
- String wildcard `"_"` → always matches
- Integer with null typeOptions → exact equality only
- Boolean: `true` matches `true`, `false` matches anything that isn't `true`
- List AND: all conditions must match (same as top-level eval)
- List NOT: no conditions must match (if any matches → false)
- List COUNT(n): exactly n conditions must match

## Test Data

Erlang test fixtures at `_reference/rqe/test/data/`:
- `rq/` — 9 reverse query definitions
- `entry/` — 8 item entry definitions
- Key validated case: `wanted_mod_and_not_count` + `crimson_w_mods_2` → match,
  `wanted_mod_and_not_count` + `crimson_w_mods_1` → no match

## Decision DAG Design

See `docs/rqe-decision-dag.md` for the full design document.

Key properties:
- **Alpha network only** — one item vs many queries, no Rete join network needed
- **AND flattening** — `AND([a, b])` decomposed into individual DAG nodes for sharing
- **Canonical ordering** — conditions sorted by selectivity (category → rarity → strings → booleans → integers → compounds)
- **NOT/OR/COUNT stay opaque** — placed last, evaluated on smallest candidate set
- **Arena-allocated** — `Vec<DagNode>` with `NodeId = u32` for cache-friendly traversal

## Benchmark: Brute-force vs Decision DAG

| Queries | Brute-force | Indexed | Speedup |
|---------|-------------|---------|---------|
| 100 | 1.6μs | 0.36μs | 4.5x |
| 1,000 | 18μs | 0.34μs | 54x |
| 10,000 | 325μs | 0.50μs | 650x |
| 50,000 | 13.7ms | 0.6μs | 22,800x |
| 100,000 | 29.8ms | 1.3μs | 22,900x |
| 1,000,000 | 344ms | 24μs | 14,300x |

DAG has 81 nodes regardless of query count (synthetic benchmark with high sharing).
Real data would have more nodes but speedup remains massive.

Run with: `cargo bench -p poe-rqe`

## Plan

See `docs/RQE_DESIGN.md` for full plan. Next: server binary (Step 5 in design doc),
then integration and notifications.
