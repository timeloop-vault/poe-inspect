# Trade Edit Search — Schema-Driven Redesign

## Problem

The app frontend currently hardcodes knowledge about what poe-trade can filter:
which fields are toggleable, what types they are, what defaults to use. Every new
filter requires changes to both poe-trade AND the frontend. The overlay and the
trade filter panel are separate, when they should be one thing.

## Design Principle

**The item overlay IS the search builder.** poe-trade tells the frontend exactly
what's editable and how. The frontend is a generic renderer with zero trade domain
knowledge.

## Data Sources (from GGG)

GGG exposes four trade data endpoints. All should be fetched and cached by poe-trade:

| Endpoint | Size | What it contains | Our use |
|----------|------|-----------------|---------|
| `/data/stats` | ~4MB | 11,624 stat filter entries in 13 categories | Stat-to-trade-ID mapping (already done) |
| `/data/filters` | 17KB | 89 structural filters in 12 groups with types/labels/options | **Filter schema** — drives Edit Search UI |
| `/data/items` | 322KB | 5,685 base types + 1,516 uniques with categories | Type search, unique name lookup, category mapping |
| `/data/static` | 190KB | Currency/fragment trade tags with images | Bulk exchange, item images |

### filters.json structure

```json
{
  "result": [
    {
      "id": "misc_filters",
      "title": "Miscellaneous",
      "hidden": true,
      "filters": [
        { "id": "ilvl", "text": "Item Level", "minMax": true },
        { "id": "corrupted", "text": "Corrupted", "option": {
          "options": [
            { "id": null, "text": "Any" },
            { "id": "true", "text": "Yes" },
            { "id": "false", "text": "No" }
          ]
        }},
        ...
      ]
    },
    ...
  ]
}
```

Filter types: `minMax: true` → range input, `option.options` → dropdown.
12 groups, 89 total filters. GGG maintains this — new league filters appear automatically.

## Architecture

```
GGG API
  ├─ stats.json    ─┐
  ├─ filters.json  ─┤  fetched & cached by poe-trade
  ├─ items.json    ─┤
  └─ static.json   ─┘
                     ↓
poe-trade::TradeDataCache
  - stats index (existing)
  - filter schema (new — parsed from filters.json)
  - items index (new)
  - static index (future — for bulk exchange)
                     ↓
poe-trade::trade_edit_schema(item, cache) → TradeEditSchema
  - projects the item onto the filter schema
  - returns which filters apply, with defaults from the item
  - returns per-stat mapping (trade_id, category, computed_min)
                     ↓
App frontend renders TradeEditSchema generically
  - range → number inputs
  - option → dropdown
  - stat → checkbox + min input on mod line
  - no trade domain knowledge
                     ↓
User edits → TradeEditValues (key-value map)
                     ↓
poe-trade::build_query_from_edit(item, values, cache) → TradeSearchBody
```

## Key Types

### TradeEditSchema (returned to frontend)

```rust
pub struct TradeEditSchema {
    /// Type scope options (base type / item class / any)
    pub type_scope: TypeScopeSchema,

    /// Structural filters applicable to this item, grouped
    pub filter_groups: Vec<FilterGroupSchema>,

    /// Per-stat filters (inline on mod lines)
    pub stats: Vec<TradeStatSchema>,
}

pub struct TypeScopeSchema {
    pub current: String,  // "baseType"
    pub options: Vec<TypeScopeOption>,  // [{id, label}]
}

pub struct FilterGroupSchema {
    pub id: String,         // "misc_filters"
    pub title: String,      // "Miscellaneous"
    pub filters: Vec<FilterSchema>,
}

pub struct FilterSchema {
    pub id: String,         // "ilvl"
    pub text: String,       // "Item Level"
    pub kind: FilterKind,
    pub default_value: Option<FilterValue>,  // from item
    pub enabled: bool,      // default toggle state
}

pub enum FilterKind {
    Range,
    Option { options: Vec<FilterOption> },
}

pub struct FilterOption {
    pub id: Option<String>,  // null = "Any"
    pub text: String,
}

pub struct TradeStatSchema {
    pub stat_index: u32,
    pub trade_id: String,       // "fractured.stat_809229260"
    pub category: String,       // "fractured"
    pub display_text: String,
    pub computed_min: Option<f64>,
    pub enabled: bool,
}
```

### TradeEditValues (sent back from frontend)

```rust
pub struct TradeEditValues {
    pub type_scope: String,
    pub filters: HashMap<String, FilterValue>,  // "ilvl" → {min: 87}
    pub stats: Vec<StatOverride>,               // {index, enabled, min}
}
```

## What poe-trade decides (domain knowledge)

- Which filter groups apply to this item (weapon filters only for weapons, etc.)
- Default values from item properties (ilvl, quality, corrupted, etc.)
- Default toggle states (corrupted defaults ON for corrupted items, etc.)
- Which stats map to trade IDs and in which category
- How to build the query body from user values

## What the frontend decides (presentation only)

- How to render a range filter (two number inputs)
- How to render an option filter (dropdown)
- How to render a stat filter (checkbox + min input on the mod line)
- Where to place filters on the overlay (inline with item elements)
- Animation, styling, layout

## Implementation Steps

1. Parse `filters.json` into Rust types in poe-trade
2. Add `filters.json` fetching + caching alongside existing `stats.json`
3. Implement `trade_edit_schema()` — project item onto filter schema
4. Add Tauri command exposing the schema
5. Build generic frontend renderer
6. Migrate existing Edit Search UI to use schema
7. Add `items.json` + `static.json` fetching/caching (for type search + bulk exchange)

## Item Property → Filter Mapping

This is the domain knowledge that lives in poe-trade:

| Item property | Filter group | Filter ID | Default |
|---------------|-------------|-----------|---------|
| `item_level` | misc_filters | ilvl | item's ilvl |
| `is_corrupted` | misc_filters | corrupted | "true" if corrupted |
| `is_fractured` | misc_filters | fractured_item | "true" if fractured |
| `quality` | misc_filters | quality | item's quality |
| `rarity` | type_filters | rarity | "nonunique" for non-unique |
| `sockets` | socket_filters | sockets | item's socket count |
| `max_link` | socket_filters | links | item's max link |
| `armour` (property) | armour_filters | ar | item's armour value |
| `evasion` (property) | armour_filters | ev | item's evasion value |
| `energy_shield` | armour_filters | es | item's ES value |
| `map_tier` | map_filters | map_tier | item's map tier |
| ... | ... | ... | ... |

This mapping is a small, stable table in poe-trade. When GGG adds a new filter
to `filters.json`, we add a row to this table mapping it to an item property.
Filters without a mapping are still available but start with no default value.
