# poe-trade: Interaction Model Redesign

## Problem

The current poe-trade interaction has two structural issues:

### 1. TradeFilterConfig grows with every filter type

Every new trade filter requires adding fields to `TradeFilterConfig` in Rust, regenerating TypeScript types, and wiring them through `buildFilterConfig()` in the frontend. Adding gem support required **6 new fields** across 4 files:

```
crates/poe-trade/src/types.rs       — 6 new fields on TradeFilterConfig
app/src/generated/TradeFilterConfig.ts — regenerated (6 new fields)
app/src/hooks/useTradeFilters.ts     — 50 lines of new buildFilterConfig wiring
crates/poe-trade/src/query.rs        — 30 lines of new build_misc_filters logic
```

This pattern repeats for every filter category: map filters, heist filters, requirement filters, etc. The current TradeFilterConfig has **17 filter-specific fields** and would need ~30+ more for full coverage.

### 2. The app re-sends item text for every operation

Every Tauri trade command (`price_check`, `trade_search_url`, `preview_trade_query`, `get_trade_edit_schema`) receives `item_text: String` and re-parses + re-resolves it. The resolved item is never cached or reused across commands.

### 3. Query building mixes item data with user selections

`build_query()` takes both `ResolvedItem` and `TradeFilterConfig` and internally decides what to include. The fallback logic is implicit:

```rust
// Example: gem level
let min = fc.gem_level_min           // user override?
    .or_else(|| extract_gem_level(item))  // fall back to item data
    .map(f64::from);
```

Every filter has this pattern: check user override, fall back to item extraction. The item extraction logic is duplicated between `build_query()` (for searching) and `filter_default()` (for schema defaults).

## Current Data Flow

```
Frontend clicks "Price Check"
    │
    ├── sends: item_text + TradeQueryConfig + TradeFilterConfig
    │
    ▼
Tauri command: price_check()
    ├── poe_item::parse(item_text)        ← re-parses every time
    ├── poe_item::resolve(raw, game_data) ← re-resolves every time
    ├── build_query(resolved, index, config, filter_config)
    │       ├── build_stat_filters()       ← reads item mods + user overrides
    │       ├── build_misc_filters()       ← reads item props + user overrides (17 fields)
    │       ├── build_type_filters()       ← reads item header + user overrides
    │       ├── build_socket_filters()     ← reads item sockets + user overrides
    │       └── build_weapon_filters()     ← reads item properties
    ├── client.price_check(body, config)   ← HTTP
    └── returns PriceCheckResult
```

## Proposed Data Flow

```
Frontend receives item text (hotkey)
    │
    ▼
Step 1: Resolve (once)
    ├── APP → poe_item::resolve() → ResolvedItem
    └── APP caches ResolvedItem (already done via state)

Step 2: Build schema (once per item)
    ├── APP → poe_trade::build_trade_schema(ResolvedItem) → TradeSchema
    │       TradeSchema {
    │           name: Option<String>,        // query.name (gem/unique name)
    │           base_type: Option<String>,   // query.type
    │           category: Option<String>,     // type_filters.category
    │           filters: Vec<TradeFilter>,    // ALL available filters with:
    │               - id: "gem_level"
    │               - kind: Range { min, max }
    │               - default_value: Some(21.0)
    │               - auto_enabled: false
    │               - group: "misc_filters"
    │           stats: Vec<TradeStat>,       // ALL mappable stats with:
    │               - trade_id: "explicit.stat_3299347043"
    │               - display_text: "+68 to maximum Life"
    │               - default_min: 57.0
    │               - auto_enabled: true
    │       }
    └── APP renders schema as UI (filters + stats)

Step 3: Search (user-driven)
    ├── APP collects UI state into generic values:
    │       TradeSearchRequest {
    │           filters: HashMap<String, FilterValue>,
    │               // "gem_level" → Range { min: 21 }
    │               // "corrupted" → Option { value: "true" }
    │               // "quality" → Range { min: 20 }
    │           stats: Vec<StatSelection>,
    │               // { trade_id: "explicit.stat_...", min: 57.0 }
    │           name: Option<String>,
    │           base_type: Option<String>,
    │           category: Option<String>,
    │       }
    │
    ├── APP → poe_trade::search(TradeSearchRequest) → results
    │       (builds POST body purely from UI selections, no ResolvedItem needed)
    └── returns prices
```

## Key Differences

| Aspect | Current | Proposed |
|--------|---------|----------|
| Adding a filter | 4 files, ~100 lines | Schema builder only (~5 lines) |
| TradeFilterConfig | 17+ typed fields, growing | `HashMap<String, FilterValue>` — generic |
| Frontend wiring | `buildFilterConfig()` with per-filter logic | Generic: read schema, send values |
| Item re-parsing | Every command re-parses text | Resolve once, schema once |
| Query building | Mixes item data + overrides | Schema provides defaults, search uses only UI values |
| Filter defaults | Duplicated in `filter_default()` + `build_*_filters()` | Single source: `build_trade_schema()` |

## What Changes

### poe-trade

**Remove:**
- `TradeFilterConfig` struct (17 fields)
- `build_misc_filters()` fallback logic (item extraction + override merge)
- `build_type_filters()` / `build_socket_filters()` override logic

**Keep:**
- `TradeStatsIndex` — stat ID mapping (unchanged)
- `FilterIndex` — GGG filter schema loading (unchanged)
- Rate-limited HTTP client (unchanged)
- `trade_edit_schema()` — **evolves into** `build_trade_schema()`

**Add:**
- `build_query_from_selections(request: &TradeSearchRequest) -> TradeSearchBody`
  Pure translation from generic filter values to trade API JSON. No item knowledge needed.

### app (Tauri commands)

**Remove:**
- `filter_config: Option<TradeFilterConfig>` parameter from all commands
- Re-parsing item text in every command

**Simplify:**
- `price_check(selections: TradeSearchRequest)` — just build body + HTTP
- `trade_search_url(selections: TradeSearchRequest)` — just build body + search
- `get_trade_schema(item_text: String)` — resolve + build schema (replaces both `preview_trade_query` and `get_trade_edit_schema`)

### app (frontend)

**Remove:**
- `buildFilterConfig()` — 90 lines of per-filter wiring in useTradeFilters.ts
- Per-filter fields in TradeFilterConfig type

**Simplify:**
- Schema drives UI generically: each `TradeFilter` renders as checkbox + input (range) or dropdown (option)
- On search: collect all enabled filters into `HashMap<filter_id, value>` and send

## Migration Path

This can be done incrementally:

1. **Add `build_trade_schema()`** that returns the unified schema (merge current `trade_edit_schema` + `build_query`'s default logic)
2. **Add `build_query_from_selections()`** that builds the POST body from generic values
3. **Add new Tauri commands** alongside existing ones (`trade_schema`, `trade_search`)
4. **Update frontend** to use new commands + generic filter state
5. **Remove old commands** and `TradeFilterConfig`

Each step is independently testable. The old and new paths can coexist during migration.

## Risks

- **Schema must capture everything `build_query` knows** — stat auto-selection logic, pseudo preference, tier thresholds, value relaxation. These currently live in `build_query()` and would need to move into the schema or be passed as search config.
- **Auto-search defaults** need a clear home. Currently `build_query(None)` (no filter config) auto-selects stats and filters. In the new model, the schema's `auto_enabled` flags serve this purpose, but the app needs to respect them when building `TradeSearchRequest`.
- **Unique disambiguation** currently flows through `TradeFilterConfig.unique_name_override`. In the new model, this becomes a regular filter value (`"unique_name" → "Headhunter"`).

## Non-Goals

- Changing how the trade API is called (POST body format, rate limiting, search+fetch flow)
- Changing how stat IDs are mapped (TradeStatsIndex stays the same)
- Changing poe-item (parsing/resolving is untouched)
- Changing poe-eval (evaluation is a separate concern)
