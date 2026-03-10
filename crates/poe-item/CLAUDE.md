# poe-item

Parses PoE's Ctrl+Alt+C item clipboard text into structured, type-safe item representations.

## Status

**Core complete** — PEST grammar (Pass 1) + Resolver (Pass 2). 98 tests, 68 fixtures.

## Scope

- Parse raw item text → structured item with fully resolved mods, tiers, and values
- Handle all item types: equipment, jewels, flasks, gems, currency, divination cards, maps, etc.
- **Ctrl+Alt+C only** — we require the advanced copy format with `{ }` mod headers.
  Ctrl+C (no headers) is not supported. The `{ }` headers eliminate all section-classification
  ambiguity (flask properties vs mods, mod grouping, etc.) that made previous parsers fail.
- Future: PoE2 format support (different requirements format, rune modifiers)

## Does NOT own

- Game data loading — that's `poe-data`
- Item evaluation/scoring — that's `poe-eval`
- Clipboard access or hotkey handling — that's `app`

## Architecture: PEST Grammar + Resolver (Two-Pass)

### Why PEST

The user has built this parser THREE times (poe_item_deconstructor in JS, poe-item-rust,
poe-inspect v1 in Rust). Each time, the section classification logic became an unmaintainable
nest of if/else/state-machine code. The Ctrl+Alt+C format is complex enough that ad-hoc
parsing creates maintenance nightmares — every league GGG adds new section types, property
formats, or modifier headers that break hardcoded checks.

A PEST grammar makes the format rules **declarative and self-documenting**. When GGG changes
something, you update the grammar rules, not hunt through code branches.

We already use PEST successfully for `stat_descriptions.txt` parsing in poe-dat (10k+ descriptions,
170k variants, 100% hit rate). Same approach here.

### Pass 1: PEST Grammar (structural parse)

The grammar defines the item text format: sections, separators, line types, modifier headers,
property formats, stat lines. PEST parses the text into a typed parse tree.

What PEST handles:
- Section structure (separator-delimited blocks)
- Header section (Item Class, Rarity, name/base lines)
- Divination Card rarity (`Rarity::DivinationCard`)
- Property lines (Key: Value with optional augmented marker)
- Requirements section
- Sockets section
- Item Level section
- Note section (`Note: ~b/o 35 chaos` — GGG trade pricing annotation)
- Modifier headers (`{ Prefix Modifier "Name" (Tier: N) — Tags }`)
- Stat lines with value ranges (`+68(65-68) to maximum Mana`)
- Reminder text (`(parenthesized text)`)
- Enchant lines (ending with `(enchant)`)
- Influence markers (`Shaper Item`, etc.)
- Status keywords (`Corrupted`, `Mirrored`, `Unidentified`, `Split`, `Transfigured`)
- Unknown sections (preserved, not dropped)

### Pass 2: Rust Resolver (data-dependent disambiguation)

Uses `GameData` for things the grammar can't know:
- **Magic item base type extraction**: Magic items embed base in name ("Seething Divine Life Flask of Staunching"). Need base item lookup to split.
- **Stat line → stat ID**: Feed display text into `ReverseIndex.lookup()` to get stat IDs + values.
- **Generic section classification**: Content-based analysis to classify unstructured sections:
  - Enchants (lines ending with `(enchant)` suffix → synthetic `ResolvedMod` with `ModSlot::Enchant`)
  - Properties (lines with `": "` pattern)
  - Descriptions (currency effects, item instructions)
  - Flavor text (unique/div card lore)
  - Usage instructions (dropped — "Right click to use", etc.)
- **Gem data extraction**: Dedicated path for gems — tags, description, stats, quality effects, Vaal variant
- **Flask property vs modifier**: Flask base properties look like mods. Need game data to tell apart.
- **Tier verification**: Verify extracted tier against mod database ranges.

### Architecture diagram

```
Clipboard text (Ctrl+Alt+C)
    │
    ▼
[PEST grammar] ─── item.pest
    │
    ▼
Parse tree (pest::Pairs)
    │
    ▼
[Tree walker] ─── walks parse tree → RawItem
    │
    ▼
RawItem (structured but unresolved)
    │
    ▼
[Resolver] ─── uses GameData (base items, mods, ReverseIndex)
    │
    ▼
ResolvedItem (fully parsed and enriched)
```

### File structure

```
src/
  lib.rs              — public API
  grammar.pest        — PEST grammar for Ctrl+Alt+C format
  parser.rs           — PEST parse + tree walker → RawItem
  types.rs            — RawItem, ResolvedItem, section types
  resolver.rs         — data-dependent disambiguation using GameData
tests/
  parse_fixtures.rs   — Pass 1 tests
  resolve_fixtures.rs — Pass 2 tests
../../fixtures/items/ — shared Ctrl+Alt+C item fixtures (workspace root)
```

## The 16 Known Ambiguities

Documented in `docs/research/parsing-strategy.md` (sections B.1–B.16). Key ones:

| ID | Problem | Handled by |
|----|---------|-----------|
| B.1 | Magic item base type embedded in name | Resolver (base item lookup) |
| B.3 | Weapon sub-header vs first property | Grammar (known item_base set) |
| B.7 | Flask base properties vs modifiers | N/A — Ctrl+Alt+C `{ }` headers separate them |
| B.8 | Multi-line enchants | Grammar (enchant suffix detection) |
| B.9 | Reminder text vs stat lines | Grammar (parenthesized lines) |
| B.11 | Negative value ranges `1(10--10)%` | Grammar (regex handles double dash) |
| B.13 | Variable section ordering | Grammar (ordered alternation) |
| B.16 | Locale decimals `9,60` vs `9.60` | Grammar (both formats) |

## Stat Line → Mod Identification Pipeline

The reverse index (`poe-dat::stat_desc::ReverseIndex`) maps display text → stat ID + value.

### Ctrl+Alt+C (advanced format)
The `{ }` header tells us the mod name, type, and tier. Stat lines under one header belong
to the same mod. The reverse index gives us stat IDs + values for downstream evaluation.

### Multi-line stat descriptions (`\n` in format strings)
Some stats produce two visual lines from one format string. Both lines fall under the same
`{ }` header. Join with `\n` before calling `ReverseIndex::lookup()`.

## Reference implementations (what went wrong)

| Project | Language | Problem |
|---------|----------|---------|
| `poe_item_deconstructor` | JS | `determine_mods()`: 120-line if/else branching on section count (1/2/3) |
| `poe-item-rust` | Rust | `NextExpectedLine` state machine, abandoned at classification wall |
| poe-inspect v1 | Rust | `next_state()` cascade, 200+ line property if/else, 16 ambiguities |

Test fixtures from these projects: `_reference/poe-item-rust/test/data/`,
`poe-inspect/packages/poe-parser/src/tests/`, `_reference/poe-item-filter/`.

## Dependencies

- `poe-data` — for `GameData` lookups during resolver pass
- `pest` + `pest_derive` — grammar parser

## Plan

1. ~~Write this CLAUDE.md with architecture decision~~ ✓
2. ~~Copy test fixtures from reference projects into `fixtures/items/`~~ ✓
3. ~~Write PEST grammar (`src/grammar.pest`)~~ ✓
4. ~~Write tree walker (`parser.rs`) — PEST parse tree → RawItem types~~ ✓
5. ~~Write output types (`types.rs`) — RawItem, section enums~~ ✓
6. ~~Test structural parsing against all fixtures (no game data needed)~~ ✓ (47 parse tests, 68 fixtures)
7. ~~Expand fixture coverage~~ ✓ (68 fixtures) — see `fixtures/items/COVERAGE.md` for gaps
8. ~~Write resolver (`resolver.rs`) — GameData-dependent disambiguation~~ ✓
9. ~~Test full pipeline: text → RawItem → ResolvedItem~~ ✓ (40 resolver tests + 11 unit tests)

## Fixture-Driven Development

**Fixtures are the foundation of parser correctness.** The Ctrl+Alt+C format varies
by item type, league mechanics, and GGG patches. When something fails:

1. Get a real Ctrl+Alt+C text of the failing item
2. Add it as a fixture in `fixtures/items/`
3. Add tests that assert the expected parse result
4. Fix the grammar/resolver/types until the test passes

See `fixtures/items/COVERAGE.md` for the full coverage matrix and gap analysis.
Remaining gaps: unique jewel (Watcher's Eye), veiled items, dagger/claw, heist contracts.
