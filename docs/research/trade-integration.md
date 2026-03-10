# Trade API Integration Research

> Research output for poe-inspect-2. Based on live API testing, awakened-poe-trade source analysis, and existing research in `trade-api-and-logs.md`.

## The Bridge: `/api/trade/data/stats`

GGG provides a public endpoint that returns **all searchable stats** with display text and trade IDs:

```
GET https://www.pathofexile.com/api/trade/data/stats
User-Agent: poe-inspect-2/0.1 (contact: github.com/timeloop-vault/poe-inspect)
```

**Response structure** (15,452 entries across 13 categories):

```json
{
  "result": [
    {
      "label": "Explicit",
      "entries": [
        {
          "id": "explicit.stat_3299347043",
          "text": "+# to maximum Life",
          "type": "explicit"
        }
      ]
    }
  ]
}
```

### Categories (PoE 3.28 Mirage)

| Category | Entries | Notes |
|----------|---------|-------|
| Pseudo | 296 | Aggregated sums (total life, total res, etc.) |
| Explicit | 6,729 | |
| Implicit | 1,428 | |
| Imbued | 161 | Imbued mods (3.28+) |
| Fractured | 1,786 | |
| Enchant | 1,464 | |
| Scourge | 409 | Legacy |
| Crafted | 286 | Bench crafts |
| Crucible | 2,492 | Legacy |
| Veiled | 20 | |
| Delve | 80 | |
| Ultimatum | 63 | Legacy |
| Sanctum | 238 | |

### Stat ID Format

Two formats coexist:

1. **Numeric hash**: `explicit.stat_3299347043` — used for real game stats. The number is a GGG-internal identifier (not CRC32, FNV, or Murmur of the stat name — tested and confirmed). Same number is shared across categories (explicit/implicit/fractured all use `stat_3299347043` for max life).

2. **Named pseudo**: `pseudo.pseudo_total_life` — human-readable IDs for aggregated pseudo stats.

### Option entries

110 stats have dropdown options instead of numeric values:

```json
{
  "id": "pseudo.pseudo_searing_implicit_tier",
  "text": "Searing Exarch Implicit Modifier (#)",
  "type": "pseudo",
  "option": {
    "options": [
      { "id": 1, "text": "Lesser" },
      { "id": 2, "text": "Greater" },
      { "id": 6, "text": "Perfect" }
    ]
  }
}
```

## Our Integration Path

### The template text bridge

Our reverse index already resolves display text → template + stat IDs:
```
"+92 to maximum Life" → template: "+# to maximum Life", stat_ids: ["base_maximum_life"]
```

The trade API maps the same template text → trade stat ID:
```
"+# to maximum Life" → "explicit.stat_3299347043"
```

**Template text is the natural join key.** Both our `stat_descriptions.txt` parsing and the trade API use `#` as the value placeholder with identical display text.

### Mapping strategy

1. **Fetch** `/api/trade/data/stats` once per league launch, cache to disk (~2MB JSON)
2. **Build lookup**: normalize template text → trade stat IDs (by category)
3. **Cross-reference**: for each reverse index template, find matching trade API entries
4. **Result**: bidirectional map `base_maximum_life` ↔ `stat_3299347043`

This is dynamic — no need to ship pre-built data files like awakened-poe-trade.

### Text normalization considerations

Trade API text and our stat_descriptions templates should match closely, but watch for:
- Capitalization differences (trade API: `+# to maximum Life`, stat_desc: `+# to Maximum Life`?)
- Whitespace or punctuation variations
- Multi-stat descriptions that produce multiple lines
- A few stats may have different wording between GGPK and trade API

**Validation step**: after building the mapping, log any trade API stats that didn't match a reverse index template and vice versa. This is our coverage metric.

## Search Query Construction

### From parsed item to trade search

```
Parsed item (poe-item):
  base_type: "Vaal Axe"
  rarity: Rare
  mods:
    - template: "+# to maximum Life", values: [92], type: explicit
    - template: "+#% to Fire Resistance", values: [41], type: explicit

↓ map via trade stats lookup

Trade query:
  POST /api/trade/search/Mirage
  {
    "query": {
      "type": "Vaal Axe",
      "stats": [{
        "type": "and",
        "filters": [
          { "id": "explicit.stat_3299347043", "value": { "min": 80 } },
          { "id": "explicit.stat_3372524247", "value": { "min": 35 } }
        ]
      }]
    },
    "sort": { "price": "asc" }
  }
```

### Value relaxation

For price checking, awakened-poe-trade relaxes values (doesn't search for exact rolls). Typical strategy:
- Search with ~80-90% of actual values as minimums
- No maximum (find better items too)
- Optionally use pseudo stats instead of explicit (e.g., `pseudo.pseudo_total_life` captures all life sources)

### Weight-based search

The trade API supports `"type": "weight"` filters — assign numeric weights to stats and filter on weighted sum. Useful for DPS estimation or custom scoring. Maps naturally to our poe-eval scoring profiles.

## How awakened-poe-trade Does It

**Repo**: `SnosMe/awakened-poe-trade` (cloned in `_reference/awakened-poe-trade`)

### Key files

| File | Purpose |
|------|---------|
| `renderer/public/data/en/stats.ndjson` | Pre-built stat → trade ID mapping |
| `renderer/src/parser/Parser.ts` | Ctrl+C text parser (section-based, like ours) |
| `renderer/src/parser/stat-translations.ts` | Text → stat matching via FNV-1a hash |
| `renderer/src/web/price-check/trade/pathofexile-trade.ts` | Trade query builder |
| `renderer/src/web/price-check/filters/create-stat-filters.ts` | Stat → filter conversion |
| `renderer/src/web/price-check/filters/pseudo/index.ts` | Pseudo stat aggregation |

### Their stats.ndjson format

```json
{
  "ref": "+# to maximum Life",
  "better": 1,
  "matchers": [
    {"string": "+# to maximum Life"},
    {"string": "+1 to maximum Life", "value": 1}
  ],
  "trade": {
    "ids": {
      "explicit": ["explicit.stat_3299347043"],
      "fractured": ["fractured.stat_3299347043"],
      "crafted": ["crafted.stat_3299347043"]
    }
  }
}
```

- `matchers` includes singular forms (value=1) for grammar
- `trade.ids` maps to multiple categories (same stat can be explicit, fractured, or crafted)
- Data files are **pre-built offline** and committed to the repo (generation tooling not included)

### Their matching pipeline

1. Parse item clipboard text → sections → mod lines
2. Replace numeric values with `#` → normalize
3. FNV-1a hash lookup into stats dictionary
4. Get trade stat IDs per category
5. Build search query with relaxed value ranges

## Implementation Plan for poe-inspect-2

### What exists

- Item parsing: poe-item (PEST grammar + resolver, 75 tests)
- Stat resolution: reverse index (15.5k templates, 100% hit rate)
- Evaluation: poe-eval (predicates, scoring profiles)
- App: Tauri overlay with item display

### What to build

**Phase 1: Trade stats cache** (poe-data or new poe-trade crate)
- Fetch `/api/trade/data/stats` with proper User-Agent
- Parse response, build `template_text → Vec<TradeStatEntry>` lookup
- Cache to disk (JSON), refresh on league change
- Build cross-reference: `ggpk_stat_id ↔ trade_stat_number`

**Phase 2: Trade query builder** (poe-eval or poe-trade)
- Take `EvaluatedItem` → construct trade search body
- Value relaxation strategy (configurable percentage)
- Support pseudo stat aggregation
- Handle option-type stats

**Phase 3: Trade client** (app or poe-trade)
- HTTP client with rate limit handling (parse `X-Rate-Limit-*` headers)
- Request queue with backoff
- Search + fetch two-step flow
- Cache recent searches

**Phase 4: UI integration** (app)
- Price check button/hotkey on overlay
- Trade results display (price, listing count)
- "Open on trade site" link (construct URL from search ID)
- Per-stat trade filter toggles

### Effort estimate

The mapping layer (phases 1-2) is straightforward — most of the plumbing exists. The main work is **UX**: designing how price information integrates into the overlay, how users customize which stats to search by, and how to handle rate limits gracefully in the UI.

## API Notes

- **No auth required** for search/fetch
- **User-Agent required**: descriptive string identifying the tool
- **Rate limits**: ~12 req/6s search, ~16 req/60s. Parse `X-Rate-Limit-*` headers dynamically.
- **403 without User-Agent**: the API blocks requests without a proper User-Agent header
- **Regional variants**: `ru.pathofexile.com`, `pathofexile.tw`, `poe.game.daum.net`
- **PoE2**: same structure at `/api/trade2/` (different stat IDs)

## References

- `docs/research/trade-api-and-logs.md` — Full API docs, rate limits, auth, Client.txt
- `_reference/awakened-poe-trade/` — Source code reference
- `SnosMe/poe-dat-viewer` — SnosMe's dat viewer (likely used to generate stats.ndjson)
