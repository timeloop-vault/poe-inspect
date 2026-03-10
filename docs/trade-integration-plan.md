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

## Phase 1: Trade Stats Index

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

## Phase 2: Query Builder

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

## Phase 3: Rate-Limited HTTP Client

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

## Phase 4: App Integration

**Goal**: Price check UX in the Tauri overlay.

**Backend** (`app/src-tauri`):
- Managed state: `TradeClient`, `TradeStatsIndex`
- Load stats index on startup (disk cache, background refresh)
- Tauri commands:
  - `price_check(item_text) → PriceCheckResult`
  - `open_trade_search(item_text)` → opens browser
  - `refresh_trade_stats()` → force refresh

**Frontend** (`app/src`):
- Price check button on overlay (or hotkey, e.g., Ctrl+P)
- Results panel: price range (cheapest N listings), total count, "Open on trade" link
- Loading/error states (rate limit cooldown display)
- Per-stat toggle: checkboxes to include/exclude stats from search

**Done when**: User can inspect an item and get a price estimate in the overlay.

---

## Phase 5: Advanced Features

- **Pseudo stats**: Aggregate explicit stats into pseudo equivalents (`pseudo.pseudo_total_life`). User toggle.
- **Weight-based search**: Map poe-eval scoring profiles to trade API weight filters.
- **Bulk exchange**: `/api/trade/exchange/` for currency/fragments. Detect by item class.
- **Search history**: Cache recent price checks by item fingerprint.
- **Comparable listings**: Show what similar items sold for, not just current listings.

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
