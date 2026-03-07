# poe-rqe — Reverse Query Engine

## Purpose

Match items against registered reverse queries. Instead of "find items matching a query,"
RQE answers "which queries match this item?" — enabling a demand marketplace where players
register what they're looking for and get notified when matching items appear.

## Status

Step 1-3: Predicate types, evaluation function, and Erlang test case ports.

## Architecture

Currently self-contained. When poe-eval matures, the predicate model (`predicate.rs`)
and evaluation logic (`eval.rs`) will be extracted into poe-eval as the shared core
for both local item evaluation (poe-inspect overlay) and remote matching (RQE service).

```
predicate.rs  — Condition, Value, CompareOp, ListOp types + serde
eval.rs       — evaluate() function, Matchable trait, Entry type
store.rs      — (future) QueryStore with add/remove/match
index.rs      — (future) Multi-level discrimination index
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

## Plan

See `docs/RQE_DESIGN.md` for full plan. Current focus: Steps 1-3 (types, eval, tests).
Steps 4+ (indexing, server, integration) come after the core is proven.
