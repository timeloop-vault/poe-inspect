# Trade Integration Plan

> Implementation plan for pathofexile.com/trade search integration.
> Research: `docs/research/trade-integration.md`, `docs/research/trade-api-and-logs.md`.
> Reference: `_reference/awakened-poe-trade/` (SnosMe/awakened-poe-trade, MIT).

## Architecture

New `poe-trade` crate. Sibling to `poe-eval` — both consume `ResolvedItem`, the app coordinates.

```
        poe-data (game tables, domain.rs)
        /     \
   poe-item    |
    /    \     |
poe-eval  poe-trade
    \       /
      app
```

### Data flow

```
APP
 ├─ poe-item::resolve(text, &game_data)         → ResolvedItem
 ├─ poe-eval::evaluate_item(&item, &gd, ...)    → EvaluatedItem (scoring)
 └─ poe-trade::price_check(&item, &index, &cfg) → PriceResult   (trade)
```

The app passes user preferences (stats to include, value relaxation %, league, POESESSID) to poe-trade as config. poe-trade has zero UI knowledge.

---

## Phase 1: Trade Stats Index ✅

**Goal**: Fetch `/api/trade/data/stats`, build template text → trade stat ID lookup, cross-reference with GGPK stat IDs, cache to disk.

**Files**: `poe-trade/src/stats_index.rs`, `poe-trade/src/types.rs`

**Key types**:
```rust
struct TradeStatEntry {
    id: String,              // "explicit.stat_3299347043"
    text: String,            // "+# to maximum Life"
    stat_type: String,       // "explicit"
}

struct TradeStatsIndex {
    by_template: HashMap<String, Vec<TradeStatEntry>>,  // normalized text → entries
    by_trade_id: HashMap<String, TradeStatEntry>,       // full trade ID → entry
    ggpk_to_trade: HashMap<String, u64>,                // "base_maximum_life" → 3299347043
    trade_to_ggpk: HashMap<u64, Vec<String>>,           // 3299347043 → ["base_maximum_life"]
}
```

**The join**: Template text (`+# to maximum Life`) matches between our `ReverseIndex` and the trade API. Case-insensitive normalized comparison.

**Cross-reference**: For each trade entry, look up the template in `ReverseIndex::stat_ids_for_template()` to get GGPK stat IDs. Build bidirectional `ggpk_stat_id ↔ trade_stat_number` map.

**Disk cache**: Save raw API response to `{data_dir}/trade_stats_cache.json`. Reload on startup. Refresh when league changes or user requests.

**Validation**: Log unmatched stats in both directions. Expect 90%+ match rate.

**Done when**: Can load the index, look up a GGPK stat ID, and get back the correct trade stat ID with category prefix.

**Current status (Phase 1 done)**: 87.4% match rate (10,160/11,624 stat entries), 7,037 GGPK stat IDs mapped. Remaining 1,464 unmatched are from stat description files we haven't parsed yet — see "Stat description coverage gap" below.

---

## Phase 2: Query Builder ✅

**Goal**: `ResolvedItem` → trade API search body. Pure logic, no HTTP.

**Files**: `poe-trade/src/query.rs`

**What it does**:

1. **Base type**: `item.header.base_type` → query `"type"` field
2. **Stat filters**: For each `ResolvedMod`:
   - Get `stat_ids` from each `ResolvedStatLine`
   - Look up trade stat number in `TradeStatsIndex`
   - Determine category prefix from `ModDisplayType` + `is_fractured`:
     - `Prefix`/`Suffix`/`Unique` → `"explicit."` (unless fractured)
     - `Implicit` → `"implicit."`
     - `Enchant` → `"enchant."`
     - `Crafted` → `"crafted."`
     - `is_fractured` → `"fractured."` (overrides explicit)
   - Combine: `"explicit.stat_3299347043"`
3. **Value relaxation**: `min = floor(value × factor)`, no max. Handles negative values correctly (relaxation widens in the appropriate direction). Multi-value stats (e.g., "Adds # to #") use the average.
4. **Item filters**: Rarity (`nonunique` for rare/magic/normal), corrupted, fractured, unidentified
5. **Output**: Serializable `TradeSearchBody` ready for POST
6. **Diagnostics**: `QueryBuildResult` reports mapped/total/unmapped stats

**Item class → trade category**: TODO for Phase 5 — `poe-data/src/domain.rs` will own this mapping (e.g., `"Body Armours"` → `"armour.body"`). Currently searches by base type alone.

**Trade URL**: `https://www.pathofexile.com/trade/search/{league}/{search_id}` — `trade_url()` helper.

**ts-rs**: All APP-facing types export to TypeScript (`TradeQueryConfig`, `Price`, `PriceCheckResult`, `QueryBuildResult`, `TradeSearchBody`, etc.)

**Current status (Phase 2 done)**: 26 tests. Tested against 4 real item fixtures (rare body armour, rare weapon, fractured axe, corrupted amulet). Produces valid trade API JSON. ~85% stat mapping rate per item (remainder are stats not yet in our reverse index).

---

## Phase 3: Rate-Limited HTTP Client ✅

**Goal**: Well-behaved async HTTP client that respects GGG rate limits.

**Files**: `poe-trade/src/client.rs`, `poe-trade/src/rate_limit.rs`

**TradeClient**:
- Wraps `reqwest::Client` with `User-Agent: poe-inspect-2/0.1`, 30s timeout
- `async fn search(query, league) → SearchResult` (search ID + listing IDs + total count)
- `async fn fetch_listings(search_id, listing_ids) → Vec<FetchResultEntry>` (max 10 per request)
- `async fn fetch_stats() → TradeStatsResponse` (raw API response)
- `async fn price_check(query, config) → PriceCheckResult` (search + fetch + extract prices)
- `fn set_session_id(poesessid)` — optional POESESSID cookie for authenticated requests

**Rate limit tracker** (`rate_limit.rs`):
- Preemptive blocking: wait before sending, don't react to 429
- Parse `X-Rate-Limit-Ip`: `hits:period:timeout` format (e.g., `12:6:60,16:12:300`)
- Track request timestamps per endpoint (separate search/fetch limiters)
- `delay_needed() → Duration`, `wait_for_capacity()` async
- On 429: parse `Retry-After`, block limiter, return `RateLimited` error to caller

**POESESSID**: Optional cookie for "online only" filtering. Set via `set_session_id()`. Never logged.

**Current status (Phase 3 done)**: 33 tests (17 unit + 8 query builder + 8 stats index). Client, rate limiter, and API response types complete. Live API testing deferred to Phase 4 app integration.

---

## Phase 4: Backend Wiring ✅

**Managed state**: `TradeState` — `Mutex<TradeClient>` + `RwLock<Option<TradeStatsIndex>>` + `RwLock<Option<FilterIndex>>`

**10 Tauri commands** (all async, all implemented):
- `price_check(item_text, config, filter_config)` — parse → build query → search → fetch
- `trade_search_url(item_text, config, filter_config)` — parse → build query → search → return trade URL
- `preview_trade_query(item_text, config, filter_config)` — build query without HTTP (for edit UI)
- `get_trade_edit_schema(item_text, config)` — schema-driven filter projection
- `refresh_trade_stats()` — fetch live API stats + filters, build indexes, cache to disk
- `open_url(url)` — open trade site in default browser
- `set_trade_session(poesessid)` — set/clear POESESSID cookie
- `get_trade_index_status()` — index health check (loaded, stat count, mapped count)
- `get_listing_statuses()` — valid listing status options
- `fetch_leagues()` — live league list from GGG

**Index lifecycle**: Loaded from disk cache on startup. User triggers `refresh_trade_stats` to fetch/update. Both `trade_stats.json` and `trade_filters.json` cached to `{app_data_dir}/`.

---

## Phase 5: Trade UI/UX ✅

**TradePanel component** (`app/src/components/TradePanel.tsx`):
- Price Check, Open Trade, Edit Search buttons
- Price results panel: price list, total count
- Loading/error/rate-limit states with retry

**Inline overlay editing** (schema-driven from GGG's filters.json):
- Header: type scope dropdown, rarity cycling badge
- Properties: checkbox + editable value inline
- Sockets: R/G/B/W color inputs + min/max
- Status: checkbox toggle inline
- Mods: checkbox + min/max value inputs per stat line
- Pseudo stats: collapsible section, auto-expands in edit mode

**`useTradeFilters` hook**: Builds `TradeFilterConfig` from user edits, passed to Tauri commands.

**TradeSettings page**: League selector, refresh button, index status display.

---

## Phase 6: Bulk Exchange & Advanced

### Bulk exchange (`/api/trade/exchange/`)

Standard search doesn't cover currency, fragments, div cards, etc. — those use the
bulk exchange endpoint with `{have, want}` trade tags instead of stat filters.

**Steps**:
1. Add item class → trade tag mapping in `poe-data/domain.rs` (PoE domain knowledge)
2. Add endpoint routing in `poe-trade/query.rs`: detect bulk-tradeable items, build exchange query
3. Add `exchange()` method to `TradeClient` — POST to `/api/trade/exchange/{league}`
4. Parse exchange-style responses (ratios + stock, not fixed prices)
5. UI: show exchange ratios differently ("1 = 180c" vs "listed at 50c")
6. In-game exchange warning for currency items (no API exists for in-game exchange)

**Reference**: APT's routing in `renderer/src/web/price-check/trade/common.ts` —
`apiToSatisfySearch()` checks if stats are enabled, falls back to bulk if `tradeTag` exists.

### Other features

- **Pseudo stats**: Computation done (poe-data definitions, poe-item resolver, overlay display). Trade query mapping (`pseudo.pseudo_*` IDs) still TODO.
- **Weight-based search**: Map poe-eval scoring profiles to trade API weight filters.
- **Search history**: Cache recent price checks by item fingerprint.
- **Comparable listings**: Show what similar items sold for, not just current listings.
- **poe.ninja integration**: Currency normalization (exchange rates), price history/trends,
  sanity-check against aggregate pricing. API reversed in `poe-agents` repo. Rate limit: 12 req / 5 min.

---

## Stat Description Coverage Gap

Current match rate is 87.4%. The unmatched 1,464 stats come from **separate stat description files** that our `ReverseIndex` doesn't parse yet. We only parse `stat_descriptions.txt` (30MB, the main file).

| Missing file | Size | Covers | Unmatched stats |
|-------------|------|--------|----------------|
| `atlas_stat_descriptions.txt` | 2.8MB | Atlas passive mods | ~636 |
| `map_stat_descriptions.txt` | 468KB | Map mods ("Your Maps have...") | (included in atlas) |
| `graft_stat_descriptions.txt` | 363KB | Sanctum graft mods | ~87 |
| `sanctum_relic_stat_descriptions.txt` | 128KB | Sanctum relic mods | (included in graft) |
| `heist_equipment_stat_descriptions.txt` | 329KB | Heist equipment | ~8 |
| `expedition_relic_stat_descriptions.txt` | 446KB | Expedition relics | (included in other) |
| Other domain-specific files | varies | Wombgifts, sentinels, etc. | ~733 |

**Fix**: Parse these additional files in `poe-dat` and merge into the `ReverseIndex`. The trade stats index will automatically improve — no changes needed in poe-trade. These files use `include "stat_descriptions.txt"`, so they extend the main file. Our PEST parser already handles the format; we just need to parse more files.

**Priority**: Parse `atlas_stat_descriptions.txt` and `map_stat_descriptions.txt` first — maps are one of the most commonly traded item types.

---

## Risks

| Risk | Mitigation |
|------|-----------|
| Template text mismatch (GGPK vs trade API) | Case-insensitive compare, `+#`→`#` fallback, `(Local)` stripping, log mismatches |
| Stat description coverage | Parse additional stat description files in poe-dat (see gap analysis above) |
| Rate limit exhaustion | Explicit user action (no auto-check), cache results, cooldown UI |
| GGG changes API without notice | Community tools break too — monitor, adapt |
| Category prefix mapping errors | Test against known items, validate `ModDisplayType` → prefix mapping |
