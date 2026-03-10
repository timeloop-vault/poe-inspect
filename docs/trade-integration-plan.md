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

---

## Phase 2: Query Builder

**Goal**: `ResolvedItem` → trade API search body. Pure logic, no HTTP.

**Files**: `poe-trade/src/query.rs`

**What it does**:

1. **Base type**: `item.header.base_type` → query `"type"` field
2. **Stat filters**: For each `ResolvedMod`:
   - Get `stat_ids` from each `ResolvedStatLine`
   - Look up trade stat number in `TradeStatsIndex`
   - Determine category prefix from `ModDisplayType` / `ModSlot`:
     - `Prefix`/`Suffix` → `"explicit."`
     - `Implicit` → `"implicit."`
     - `Enchant` → `"enchant."`
     - `Crafted` → `"crafted."`
     - Fractured → `"fractured."` (overrides explicit)
   - Combine: `"explicit.stat_3299347043"`
3. **Value relaxation**: Default `min = floor(value × 0.85)`, no max. Configurable.
4. **Item filters**: Rarity, item class (via `poe-data` domain mapping), ilvl, corrupted, influences
5. **Output**: Serializable `TradeSearchBody` ready for POST

**Item class → trade category**: `poe-data/src/domain.rs` owns this mapping (e.g., `"Body Armours"` → `"armour.body"`). It's PoE domain knowledge.

**Trade URL**: `https://www.pathofexile.com/trade/search/{league}/{search_id}`

**Done when**: Can construct a valid trade query JSON from a parsed item fixture.

---

## Phase 3: Rate-Limited HTTP Client

**Goal**: Well-behaved async HTTP client that respects GGG rate limits.

**Files**: `poe-trade/src/client.rs`, `poe-trade/src/rate_limit.rs`

**TradeClient**:
- Wraps `reqwest::Client` with `User-Agent: poe-inspect-2/0.1`
- `async fn search(query) → SearchResult` (search ID + listing IDs + total count)
- `async fn fetch(listing_ids, search_id) → Vec<Listing>` (max 10 per request)
- `async fn fetch_stats() → TradeStatsResponse` (raw API response)

**Rate limit state machine** (`rate_limit.rs`):
- Parse `X-Rate-Limit-Ip`: `hits:period:timeout` format
- Track state from `X-Rate-Limit-Ip-State`: `current:period:penalty`
- Methods: `can_request() → bool`, `delay_until_available() → Duration`
- On 429: parse `Retry-After`, wait, retry once

**POESESSID**: Optional cookie for "online only" filtering. Stored securely by app, passed to client as config. Never logged.

**Done when**: Can execute a full search+fetch cycle against the live API with rate limiting.

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

## Risks

| Risk | Mitigation |
|------|-----------|
| Template text mismatch (GGPK vs trade API) | Case-insensitive compare, log mismatches, manual override table in `poe-data/domain.rs` |
| Rate limit exhaustion | Explicit user action (no auto-check), cache results, cooldown UI |
| GGG changes API without notice | Community tools break too — monitor, adapt |
| Category prefix mapping errors | Test against known items, validate `ModDisplayType` → prefix mapping |
