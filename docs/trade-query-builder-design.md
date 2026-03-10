# Trade Query Builder — Phase 6 Design

> The overlay becomes an interactive trade search builder.
> "Edit Search" mode transforms the item display into the query configuration UI.
>
> Reference sketch: `Screenshots/Skärmbild 2026-03-10 170802.png` (Death Needle wand mockup)

## Core Idea

The item display IS the query builder. No separate filter panel, no context switching.
The user reads the item, toggles what matters, adjusts values, and searches — all inline.

This works because the overlay already renders tier badges, type badges, and roll quality
bars on every mod line. Adding checkboxes and value adjusters is the same visual language.

## Mode Toggle

**Option C**: A toggle button in the trade panel switches between "Simple View" (current)
and "Edit Search" mode. Keeps the default view clean, gives explicit user control.

```
[Edit Search]  [Price Check]  [Open Trade]
```

When "Edit Search" is active, interactive controls appear on every filterable element.
Clicking "Price Check" or "Open Trade" uses whatever the current filter state is.

---

## Filter Dimensions

Everything the user might want to narrow or broaden in a trade search:

### 1. Type Search Scope (GGPK hierarchy)

| Scope | GGPK Table | Example | Trade API field |
|-------|-----------|---------|-----------------|
| `BaseType` | `BaseItemTypes` | "Demon's Horn" | `query.type: "Demon's Horn"` |
| `ItemClass` | `ItemClasses` | "Wands" | `filters.type_filters.filters.category: "weapon.wand"` |
| `Any` | — | (no restriction) | (omit both fields) |

**Default**: `BaseType` for rares, name+base for uniques.
**UI**: Clickable breadcrumb near header: `Demon's Horn → Wand → Any`

**Ownership**:
- poe-data: base type → item class mapping (GGPK FK), item class → trade category (`domain.rs`)
- poe-trade: builds the query with the right specificity level
- app: renders the toggle, sends the choice

### 2. Level Requirement

Filter by minimum item level (useful for ilvl-gated mods).

| Filter | Trade API field |
|--------|-----------------|
| Min ilvl | `filters.misc_filters.filters.ilvl.min` |

**Default**: Not filtered.
**UI**: Toggleable in edit mode — "Item Level: 83" becomes clickable → "ilvl ≥ 83".

### 3. Attribute Requirements

Dex/Str/Int requirements help narrow to items usable by specific builds.
Not commonly filtered but useful for niche searches.

**Note**: The trade API doesn't have direct attribute requirement filters.
This would need to be a local post-filter or just informational display.
**Decision**: Display-only for now, not a search filter. Revisit if needed.

### 4. Prefix/Suffix Counts (Open Affixes)

Critical for crafting — "has 1 open suffix" is a common search criterion.

| Filter | Trade API field |
|--------|-----------------|
| Open prefixes | `filters.misc_filters.filters.crafted` (indirect) |
| Open suffixes | (not directly filterable — only `#prefixes`, `#suffixes` pseudo stats) |

**Trade API approach**: Use pseudo stats `pseudo.open_prefix` / `pseudo.open_suffix`
if available, otherwise count-based filters.
**Decision**: Phase 6b — needs research on exact trade API pseudo stat IDs.

### 5. Per-Stat Filters (The Big One)

Each stat line gets:
- **Checkbox**: Include/exclude from search
- **Min value**: Editable number, pre-filled from relaxation %
- **Disabled state**: Stats that can't be mapped to trade IDs get a dimmed "?" indicator

```
 [✓] [T1 P] +139 to maximum Life    [min: 118] 98%
 [✓] [T6 P] +113 to Armour          [min: 96 ] 55%
 [ ] [T3 P] +35 to Armour, +24 Life [min: —  ] 40%   ← unchecked = excluded
 [✓] [T6 S] Regen 41.1 Life/s       [min: 34 ] 28%
 [✓] [T2 S] +42% Cold Resistance    [min: 35 ] 0%
 [ ] [T3 S] +37% Fire Resistance    [min: —  ] 25%   ← unchecked
```

**Default**: All mappable stats checked, min values from global relaxation %.
**UI**: Checkbox left of badges, min value input right of (or replacing) roll bar.

### 6. Tier/Rank Slider (Advanced, Future)

Instead of setting a min value directly, the user could set "T1-T3 only" on a stat.
This would look up the tier's value range from poe-data and set the min accordingly.

**Example**: "+# to maximum Life" T1-T3 in GGPK covers 130-144, 120-129, 110-119.
Setting "T3 or better" → min: 110.

**Ownership**:
- poe-data: tier → value range mapping (from Mods table)
- poe-trade: translates tier constraint to min value
- app: renders the tier slider

**Decision**: Phase 6c — requires tier range data from poe-data that we don't expose yet.

### 7. Rarity Filter

| Level | Trade API |
|-------|-----------|
| Non-unique (default for rares) | `rarity: "nonunique"` |
| Any rarity | (omit) |

**UI**: Small toggle near header, or part of the base type breadcrumb.

### 8. Corrupted / Fractured / Identified

Already handled automatically in `build_query()`. Could be made toggleable.
**Decision**: Auto for now. Toggleable later if users request it.

---

## Data Flow

```
                        ┌─────────────────────┐
                        │   "Edit Search"      │
                        │   mode activated      │
                        └──────────┬────────────┘
                                   │
                        ┌──────────▼────────────┐
                        │ preview_trade_query()  │ ← lightweight, no HTTP
                        │ Returns MappedStat[]   │
                        │ (which stats mapped,   │
                        │  computed min values)  │
                        └──────────┬────────────┘
                                   │
                        ┌──────────▼────────────┐
                        │ Frontend builds UI:    │
                        │ - checkboxes per stat  │
                        │ - min value inputs     │
                        │ - base type toggle     │
                        └──────────┬────────────┘
                                   │
                        ┌──────────▼────────────┐
                        │ User toggles filters   │
                        │ Adjusts values         │
                        └──────────┬────────────┘
                                   │
                        ┌──────────▼────────────┐
                        │ price_check() with     │
                        │ TradeFilterConfig:     │
                        │ - type_scope           │
                        │ - stat overrides[]     │
                        │ - (future: ilvl, etc.) │
                        └───────────────────────┘
```

### Preview Command

New Tauri command `preview_trade_query` — runs `build_query()` but doesn't execute
the HTTP search. Returns `QueryBuildResult` with `mapped_stats` so the frontend knows:
- Which stats have trade IDs (can be toggled)
- What min values relaxation computed (pre-fills the inputs)
- Which stats couldn't be mapped (shown as disabled)

This is cheap (no HTTP, no rate limit cost) and can be called on "Edit Search" click.

---

## Rust Types

### poe-trade/types.rs

```rust
/// User's filter overrides for a trade search.
/// Sent from frontend when in "Edit Search" mode.
pub struct TradeFilterConfig {
    /// How specific the type filter should be.
    pub type_scope: TypeSearchScope,
    /// Per-stat overrides, indexed by flat stat position
    /// (order: enchants → implicits → explicits, skipping reminder text).
    pub stat_overrides: Vec<StatFilterOverride>,
}

/// Matches the GGPK hierarchy: BaseItemTypes → ItemClasses → no restriction.
pub enum TypeSearchScope {
    /// Filter by exact base item type (GGPK BaseItemTypes, e.g., "Demon's Horn").
    BaseType,
    /// Filter by item class only (GGPK ItemClasses, e.g., "Wands" → "weapon.wand").
    ItemClass,
    /// No type restriction.
    Any,
}

pub struct StatFilterOverride {
    /// Flat index into the item's non-reminder stat lines.
    pub stat_index: u32,
    /// Whether this stat is included in the search.
    pub enabled: bool,
    /// Min value override. None = use relaxation-computed default.
    pub min_override: Option<f64>,
}
```

### QueryBuildResult enrichment

```rust
/// Info about a stat that was considered during query building.
pub struct MappedStat {
    /// Flat index (position in enchants → implicits → explicits iteration).
    pub stat_index: u32,
    /// Trade stat ID if mapped (e.g., "explicit.stat_3299347043").
    pub trade_id: Option<String>,
    /// Display text for UI label.
    pub display_text: String,
    /// Relaxation-computed min value (default for the input).
    pub computed_min: Option<f64>,
    /// Whether this stat was included in the final query.
    pub included: bool,
}
```

---

## Ownership Summary

| Knowledge | Owner | Why |
|-----------|-------|-----|
| Base type → item class | poe-data (GGPK FK) | Game data |
| Item class → trade category | poe-data/domain.rs | GGG trade system knowledge |
| Tier → value range | poe-data (Mods table) | Game data |
| Query construction from filters | poe-trade/query.rs | Trade API logic |
| Filter config types | poe-trade/types.rs | Used by query builder |
| UI for toggling filters | app (overlay) | Presentation |
| Filter state management | app (useTradeFilters hook) | UI state |

**Hard rule**: The app never decides what trade API fields to set. It sends a
`TradeFilterConfig` to poe-trade, which translates it into the correct API body.

---

## Implementation Phases

### Phase 6a: Foundation (current)
- `StatFilterOverride`, `BaseTypeFilter`, `MappedStat` types in poe-trade
- `build_query()` accepts optional `TradeFilterConfig`
- `preview_trade_query` Tauri command
- Backward compatible — `None` filter config = today's behavior

### Phase 6b: Frontend — stat toggles
- `useTradeFilters` hook (state management)
- "Edit Search" toggle button in TradePanel
- Checkboxes on ModLine when in edit mode
- Min value inputs per stat
- Base type breadcrumb toggle

### Phase 6c: Advanced filters
- Tier/rank slider (needs poe-data tier range exposure)
- Open prefix/suffix filters (needs trade API pseudo stat research)
- Item level filter
- Rarity toggle

### Phase 6d: Polish
- Remember filter state per item class (e.g., always exclude fire res on boots)
- Default filter presets (e.g., "life build", "ES build")
- Keyboard shortcuts for common toggles
