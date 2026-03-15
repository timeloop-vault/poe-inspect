# poe-data

Game data lookup tables built from parsed GGPK data.

## Status

**Done** — tables loaded, indexed, FK resolution working. Domain knowledge in `domain.rs` (pseudo stat definitions, trade mappings, item class capabilities). ClientStrings extracted. `all_stat_templates()` and `stat_suggestions_for_query()` include pseudo templates.

## Scope

- Hold all extracted tables (poe-dat row structs)
- Build id-based indexes for fast string lookup
- Resolve FK row indices to human-readable strings
- Provide `GameData` struct (intended for `Arc<GameData>`)
- Slot for stat description `ReverseIndex` (set separately)
- **Domain knowledge** (`domain.rs`): pseudo stat definitions, item class capabilities, trade category mappings
- **ClientStrings**: extracted from GGPK (8,264 rows), lookup API
- **Pseudo definitions**: `PSEUDO_DEFINITIONS` with explicit stat_ids + multipliers, injected into autocomplete
- **ModFamily list**: committed at `data/mod_families.txt` for reference

## Does NOT own

- Raw file parsing — that's `poe-dat`
- Item text parsing — that's `poe-item`
- Evaluation rules — that's `poe-eval`
- Trade API HTTP / query building — that's `poe-trade`

## Architecture

```
src/
  lib.rs         — re-exports
  game_data.rs   — GameData struct, indexes, loader, FK helpers, stat suggestions
  domain.rs      — PoE domain knowledge: pseudo definitions, trade mappings, item class capabilities
data/
  mod_families.txt — ModFamily reference list (7,678 entries)
```

### GameData contents

| Field | Type | Index |
|-------|------|-------|
| stats | `Vec<StatRow>` | `stat_by_id: HashMap<String, usize>` |
| tags | `Vec<TagRow>` | `tag_by_id` |
| item_classes | `Vec<ItemClassRow>` | `item_class_by_id` |
| item_class_categories | `Vec<ItemClassCategoryRow>` | `item_class_category_by_id` |
| base_item_types | `Vec<BaseItemTypeRow>` | `base_item_by_name` |
| mod_families | `Vec<ModFamilyRow>` | — |
| mod_types | `Vec<ModTypeRow>` | — |
| mods | `Vec<ModRow>` | `mod_by_id` |
| rarities | `Vec<RarityRow>` | `rarity_by_id` |
| reverse_index | `Option<ReverseIndex>` | — |

### Loading

```rust
let gd = poe_data::load(&dir)?;  // reads 9 datc64 files from dir
let stat = gd.stat("base_maximum_life");
let tag_name = gd.tag_id(some_fk);
```

### Testing

```sh
cargo test -p poe-data --test load_game_data -- --nocapture
```
Requires extracted datc64 files in `%TEMP%/poe-dat/`.

## Future

- Pre-computed tier tables (mod group → ordered tiers)
- Disk caching (serialization to avoid re-parsing GGPK every launch)
- More pseudo definitions (currently ~25 common ones)

## Dependencies

- `poe-dat` — datc64 reader, table extraction, stat descriptions
