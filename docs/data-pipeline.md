# Data Pipeline Architecture

How game data flows from GGPK to item evaluation.

## Pipeline Overview

```
GGPK file (Content.ggpk, ~35GB)
    │
    ├── poe-bundle ──► bundles ──► decompress (Oodle) ──► index
    │
    ├── poe-query ──► dat-schema + DatReader ──► raw table rows
    │
    ├── poe-dat
    │   ├── stat_desc ──► parse stat_descriptions.txt ──► ReverseIndex
    │   └── tables ──► extract ~15 dat tables ──► typed rows
    │
    └── poe-data ──► GameData (domain types + indexed lookups)
            │
            ├── poe-item ──► parse Ctrl+Alt+C text ──► ParsedItem
            │
            └── poe-eval ──► score against profiles ──► evaluation result
```

## Decision: Own the GGPK Pipeline

We parse GGPK data directly via poe-bundle instead of depending on RePoE
(repoe-fork.github.io JSON files). Reasons:

- **Avoids 1000+ lines of reshaping code** that v1 needed to transform RePoE's
  data model into our access patterns
- **No external dependency** on a community-maintained JSON export
- **Day-zero league support** — we can read updated tables from a patched GGPK
  without waiting for RePoE to update
- **Schema ownership** — we can reverse-engineer new fields ourselves

RePoE remains a **fallback** for quick validation or fields we haven't mapped yet.

## Stat Description Pipeline (DONE)

### What stat_descriptions.txt contains

The file `Metadata/StatDescriptions/stat_descriptions.txt` (19MB UTF-16LE, 429k lines)
is the game's translation layer between internal stat IDs and display text. It defines
10,858 description blocks with 169,847 format variants across 11 languages.

Each block maps one or more stat IDs to a format string:

```
description
    2 attack_minimum_added_fire_damage attack_maximum_added_fire_damage
    1
        # "Adds {0} to {1} Fire Damage to Attacks"
```

Format strings support:
- **Placeholders**: `{0}`, `{1}`, `{0:+d}` (signed)
- **Ranges**: `#` (any), `N` (exact), `N|M` (min/max), `!N` (negated)
- **Transforms**: `negate`, `divide_by_ten_1dp`, `milliseconds_to_seconds`, etc. (40+ kinds)
- **Multi-line**: `\n` in format strings produces two visual lines from one stat
- **All languages inline**: English, Portuguese, Chinese, Japanese, Korean, etc.

Full format spec: `docs/research/stat-description-file-format.md`

### Reverse Index

The `ReverseIndex` maps display text back to stat IDs and raw values:

```
"+92 to maximum Life"  →  (base_maximum_life, 92)
"Adds 5 to 10 Fire Damage to Attacks"  →  (attack_minimum_added_fire_damage=5, attack_maximum_added_fire_damage=10)
"33% reduced Recovery rate"  →  (local_flask_recovery_speed_+%, -33)   [negate transform reversed]
```

**Implementation**: Template-key matching. Replace `{N}` placeholders with `\x00` marker in
format strings, replace numbers in display text with same marker, HashMap lookup. O(1) for
common cases, subset enumeration for format strings containing literal numbers.

**Performance**: 15,500 patterns indexed, builds in ~1.8s, 100% hit rate on real character data.

### What the Reverse Index Does NOT Do

The reverse index resolves **stat text → stat IDs + values**. It does not identify which
**mod** (name, tier, hybrid grouping) produced the stat. That requires a second lookup step.

## Stat Resolution → Mod Identification

This is the critical handoff between `poe-dat` (stat resolution) and `poe-data`/`poe-item`
(mod identification). There are three distinct scenarios:

### 1. Single-stat mods (most common)

One mod produces one stat line. Reverse index gives us the stat ID and value.
Cross-reference against the mod database to find the mod name and tier.

```
Display: "+103 to maximum Life"
Reverse index: base_maximum_life = 103
Mod database: "Virile" (Tier 6), prefix, range 100-114
```

### 2. Hybrid mods (e.g., "Beetle's" armour + stun recovery)

One mod produces **two separate stat lines** that resolve independently via
separate single-stat description blocks. The reverse index doesn't know they
came from the same mod.

```
Display line 1: "13% increased Armour"
Reverse index: physical_damage_reduction_rating_+% = 13

Display line 2: "7% increased Stun and Block Recovery"
Reverse index: base_stun_recovery_+% = 7
```

Both are "Beetle's" (Tier 6), but nothing in stat_descriptions.txt links them.
The connection only exists in the mod database (mods.json / Mods.dat), which
lists both stat IDs under one mod entry.

**In Ctrl+Alt+C format**: the `{ }` header groups them — both lines appear under
`{ Prefix Modifier "Beetle's" (Tier: 6) }`. The item parser handles grouping.

**In Ctrl+C format**: no grouping indicator. Must cross-reference the mod database
to discover they share a mod. Adjacent lines with stat IDs that appear in the
same mod entry are likely from the same hybrid mod.

### 3. Multi-line stat descriptions (`\n` in format strings)

One stat ID has a format string that produces **two visual lines** via `\n`.
The reverse index expects the full joined string.

```
Format string: "Grants Immunity to Bleeding for {0} seconds if used while Bleeding\n
                Grants Immunity to Corrupted Blood for {0} seconds if used while affected by Corrupted Blood"

Display (PoE API): single string with embedded newline
Display (Ctrl+Alt+C): two lines under one { } header
Display (Ctrl+C): two consecutive lines, no grouping
```

The caller must join these lines with `\n` before calling `ReverseIndex::lookup()`.

### Summary of responsibilities

| Layer | Responsibility |
|-------|---------------|
| `poe-dat` reverse index | Display text → stat IDs + raw values |
| `poe-data` mod database | Stat IDs + values → mod name, tier, hybrid grouping |
| `poe-item` (Ctrl+Alt+C) | Parse `{ }` headers for mod name/tier, group stat lines, join multi-line stats with `\n`, call reverse index per stat line |
| `poe-item` (Ctrl+C) | Call reverse index per line, use mod database for disambiguation, heuristic joining for multi-line stats |

## Tables Still Needed (poe-dat/tables)

~15 dat tables from GGPK, extracted via poe-query's DatReader. See
`docs/ggpk-data-inventory.md` for the full list. Key ones:

| Table | Rows | Purpose |
|-------|------|---------|
| BaseItemTypes | 5.3k | Base type → item class, tags, requirements |
| Mods | 39.3k | Mod definitions: stat IDs, value ranges, spawn weights, groups |
| Stats | 22.7k | Stat definitions (links stat IDs to internal indices) |
| Tags | ~500 | Mod spawn weight tags |
| ModType | ~100 | Prefix/suffix classification |
| CraftingBenchOptions | ~500 | Bench crafts with costs |
| ClientStrings | ~10k | UI text translations |

## Test Data

- **Stat descriptions**: real 19MB file extracted from 3.28 GGPK
- **Character mods**: 52 real mod strings from a 3.28 Mirage character
  (`crates/poe-dat/tests/test_data/scripter_boomboom_mods.txt`)
- **Ctrl+Alt+C fixtures**: 17 curated items + 15 session items in
  `_reference/poe-inspect/data/test-fixtures/poe1/` (advanced format)
- **Ctrl+C fixtures**: corresponding simple format versions in
  `_reference/poe-inspect/data/poe1/simple.txt`
