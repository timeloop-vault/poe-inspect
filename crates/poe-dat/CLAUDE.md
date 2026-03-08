# poe-dat

Read and parse game data files from the PoE GGPK bundle.

## Status

**Core complete** — Stat description parser + reverse index + table extraction all done.

## Scope

- Parse stat description files (the key translation layer between stat IDs and display text)
- Read raw datc64 binary files directly (minimal reader, no dependency on poe-query at runtime)
- Extract 9 typed tables: Stats, Tags, ItemClasses, ItemClassCategories, BaseItemTypes, ModFamily, ModType, Mods, Rarity
- Can be used as a library (by poe-data) or CLI (for debugging)

## Does NOT own

- Game-domain types (mods, base items, etc.) — that's `poe-data`
- Bundle extraction / GGPK reading — that's `poe-bundle`
- Any interpretation of what the data *means*

## Current Work: Stat Description Parser

### What
Parse `metadata/statdescriptions/stat_descriptions.txt` (30MB, 429k lines, UTF-16LE) into structured data and a reverse lookup index.

### Architecture
```
src/
  stat_desc/
    mod.rs          — public API
    grammar.pest    — PEST grammar for the file format
    parser.rs       — walks PEST parse tree, builds types
    types.rs        — StatDescription, Variant, Range, Transform, LangBlock
    reverse.rs      — reverse index: display text → stat ID + values
  lib.rs
```

### Grammar Elements
1. `description` blocks: stat count + stat IDs, variant lines per language
2. Range syntax: `#` (any), `N` (exact), `N|M` (range), `#|N`, `N|#`
3. Format strings: `{0}`, `{0:+d}`, `{1}` placeholders
4. 28 value transforms: `negate N`, `milliseconds_to_seconds N`, `divide_by_* N`, etc.
5. `no_description <stat_id>` — stats with no visible text
6. `include "path"` — file hierarchy (smaller files include larger)
7. `lang "Name"` blocks — all 10 localizations inline
8. `canonical_line`, `reminderstring <id>` — metadata on variant lines

### Key Design Decisions
- **PEST grammar** — format is too complex for ad-hoc if/then parsing; edge cases accumulate each league
- **Isolated module** — stat_desc/ has own grammar, types, clean API boundary
- **All languages stored** — not English-only; localization support from day one
- **Reverse index is the end goal** — `"+92 to maximum Life"` → `(base_maximum_life, 92)`
- **Test against real data** — parse the actual 30MB file, not synthetic data

### Reverse Index: Scope and Boundaries

The reverse index maps **display text → stat IDs + raw values**. That's all it does.

**It does NOT** identify which mod (name, tier) produced the stat. That requires cross-referencing
stat IDs + values against `mods.json` — which is `poe-data`'s job.

**Hybrid mods** (e.g., "Beetle's" Tier 6 → `13% increased Armour` + `7% increased Stun and Block
Recovery`): each stat line resolves independently via separate single-stat descriptions. The reverse
index doesn't know they came from the same mod. Grouping them is the caller's responsibility
(`poe-item` uses `{ }` headers from Ctrl+Alt+C format).

**Multi-line stat descriptions** (`\n` in format strings): some stats produce two visual lines from
one format string. The caller must join these lines with `\n` before calling `lookup()`. Example:
`"Grants Immunity to Bleeding for 7 seconds...\nGrants Immunity to Corrupted Blood for 7 seconds..."`

### Format Reference
See `docs/research/stat-description-file-format.md` for complete format specification.

## Table Extraction (dat_reader + tables)

### Architecture
```
src/
  dat_reader.rs        — minimal datc64 binary reader (no poe-query dependency)
  tables/
    mod.rs             — public API
    types.rs           — typed row structs (StatRow, ModRow, BaseItemTypeRow, etc.)
    extract.rs         — extraction functions with hardcoded byte offsets
```

### datc64 Format Notes
- Row count: u32 LE at byte 0
- Fixed rows: bytes 4..(4 + row_count * row_size)
- BB marker: 8 bytes `0xBBBBBBBBBBBBBBBB`
- Variable data section starts at the marker (string/list offsets are relative to marker position)
- Strings: `ref|string` = u64 offset (8 bytes in row), UTF-16LE null-terminated in data section
- FK: 16 bytes (u64 row index + u64 key hash), null = `0xFEFEFEFEFEFEFEFE`
- Lists: 16 bytes (u64 length + u64 offset), elements in data section
- Enums: u32 (4 bytes)
- `i16` fields: 2 bytes in datc64 (NOT padded to 4 — confirmed empirically for Mods.HASH16)

### Tables Extracted
| Table | Rows (3.28) | Row Size | Key Fields |
|-------|-------------|----------|------------|
| Stats | 22,749 | 105 | id, is_local, is_weapon_local, is_virtual |
| Tags | 1,353 | 28 | id |
| ModFamily | — | — | id |
| ModType | — | — | name |
| ItemClasses | 99 | 153 | id, name, category (FK), can_have_veiled_mods |
| BaseItemTypes | 5,334 | — | id, item_class (FK), width, height, name, drop_level, implicit_mods, tags |
| Mods | 39,291 | 654 | id, mod_type, level, 6×stat_keys, domain, name, generation_type, families, 6×stat_ranges, spawn_weights, tags, is_essence_only, max_level |
| Rarity | ~4 | — | id, min_mods, max_mods, max_prefix, max_suffix, text |
| ItemClassCategories | ~10 | — | id, text |

### Extracting Raw Files
Use `extract_dat` binary (in poe-query crate):
```sh
cd crates/poe-query
cargo run --bin extract_dat -- -p "D:/games/PathofExile"
# Writes to %TEMP%/poe-dat/{table}.datc64
```

### Testing
```sh
cargo test -p poe-dat --test extract_tables -- --nocapture
```
Requires extracted datc64 files in `%TEMP%/poe-dat/`.
