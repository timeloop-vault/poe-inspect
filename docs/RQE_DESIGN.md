# Reverse Query Engine (RQE) — Design Document

Living document. Captures vision, architecture thinking, and prototype plan for a community-scale reverse query engine for Path of Exile.

## Problem Statement

PoE trading is **seller-push only**: players list items, buyers search. Nobody can express demand. There is no way to say "I want this — does anyone have it?"

The RQE inverts this: players register **what they're looking for** (reverse queries). When an item enters the system, it's matched against all registered queries. Matching players are notified.

This is a **demand marketplace** — the missing half of PoE trade.

## Prior Art

An Erlang prototype (`_reference/rqe/`) was built ~2018-2019. Key properties:
- Queries stored in-memory across Erlang processes (~10k per store)
- Items broadcast to all stores via Mnesia table subscriptions (push, not poll)
- Matching is brute-force: every query evaluated against every item
- Distributed via Mnesia replication + gRPC gateway
- Composable predicates: boolean, string (with wildcard), integer (with operators), nested lists with AND/OR/COUNT

The Erlang design validated the concept. What follows is a modern rethink.

## Relationship to poe-inspect

poe-eval is the shared core:

```
poe-eval (predicate model + evaluation logic)
    |                       |
    v                       v
poe-inspect app         poe-rqe (reverse query engine crate)
(local overlay)             |
"does MY item              v
 match MY rules?"      rqe-server (Cloud Run binary)
                       "does THIS item match
                        ANYONE's rules?"
```

The predicate types and matching logic are identical whether evaluating locally or in the cloud. poe-rqe adds indexing, storage, and query management on top.

## Predicate Model

Carried forward from the Erlang RQE, adapted for Rust. A reverse query is a list of conditions:

```
Condition:
  key: String          — stat template text or item property name
  value: Value         — typed value to compare against
  operator: Operator   — eq, gt, lt, gte, lte, wildcard

Value:
  | Boolean(bool)
  | String(String)
  | Integer(i64)
  | Float(f64)
  | List { operator: ListOp, conditions: Vec<Condition> }

ListOp:
  | And        — all must match
  | Or         — any must match
  | Not        — none must match
  | Count(u32) — exactly N must match
```

Keys use **template strings** (e.g., `"explicit +% to Fire Resistance"`) — same template-keyed approach as poe-inspect's lookups. The parser sees text, not stat IDs.

## Architecture Overview

### Components

| Component | Character | Runtime |
|-----------|-----------|---------|
| **Web API** | Stateless. Registration, auth, CRUD for queries | Cloud Run (auto-scaling) |
| **Web UI** | SPA for managing want-lists | Static hosting (Cloud CDN) |
| **Matcher** | Stateful. Holds queries in memory, evaluates items | Cloud Run (min-instances, CPU always allocated) |
| **Persistent Store** | Durable query storage, user data | Cloud SQL (PostgreSQL) or Memorystore (Valkey) |
| **Message Bus** | Sync query changes to matcher instances | Cloud Pub/Sub or NATS |
| **Notification Delivery** | Push matches to clients | WebSocket / SSE / webhook |

### Data Flow

```
Player registers RQ
    -> Web API writes to persistent store
    -> Pub/Sub notifies all matcher instances
    -> Matchers update in-memory index

Item enters system (via poe-inspect client, API, etc.)
    -> Web API publishes item to matcher(s)
    -> Matcher: indexed lookup -> candidate queries -> full evaluation
    -> Matches sent to notification service
    -> Players notified
```

### Cloud Run Specifics

- **`--min-instances`**: Keep matchers warm — queries must be in memory
- **`--cpu-always-allocated`**: Matcher does background work (index maintenance)
- **Startup**: Load queries from persistent store into memory + build index
- **Refresh strategy**: Pub/Sub subscription for incremental updates (add/remove individual queries). Periodic full reload as safety net.
- **Scaling concern**: Each instance holds ALL queries (or a partition). Need to think about sharding strategy if query volume exceeds single-instance memory.

### Future Exploration

- **Fastly / CDN caching**: Could cache "popular" match results or item-category indexes at the edge. Worth exploring if latency matters.
- **In-memory crate**: Rust ecosystem has fast concurrent data structures (dashmap, evmap, etc.) that could serve as the query store. Profile before choosing.

## Indexed Predicate Matching

The Erlang RQE's biggest limitation: brute-force evaluation of every query against every item. This section captures strategies to avoid that.

### Multi-Level Discrimination Network

Instead of evaluating all N queries for every item, build an inverted index from condition keys to query sets:

```
Index Structure (simplified):

Level 1 — Item Category (highest selectivity):
  "Crimson Jewel"     -> {Q12, Q47, Q903, Q1204}
  "Cobalt Jewel"      -> {Q3, Q88, Q455}
  "Titan Greaves"     -> {Q99, Q100, Q501}
  [wildcard / any]    -> {Q7, Q22}           <- always evaluated

Level 2 — Rarity or other common condition:
  "Non-Unique"        -> {Q12, Q47, Q903}
  "Unique"            -> {Q1204}

Level 3 — Stat presence (which queries care about this stat?):
  "+% Fire Res"       -> {Q47, Q1204}
  "+% Cold Res"       -> {Q12, Q903}
```

**Matching flow:**

1. Item arrives: `category="Crimson Jewel", rarity="Non-Unique", fire_res=15`
2. Level 1: category index -> `{Q12, Q47, Q903, Q1204}` (4 candidates, not 50,000)
3. Level 2: intersect with rarity -> `{Q12, Q47, Q903}` (3 candidates)
4. Level 3: stat presence further narrows candidates
5. Full evaluation: brute-force only the remaining candidates against all their conditions

**Even one good index key (category) turns O(N) into O(N/hundreds).**

### Handling Tricky Cases

**Range predicates** (armor > 200): Can't do exact-match lookup. Use sorted structures (BTreeMap) and range scans to find queries whose thresholds are satisfied.

**OR / COUNT conditions** ("any 2 of these 5 stats"): Cannot be fully indexed. Strategy: index the outer required conditions (category, rarity) to reduce candidates, then brute-force the inner combinatorial part on the small candidate set.

**Wildcard `_` values**: Queries with "any category" skip that index level and go into an "always evaluate" bucket. As long as most queries have at least one selective condition, this bucket stays small.

**Queries with only loose conditions**: Some queries might be very broad ("any rare item with life > 50"). These end up in the always-evaluate bucket. Could add a cost estimate and reject queries that are too broad, or rate-limit notifications for them.

### Rete Algorithm

The Rete algorithm (used in production rule systems like Drools, CLIPS, OPS5) is the academic gold standard for this problem. Core ideas:

- **Discrimination network**: Rules are compiled into a directed acyclic graph. Shared conditions across rules become shared nodes — evaluated once, result reused.
- **Alpha network**: Tests individual conditions (single-field checks). Filters facts that match each condition.
- **Beta network**: Joins results from multiple alpha nodes. Handles multi-condition rules.
- **Working memory**: Partial match results are cached. When a new fact arrives, only affected branches are re-evaluated (incremental matching).

**Why it matters for RQE**: If 10,000 queries all require `category = "Crimson Jewel"`, Rete evaluates that condition once and propagates the result to all 10,000 downstream branches. The brute-force approach evaluates it 10,000 times.

**Trade-offs**:
- Rete uses more memory (caches partial matches)
- Network construction has upfront cost (amortized over many matches)
- Complex to implement correctly
- May be overkill if the multi-level index approach gets us to small enough candidate sets

**Recommendation**: Start with the multi-level discrimination network (simpler, big wins). Profile. If hotspots remain, study Rete for specific optimizations. The approaches are complementary — the index IS a simplified two-level Rete.

### References

- Forgy, C. (1982). "Rete: A Fast Algorithm for the Many Pattern/Many Object Pattern Match Problem"
- Doorenbos, R. (1995). "Production Matching for Large Learning Systems" (Rete/UL)
- Modern implementations: Drools (Java), Clara Rules (Clojure), `rete-rs` (Rust, experimental)

## Open Questions

1. **Sharding strategy**: If query volume exceeds single-instance memory, how to partition? By item category? By user? By hash?
2. **Notification delivery**: WebSocket (real-time but complex), SSE (simpler), webhook (decoupled), or in-app polling from poe-inspect?
3. **Rate limiting**: How to prevent abuse (millions of broad queries)?
4. **Authentication**: GGG OAuth? Discord? Custom accounts?
5. **Item ingestion**: How do items enter the system? Only via poe-inspect clients? Public trade API scraping?
6. **Economy integration**: Could poe.ninja price data inform "is this query reasonable?" or "this item is worth X to Y people"?
7. **PoE2 compatibility**: Same predicate model? Different stat templates?

## Prototype Plan

### Phase 0 — Predicate Model (inside poe-eval)
- Define `Condition`, `Value`, `ListOp` types in poe-eval
- Implement `evaluate(conditions: &[Condition], item: &Item) -> bool`
- Test with the Erlang RQE's test data (port the JSON test cases)
- This serves both poe-inspect (local) and RQE (remote)

### Phase 1 — In-Process RQE (poe-rqe crate)
- Query store: add/remove/list reverse queries
- Multi-level index: build index on insert, update on remove
- `match_item(item) -> Vec<QueryId>` using indexed lookup + evaluation
- Benchmark: how many queries can we match per second?

### Phase 2 — Server Binary
- Axum or similar HTTP server wrapping poe-rqe
- REST API: register query, submit item, get matches
- In-memory storage (HashMap/DashMap), persistence to SQLite or PostgreSQL
- Deploy to Cloud Run with min-instances

### Phase 3 — Integration & Notifications
- poe-inspect client: "who wants this item?" button
- WebSocket/SSE for real-time match notifications
- Web UI for managing want-lists without poe-inspect

Each phase is independently useful and testable.
