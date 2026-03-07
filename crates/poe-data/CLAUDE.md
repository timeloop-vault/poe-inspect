# poe-data

Game-domain data layer. Transforms raw parsed data (from poe-dat) into the lookup tables that the item parser and evaluator need.

## Scope

- Define game-domain types: `Mod`, `BaseItem`, `StatTranslation`, `ItemClass`, `CraftingBenchOption`, etc.
- Build pre-indexed lookup tables optimized for the parser's access patterns
- Provide `GameData` struct (intended to be wrapped in `Arc<GameData>`) as single entry point
- Load data from extracted GGPK files (via poe-dat), cache to disk as serialized format

## Does NOT own

- Raw file parsing — that's `poe-dat`
- Item text parsing — that's `poe-item`
- Evaluation rules — that's `poe-eval`
- Network requests or API calls

## Key Design Decisions

- **Template-keyed lookups**: Stat translations indexed by template string (what appears in item text), NOT by stat ID. This is the #1 lesson from v1 — the parser sees text, not IDs, so the lookup must go text → mod.
- **Pre-filtered mods**: Only include rollable mods relevant to item evaluation. V1 spent ~190 lines filtering out non-rollable mods (essences, fossils, delve, etc.) at runtime with fragile heuristics. We filter during data build.
- **Pre-computed tier tables**: For each mod, pre-compute the tier table (tier number, stat ranges, required level) grouped by mod group. V1 computed this at query time across ~330 lines.
- **Base item indexing**: Base items indexed by name for direct lookup from parsed item text. V1 had duplicate implementations of this.
- **`Arc<GameData>` pattern**: GameData is built once (potentially expensive), then shared immutably via `Arc`. Confirmed useful pattern from v1.

## Lookup Access Patterns

The item parser needs these lookups (derived from v1's 8 join operations):

| Lookup | Key | Returns |
|--------|-----|---------|
| Stat translation | template string | stat IDs, value extraction pattern |
| Mod by stat combo | set of stat IDs | matching Mod(s) with tier info |
| Base item | item name | BaseItem with item class, tags, implicits |
| Item class | class name | ItemClass |
| Mod tier table | mod group | ordered tiers with ranges |
| Craftable mods | item tags | applicable bench/currency crafts |

## Dependencies

- `poe-dat` — for reading raw .dat files and stat descriptions

## Plan

1. Define domain types (Mod, BaseItem, StatTranslation, etc.)
2. Build loader that reads poe-dat output → GameData
3. Build pre-indexed lookup tables with the access patterns above
4. Serialization for disk caching (avoid re-parsing GGPK every launch)
5. Tests: round-trip a known mod through template lookup → mod identification → tier resolution
