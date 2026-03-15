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

## Architecture

### Ownership

| Concern | Owner | Notes |
|---------|-------|-------|
| ModFamily list | poe-data | Extracted from GGPK, committed as reference |
| Family → stat_id mapping | poe-data | Built at load time from Mods table |
| Pseudo definitions | poe-data/domain.rs | Hardcoded: pseudo_id → [(family, multiplier)] |
| Resolved pseudo → stat_ids | poe-data | Resolved at load time: definitions × family index |
| Computing pseudo values | poe-item | During resolve(): scan stat lines, sum matching values |
| Pseudo stat on item | poe-item | `ResolvedItem.pseudo_stats: Vec<PseudoStat>` |
| Evaluating pseudos | poe-eval | `PseudoStatValue` predicate reads `item.pseudo_stats` |
| Searching by pseudo | poe-trade | Maps pseudo_id → trade API `pseudo.pseudo_*` ID |

### Data Flow

```
GGPK ModFamily table
    ↓ (extracted at build/load time)
poe-data: family_name → Set<stat_id>   (e.g., "Strength" → {"additional_strength", "additional_strength_and_dexterity", ...})
    +
poe-data/domain.rs: pseudo definitions  (e.g., pseudo_total_life → [("IncreasedLife", 1.0), ("Strength", 0.5)])
    ↓ (resolved at load time)
poe-data: pseudo_id → Vec<(stat_id, multiplier)>  (flat lookup table)
    ↓ (used during item resolution)
poe-item resolver: for each pseudo definition, scan item's stat lines,
                   sum values × multiplier → PseudoStat { id, label, value }
    ↓
ResolvedItem.pseudo_stats: Vec<PseudoStat>
    ↓                              ↓
poe-eval: PseudoStatValue         poe-trade: pseudo search filters
          predicate checks                   map to trade API IDs
          item.pseudo_stats
```

### Types

```rust
// poe-item/types.rs
pub struct PseudoStat {
    /// Pseudo stat ID (e.g., "pseudo_total_life")
    pub id: String,
    /// Display label with (Pseudo) prefix (e.g., "(Pseudo) +# total maximum Life")
    pub label: String,
    /// Computed value (sum of contributing stats × multipliers)
    pub value: f64,
}

// poe-data/domain.rs
pub struct PseudoDefinition {
    /// Matches trade API ID suffix (e.g., "pseudo_total_life")
    pub id: &'static str,
    /// Display template from trade API (e.g., "+# total maximum Life")
    pub label: &'static str,
    /// Component families with multipliers
    pub components: &'static [PseudoComponent],
}

pub struct PseudoComponent {
    /// ModFamily name (e.g., "Strength")
    pub family: &'static str,
    /// Multiplier applied to the stat value (e.g., 0.5 for Strength → Life)
    pub multiplier: f64,
    /// If true, pseudo only shows when this component has a value
    pub required: bool,
}
```

### Pseudo Evaluation in poe-item

During `resolve()`, after all mods are resolved:

1. For each pseudo definition from `GameData`:
   - For each component (family, multiplier):
     - Find the stat_ids associated with that family
     - Scan all stat lines on the item for matching stat_ids
     - Sum `value × multiplier`
   - If any `required` component has no match → skip this pseudo
   - If total > 0 → add `PseudoStat { id, label: "(Pseudo) {template}", value }` to item

### Phase 1 Pseudo Definitions (~20 rules)

Based on Awakened PoE Trade's proven definitions:

**Resistances:**
- `pseudo_total_fire_resistance` ← families: FireResistance, AllResistances, AllResistancesWithChaos
- `pseudo_total_cold_resistance` ← families: ColdResistance, AllResistances, AllResistancesWithChaos
- `pseudo_total_lightning_resistance` ← families: LightningResistance, AllResistances, AllResistancesWithChaos
- `pseudo_total_chaos_resistance` ← families: ChaosResistance, AllResistancesWithChaos
- `pseudo_total_elemental_resistance` ← sum of fire + cold + lightning totals

**Attributes:**
- `pseudo_total_strength` ← families: Strength, AllAttributes
- `pseudo_total_dexterity` ← families: Dexterity, AllAttributes
- `pseudo_total_intelligence` ← families: Intelligence, AllAttributes

**Life/Mana/ES:**
- `pseudo_total_life` ← families: IncreasedLife (required), Strength @ 0.5 multiplier
- `pseudo_total_mana` ← families: IncreasedMana (required?), Intelligence @ 0.5 multiplier
- `pseudo_total_energy_shield` ← families: (flat ES family)

**Speed:**
- `pseudo_total_attack_speed` ← families: (attack speed family)
- `pseudo_total_cast_speed` ← families: (cast speed family)
- `pseudo_increased_movement_speed` ← families: (movement speed family)

**Note:** Some pseudos don't map cleanly to families (e.g., "+# to Level of Socketed Gems" families). These may need stat_id-based definitions instead of family-based. The system should support both.

## Implementation Steps

1. **poe-data**: Extract and commit `mod_families.txt` (full list of family names)
2. **poe-data**: Build `family_name → Set<stat_id>` index at load time
3. **poe-data/domain.rs**: Define `PSEUDO_DEFINITIONS` with family + multiplier rules
4. **poe-data**: Expose resolved `pseudo_id → Vec<(stat_id, multiplier)>` on GameData
5. **poe-item/types.rs**: Add `PseudoStat` type + `pseudo_stats` field on `ResolvedItem`
6. **poe-item/resolver.rs**: Compute pseudo values during resolve()
7. **poe-eval**: Add `PseudoStatValue` predicate
8. **poe-trade**: Map pseudo_ids to trade API IDs for search
9. **app**: Display pseudo stats on overlay, add to profile editor suggestions

## Reference

- Awakened PoE Trade pseudo rules: `_reference/awakened-poe-trade/renderer/src/web/price-check/filters/pseudo/index.ts`
- Trade API pseudo stats: `crates/poe-trade/tests/fixtures/trade_stats_3.28.json` (Pseudo category, 296 entries)
- ModFamily dump test: `cargo test -p poe-data --test load_game_data -- dump_mod_details --nocapture`
