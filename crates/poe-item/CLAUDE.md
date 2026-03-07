# poe-item

Parses PoE's Ctrl+Alt+C (and Ctrl+C) item clipboard text into structured, type-safe item representations.

## Scope

- Parse raw item text → `ParsedItem` struct with fully resolved mods, tiers, and values
- Handle all item types: equipment, jewels, flasks, gems, currency, divination cards, maps, etc.
- Support both Ctrl+Alt+C (advanced, with `{ }` mod headers) and Ctrl+C (basic) formats
- Future: PoE2 format support (different requirements format, rune modifiers)

## Does NOT own

- Game data loading — that's `poe-data`
- Item evaluation/scoring — that's `poe-eval`
- Clipboard access or hotkey handling — that's `app`

## Key Design Decision: Section-First Parser

The critical architectural decision. NOT a code-path parser (v1's mistake). Three layers:

### Layer 1: Structural Tokenization
Split item text on `--------` separator lines into raw sections. No interpretation yet.

### Layer 2: Section Classification
Classify each section by inspecting its content. This is where game data enters — some sections can only be identified by looking up content in `GameData`:
- **Unambiguous**: Item Class (always first section, starts with `Item Class:`), Requirements (`Requirements:` header)
- **Data-assisted**: Base type detection (is a line a base item name? need BaseItem lookup), modifier sections vs property sections (need mod template matching)

### Layer 3: Typed Section Parsing
Each classified section gets a dedicated typed parser. Mod sections parse `{ }` headers to extract mod name, type (prefix/suffix/implicit/etc.), tier, and tags, then parse the stat lines with value ranges.

## The 16 Known Ambiguities

Documented in detail in `docs/research/parsing-strategy.md` (sections B.1–B.16). The hardest:

- **B.1 Magic item base type**: Magic items embed base type in the name ("Seething Divine Life Flask of Staunching"). Need base item lookup to split.
- **B.3 Base type line detection**: In the header, the base type line has no label. Must match against known base items.
- **B.7 Flask properties vs modifiers**: Flask stat lines look identical to modifier lines.
- **B.13 Variable section ordering**: Section order varies by item type — can't rely on position alone.

## Ctrl+Alt+C Mod Header Format

```
{ Mod Name — Type (Tier) — Tags: tag1, tag2 }
stat line with [range] values
optional second stat line
```

Example:
```
{ Merciless Prefix — Prefix Modifier "Tier 1" — default }
Adds [53–77] to [110–130] Physical Damage
```

## Stat Line → Mod Identification Pipeline

The reverse index (`poe-dat::stat_desc::ReverseIndex`) maps display text → stat ID + value.
But identifying which **mod** (name, tier, hybrid grouping) a stat line belongs to requires
additional steps that differ between formats:

### Ctrl+Alt+C (advanced format)
The `{ }` header already tells us the mod name, type, and tier. Stat lines grouped under
one header belong to the same mod — even hybrid mods like "Beetle's" (Tier 6) that produce
two separate stat lines (`13% increased Armour` + `7% increased Stun and Block Recovery`).

**This crate's job:** parse the `{ }` header and group stat lines under it, then use the
reverse index on each stat line to get stat IDs + values for downstream evaluation.

### Ctrl+C (simple format)
No `{ }` headers exist. Each stat line appears independently. Hybrid mod lines appear
consecutively but with no grouping indicator.

**This crate's job:** use the reverse index to resolve each line to stat ID + value, then
use `poe-data` mod database to disambiguate:
1. Look up which mods in `mods.json` contain each stat ID with a matching value range
2. Adjacent stat lines that share a mod candidate → likely the same hybrid mod
3. This is inherently ambiguous — some stat IDs appear in both single-stat and hybrid mods

### Multi-line stat descriptions (`\n` in format strings)
Some stat descriptions produce two visual lines from a single stat ID. Example:
```
Grants Immunity to Bleeding for 7 seconds if used while Bleeding
Grants Immunity to Corrupted Blood for 7 seconds if used while affected by Corrupted Blood
```
This is ONE stat with a `\n` in its format string. The reverse index expects the joined
string (with real `\n`). In Ctrl+Alt+C format, both lines fall under the same `{ }` header,
so this crate should join them with `\n` before calling `ReverseIndex::lookup()`.

In Ctrl+C format, these appear as two consecutive lines with no indicator they're joined.
Heuristic: if a line fails reverse lookup, try joining it with the next line (separated by
`\n`) and retry.

## Dependencies

- `poe-data` — for `GameData` lookups during parsing

## Plan

1. Section splitter (Layer 1) — trivial, test with real fixtures
2. Section classifier (Layer 2) — needs GameData, start with equipment items
3. Header parser for `{ }` mod annotations
4. Stat line parser with value range extraction
5. Full item parser composing all layers
6. Test against real item fixtures from `_reference/poe-item-filter` and v1 test data
7. Ctrl+C fallback parser (less info, harder, lower priority)
