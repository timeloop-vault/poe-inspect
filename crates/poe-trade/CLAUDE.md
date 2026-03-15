# poe-trade

Trade API client for pathofexile.com — fetches trade stats, builds search queries, and executes rate-limited price lookups.

## Status

**Phase 3 + schema done** — Rate-limited HTTP client, filter schema from GGG's filters.json, inline trade edit controls on overlay. 58 tests. `EditFilterKind::Sockets` for socket color filters, `max_override` on `StatFilterOverride`, `trade_edit_schema()` per-inspect projection.

## Scope

- Fetch and cache GGG's trade stats dictionary (`/api/trade/data/stats`)
- Build bidirectional mapping between GGPK stat IDs and trade API stat IDs
- Construct trade search query bodies from `ResolvedItem`
- Rate-limited HTTP client for search + fetch two-step flow
- Trade URL construction ("open on trade site")
- Value relaxation strategies for price checking
- Pseudo stat aggregation for trade searches
- Bulk exchange queries (currency/fragments)

## Does NOT own

- Item parsing — that's `poe-item`
- Game data / PoE domain knowledge — that's `poe-data`
- Item evaluation / scoring — that's `poe-eval`
- UI / overlay / user preferences — that's `app`
- Clipboard access or hotkey handling — that's `app`
- Item class → trade category mapping — that's `poe-data/domain.rs` (PoE domain knowledge)

## Key Design Decisions

- **Separate crate, not in poe-data**: Trade API data is dynamic (fetched per league, external HTTP). `poe-data` owns static GGPK-extracted game data. Different lifecycles, different concerns.
- **Not in poe-eval**: `poe-eval` is pure evaluation logic (no network). `poe-trade` does HTTP.
- **Sibling to poe-eval**: Both consume `ResolvedItem`. The app coordinates both. No dependency between them.
- **Template text is the join key**: Our reverse index templates (`+# to maximum Life`) match the trade API's stat text. No hash reverse-engineering needed.
- **`reqwest` with `rustls-tls`**: Satisfies workspace `unsafe_code = "forbid"` for our code. Cross-platform.
- **Async API**: Tauri 2 runs Tokio. All HTTP methods are async.

## Architecture

```
src/
  lib.rs           — public API
  types.rs         — TradeStatEntry, TradeStatsIndex, SearchResult, Price, StatFilterOverride, etc.
  stats_index.rs   — fetch /data/stats, build template→trade_id lookup, cross-ref with ReverseIndex
  query.rs         — ResolvedItem → trade search body (value relaxation, stat filters, max_override)
  filter_schema.rs — FilterIndex from filters.json, trade_edit_schema() per-inspect projection
  client.rs        — rate-limited HTTP client (search + fetch)
  rate_limit.rs    — parse X-Rate-Limit-* headers, request throttling
```

## Dependency Position

```
        poe-data (game tables, domain knowledge)
        /     \
   poe-item    |
    /    \     |
poe-eval  poe-trade
    \       /
      app
```

- Depends on `poe-item` (for `ResolvedItem`, `ResolvedMod`, `ModDisplayType`, etc.)
- Depends on `poe-data` (for `ReverseIndex` cross-referencing, item class → trade category)
- Does NOT depend on `poe-eval`
- App depends on both `poe-eval` and `poe-trade`

## Dependencies

- `poe-item` — `ResolvedItem` types (with `serde` feature)
- `poe-data` — `ReverseIndex`, `GameData` for cross-referencing
- `reqwest` — HTTP client (with `rustls-tls`)
- `tokio` — async runtime
- `serde` / `serde_json` — API response parsing, query serialization
- `thiserror` — error types
- `tracing` — logging
- `ts-rs` — TypeScript type generation (feature-gated `ts`)

## Plan

See `docs/trade-integration-plan.md` for the full phased plan.

1. ~~Trade stats index — fetch, parse, template matching, disk cache~~ ✅
2. ~~Query builder — `ResolvedItem` → trade search body~~ ✅
3. ~~Rate-limited HTTP client — search + fetch with header-based throttling~~ ✅
4. ~~Filter schema — `FilterIndex` from GGG's filters.json, `trade_edit_schema()` projection~~ ✅
5. ~~Inline overlay editing — socket filters, max_override, type scope~~ ✅
6. App integration — Tauri commands for price check flow, overlay price results
7. Advanced — pseudo trade IDs, weight-based search, bulk exchange
