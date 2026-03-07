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
