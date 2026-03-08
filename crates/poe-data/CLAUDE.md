# poe-data

Game data lookup tables built from parsed GGPK data.

## Status

**Minimal foundation done** — tables loaded, indexed, FK resolution working. No domain types yet; poe-item will drive what we reshape.

## Scope

- Hold all 9 extracted tables (poe-dat row structs, no reshaping)
- Build id-based indexes for fast string lookup
- Resolve FK row indices to human-readable strings
- Provide `GameData` struct (intended for `Arc<GameData>`)
- Slot for stat description `ReverseIndex` (set separately)

## Does NOT own

- Raw file parsing — that's `poe-dat`
- Item text parsing — that's `poe-item`
- Evaluation rules — that's `poe-eval`
- New domain types (yet) — waiting for poe-item to drive requirements

## Architecture

```
src/
  lib.rs         — re-exports
  game_data.rs   — GameData struct, indexes, loader, FK helpers
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

## Future (when poe-item needs it)

- Domain types if raw row structs aren't the right shape
- Pre-filtered mod tables (rollable only)
- Pre-computed tier tables (mod group → ordered tiers)
- Template-keyed lookups for stat text → mod identification
- Disk caching (serialization to avoid re-parsing GGPK every launch)

## Dependencies

- `poe-dat` — datc64 reader, table extraction, stat descriptions
