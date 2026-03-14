# poe-rqe — Reverse Query Engine

## Purpose

Generic, domain-free reverse query engine. Instead of "find items matching a query,"
RQE answers "which queries match this entry?" — applicable to any flat key-value
matching scenario (product marketplaces, item demand, alert systems, etc.).

## Status

Core engine complete: predicate types, evaluation, brute-force QueryStore,
**decision DAG indexed matching** (IndexedStore), and **SelectivityConfig**
for domain-specific condition ordering.

All unit tests use a domain-neutral product marketplace domain. PoE-specific
integration tests live in `rqe-server/tests/poe_fixtures.rs`.

## Architecture

Fully self-contained and domain-free. No PoE-specific logic in the crate.

```
predicate.rs  — Condition, Value, CompareOp, ListOp types + serde
eval.rs       — evaluate() + evaluate_one(), Entry type
store.rs      — QueryStore: brute-force baseline (kept for testing/equivalence)
index.rs      — IndexedStore: decision DAG with SelectivityConfig, canonical ordering + AND flattening
```

## Design Decisions

- **Domain-free**: All PoE-specific tests moved to `rqe-server/tests/poe_fixtures.rs`. Core crate uses product marketplace tests.
- **SelectivityConfig**: User-supplied key ranking for condition evaluation order. Supports exact and prefix matching. Defaults to type-based ordering when no config provided.
- **Predicate model ported from Erlang RQE** (`_reference/rqe/`), adapted for Rust idioms
- **JSON wire format** matches Erlang's: conditions use `key`/`value`/`type`/`typeOptions`
- **Entry** is a flat `HashMap<String, EntryValue>` — same as Erlang's flat map approach
- **Evaluation** is recursive with short-circuit, mirroring `rqe_lib:eval_rq/2` exactly

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

Unit tests use inline product marketplace data (Electronics, Clothing, Books).

Erlang test fixtures for PoE integration tests at `_reference/rqe/test/data/`:
- `rq/` — 9 reverse query definitions
- `entry/` — 8 item entry definitions
- Tested in `rqe-server/tests/poe_fixtures.rs`

## Decision DAG Design

See `docs/rqe-decision-dag.md` for the full design document.

Key properties:
- **Alpha network only** — one entry vs many queries, no Rete join network needed
- **AND flattening** — `AND([a, b])` decomposed into individual DAG nodes for sharing
- **Canonical ordering** — conditions sorted by SelectivityConfig (user-defined) then type-based defaults (strings → booleans → integers → compounds)
- **NOT/OR/COUNT stay opaque** — placed last, evaluated on smallest candidate set
- **Arena-allocated** — `Vec<DagNode>` with `NodeId = u32` for cache-friendly traversal

## Benchmark: Brute-force vs Decision DAG

| Queries | Brute-force | Indexed | Speedup |
|---------|-------------|---------|---------|
| 100 | 1.6us | 0.36us | 4.5x |
| 1,000 | 18us | 0.34us | 54x |
| 10,000 | 325us | 0.50us | 650x |
| 50,000 | 13.7ms | 0.6us | 22,800x |
| 100,000 | 29.8ms | 1.3us | 22,900x |
| 1,000,000 | 344ms | 24us | 14,300x |

DAG has 81 nodes regardless of query count (synthetic benchmark with high sharing).
Real data would have more nodes but speedup remains massive.

Run with: `cargo bench -p poe-rqe`

## Plan

See `docs/RQE_DESIGN.md` for full plan. Next: server binary (Step 5 in design doc),
then integration and notifications.
