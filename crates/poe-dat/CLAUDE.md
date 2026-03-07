# poe-dat

Read and parse game data files from the PoE GGPK bundle.

## Status

**Active development** — Building stat description parser.

## Scope

- Parse stat description files (the key translation layer between stat IDs and display text)
- Extract specific tables from GGPK via poe-bundle/poe-query's DatReader
- Expose typed access to the ~15 tables needed for item evaluation
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

## Future Work (after stat descriptions)
- Table extraction for BaseItemTypes, Mods, Stats, Tags, etc.
- Uses poe-query's DatReader for dat table access
- CLI for debugging/exploration
