# Pseudo Stats Design

## What Are Pseudo Stats

Pseudo stats are aggregated values computed from multiple mods on an item. The trade site uses them for searching (e.g., "total maximum Life >= 140" matches items where life mod + strength mod contribute together). They're essential for practical item evaluation.

The trade API has 296 pseudo stats. ~25 are commonly used for pricing (life, resistances, attributes, damage, speed). The rest are niche (Atzoatl rooms, Logbook, etc.).

## Data Sources

### Trade API (`/api/trade/data/stats` — Pseudo category)

Provides the **list** of pseudo stats with IDs and display text:
- `pseudo.pseudo_total_life` → `"+# total maximum Life"`
- `pseudo.pseudo_total_fire_resistance` → `"+#% total to Fire Resistance"`
- `pseudo.pseudo_total_strength` → `"+# total to Strength"`

Does NOT provide the aggregation formula (which component stats sum together).

### GGPK ModFamily table

Each mod belongs to one or more **families** — semantic tags that group related mods:
- `FireResistance` — all fire resistance mods
- `Strength` — all strength mods
- `IncreasedLife` — all flat life mods
- `AllResistances` — mods that give all elemental resistances

**Key finding: families are individual tags, not compound.** A "+Strength and Dexterity" mod has `families: ["Strength", "IncreasedLife"]` — it belongs to BOTH families separately. This means querying "all mods in the Strength family" automatically includes dual-attribute mods.

The full family list is committed at `crates/poe-data/data/mod_families.txt` for reference when adding new pseudo definitions.

### What's NOT in GGPK

- **Aggregation formulas** — which families/stats contribute to which pseudo
- **Multipliers** — Strength gives 0.5 life per point (game mechanic, not declared in data)
- **Cross-family rules** — "pseudo total life = IncreasedLife + Strength × 0.5"

These must be hardcoded as PoE domain knowledge.

## Core Design Principle: Pseudos Are Just Stats

**Pseudo stats use the exact same types as regular stats.** No separate `PseudoStat` type, no `PseudoStatValue` predicate. The only difference is how the value is computed (aggregate vs single mod).

The data pipeline:

| Layer | Regular stat | Pseudo stat |
|-------|-------------|-------------|
| **stat_id** | `"base_maximum_life"` | `"pseudo_total_life"` |
| **stat_template** | `"+# to maximum Life"` | `"(Pseudo) +# total maximum Life"` |
| **StatSuggestion** | `{ template, stat_ids, kind: Single }` | `{ template, stat_ids, kind: Single }` |
| **ResolvedStatLine** | `{ display_text, stat_ids, values }` | `{ display_text, stat_ids, values }` |
| **ResolvedMod** | `display_type: Prefix` | `display_type: Pseudo` |
| **StatValue predicate** | `stat_ids: ["base_maximum_life"]` | `stat_ids: ["pseudo_total_life"]` |
| **Profile editor UI** | User picks from autocomplete | User picks from autocomplete |

Downstream consumers (poe-eval, poe-trade, app UI) don't need to know or care that a stat is pseudo. They just see a stat_id, a template, and a value — same as any other stat.

## Architecture

### Ownership

| Concern | Owner | Notes |
|---------|-------|-------|
| ModFamily list | poe-data | Extracted from GGPK, committed as reference |
| Family → stat_id mapping | poe-data | Built at load time from Mods table |
| Pseudo definitions | poe-data/domain.rs | Hardcoded: pseudo_id → [(stat_ids, multiplier)] |
| Pseudo as StatSuggestion | poe-data | Injected into `stat_suggestions_for_query()` + `all_stat_templates()` |
| Computing pseudo values | poe-item | During resolve(): scan stat lines, sum matching values |
| Pseudo on item | poe-item | Synthetic `ResolvedMod` with `display_type: Pseudo` |
| Evaluating pseudos | poe-eval | Existing `StatValue` predicate — no changes needed |
| Searching by pseudo | poe-trade | Maps pseudo stat_ids to trade API `pseudo.pseudo_*` IDs |

### Data Flow

```
poe-data/domain.rs: pseudo definitions (pseudo_id → [(stat_ids, multiplier)])
    +
poe-data: pseudo templates injected into StatSuggestion + all_stat_templates()
    ↓ (used during item resolution)
poe-item resolver: for each pseudo definition, scan item's stat lines,
                   sum values × multiplier → synthetic ResolvedMod
    ↓
ResolvedItem: synthetic pseudo mods alongside regular mods
    ↓                              ↓
poe-eval: StatValue predicate     poe-trade: pseudo search filters
          scans all stat lines              map to trade API IDs
          (including pseudo mods)
```

### Types — Reusing Existing Structures

No new item types needed. Pseudo stats use existing types:

```rust
// poe-item/types.rs — add Pseudo variant to existing enum
pub enum ModDisplayType {
    Prefix, Suffix, Implicit, Crafted, Enchant, Unique,
    Pseudo,  // ← new variant
}

// Pseudo mods are synthetic ResolvedMod entries:
ResolvedMod {
    header: ModHeader { source: Computed, slot: Pseudo, ... },
    stat_lines: vec![ResolvedStatLine {
        display_text: "(Pseudo) +142 total maximum Life",
        stat_ids: Some(vec!["pseudo_total_life"]),
        values: vec![ValueRange { current: 142, min: 0, max: 0 }],
        is_reminder: false,
    }],
    display_type: ModDisplayType::Pseudo,
    is_fractured: false,
}
```

poe-data definitions:

```rust
// poe-data/domain.rs
pub struct PseudoDefinition {
    pub id: &'static str,       // "pseudo_total_life"
    pub label: &'static str,    // "(Pseudo) +# total maximum Life"
    pub components: &'static [PseudoComponent],
}

pub struct PseudoComponent {
    pub stat_ids: &'static [&'static str],  // ["base_maximum_life"]
    pub multiplier: f64,                     // 0.5
    pub required: bool,                      // true = pseudo only shows when this has a value
}
```

### Pseudo Computation in poe-item

During `resolve()`, after all mods are resolved:

1. For each pseudo definition:
   - For each component (stat_ids, multiplier):
     - Scan all stat lines on the item for matching stat_ids
     - Sum `value × multiplier`
   - If any `required` component has no match → skip
   - If total > 0 → create synthetic `ResolvedMod` with `display_type: Pseudo`
2. Pseudo mods stored in `ResolvedItem.pseudo_mods`, included in `all_mods()` iterator

### Profile Editor — Zero Changes

The stat suggestion autocomplete in poe-data already returns `StatSuggestion` entries. Pseudo templates are injected into the same system:

1. User types "total life" in the stat template input
2. Autocomplete shows both `"+# to maximum Life"` and `"(Pseudo) +# total maximum Life"`
3. User picks the pseudo one
4. `StatCondition` gets `stat_ids: ["pseudo_total_life"]` — same as any other stat
5. `StatValue` predicate evaluates against the item's pseudo mod stat lines
6. Works with existing op/value comparison — no UI changes

### Phase 1 Pseudo Definitions

See `crates/poe-data/src/domain.rs` — `PSEUDO_DEFINITIONS` for the full list.

Components use explicit stat_ids (including local_ variants for weapon mods).
ModFamily list (`crates/poe-data/data/mod_families.txt`) is a discovery aid for
finding related stat_ids when authoring new definitions, but definitions
themselves must list exact stat_ids — families are mod-level anti-stacking tags,
not stat-level semantic groups.

## Implementation Steps

1. ✅ **poe-data**: Extract and commit `mod_families.txt`
2. ✅ **poe-data/domain.rs**: Define `PSEUDO_DEFINITIONS` with explicit stat_ids + multipliers
3. ✅ **poe-data**: Expose definitions on GameData, inject into autocomplete suggestions
4. ✅ **poe-item/types.rs**: `Pseudo` variant on `ModDisplayType`, `ModSlot`, `ModSource::Computed`
5. ✅ **poe-item/resolver.rs**: Compute pseudo values → synthetic `ResolvedMod` entries
6. ✅ **poe-eval**: No changes — existing `StatValue` predicate works via `all_mods()`
7. ✅ **app**: Collapsible pseudo section on overlay, auto-expands in trade edit mode
8. **poe-trade**: Map pseudo stat_ids to trade API IDs for search (TODO)

## Reference

- Trade API pseudo stats: `crates/poe-trade/tests/fixtures/trade_stats_3.28.json` (Pseudo category, 296 entries)
- ModFamily list: `crates/poe-data/data/mod_families.txt` (7,678 entries)
- Pseudo definitions: `crates/poe-data/src/domain.rs` — `PSEUDO_DEFINITIONS`
