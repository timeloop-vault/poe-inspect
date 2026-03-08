# poe-data Gap Filling — Way of Working

Recurring process for extending `poe-data` when higher layers (poe-item, poe-eval) need
game knowledge that isn't yet exposed.

## Principle

**`poe-data` is the single source of truth for ALL game knowledge.** Higher layers
(poe-item, poe-eval, app) have zero PoE domain knowledge — they only see what poe-data
exposes. If the GGPK doesn't have the data, poe-data maintains fallback tables that look
identical to callers.

This means callers never need to know whether data came from the GGPK or a maintained
fallback. The API is the same either way.

## Process

### 1. Identify the gap

A higher layer needs game knowledge that `GameData` doesn't expose yet.

Example: poe-eval needs "max number of prefixes for a Rare item" to compute open affixes.

### 2. Research the GGPK

Check whether the data exists in the GGPK:

1. Search `crates/poe-query/dat-schema/_Core.gql` for relevant table schemas
2. If found, note the schema, field types, and byte sizes
3. Verify the table exists and has reasonable data (use `extract_dat` to dump it)

### 3. Decide: GGPK extraction or fallback table

**Prefer GGPK extraction** — it's authoritative and auto-updates with patches.

Use a **fallback table** only when:
- The data genuinely doesn't exist in the GGPK
- The GGPK data is incomplete or unreliable for our use case
- The data requires human curation (e.g., meta-tier thresholds)

Fallback tables live in `poe-data` (not in higher layers) and are exposed through the
same `GameData` API, so callers can't distinguish them from GGPK data.

### 4. Implement

For GGPK data:
1. **poe-dat**: Add row struct in `tables/types.rs`, extraction fn in `tables/extract.rs`
2. **poe-data**: Add table + indexes to `GameData`, add lookup methods
3. **poe-data**: Wire into `load()` function
4. **Tests**: Add extraction test, add GameData lookup tests

For fallback data:
1. **poe-data**: Add a `fallback/` module with the maintained data
2. **poe-data**: Expose through same `GameData` API and lookup methods
3. **Tests**: Test the fallback values match expected game behavior

### 5. Update inventory

Update `docs/ggpk-data-inventory.md` with:
- Which table was added and why
- Which fields are extracted
- Which higher layer drove the need

## History

| Date | Gap | Source | Driven by |
|------|-----|--------|-----------|
| 2026-03-08 | ItemClassCategories | GGPK `ItemClassCategories.datc64` | poe-eval (item class grouping) |
| 2026-03-08 | Rarity (max affixes) | GGPK `Rarity.datc64` | poe-eval (open prefix/suffix count) |
