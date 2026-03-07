# poe-rqe — Reverse Query Engine

## Purpose

Match items against registered reverse queries. Instead of "find items matching a query,"
RQE answers "which queries match this item?" — enabling a demand marketplace where players
register what they're looking for and get notified when matching items appear.

## Status

Steps 1-4 (brute-force): Predicate types, evaluation, Erlang test ports, and brute-force QueryStore with benchmark.

## Architecture

Currently self-contained. When poe-eval matures, the predicate model (`predicate.rs`)
and evaluation logic (`eval.rs`) will be extracted into poe-eval as the shared core
for both local item evaluation (poe-inspect overlay) and remote matching (RQE service).

```
predicate.rs  — Condition, Value, CompareOp, ListOp types + serde
eval.rs       — evaluate() function, Entry type
store.rs      — QueryStore: add/remove/match_item (brute-force baseline)
index.rs      — (future) Multi-level discrimination index (optimization)
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

## Benchmark Baseline (brute-force)

Measured on release builds. This is the baseline before indexing optimization.

| Queries | Evals/sec |
|---------|-----------|
| 100 | ~66M |
| 1,000 | ~55M |
| 10,000 | ~30M |
| 50,000 | ~6M |
| 100,000 | ~5M |

Drop at 50k+ is CPU cache pressure. Indexing would keep candidate sets small enough
to stay in the fast range regardless of total query count.

Run with: `cargo bench -p poe-rqe`

## Plan

See `docs/RQE_DESIGN.md` for full plan. Next: indexed matching (optimization),
then integration and notifications.
