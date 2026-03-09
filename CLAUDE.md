# PoE Inspect 2

Real-time item evaluation overlay for Path of Exile.

## Project Status

**Phase: Foundation** — GGPK pipeline validated, building core crates iteratively.

### Pipeline Progress

| Crate | Status | Notes |
|-------|--------|-------|
| poe-bundle | **Done** | GGPK extraction, Oodle FFI, patched for 3.28 |
| poe-query | **Done** | Generic dat reader + schema, PQL queries |
| poe-dat (stat_desc) | **Done** | PEST parser + reverse index (15.5k patterns, 100% hit rate) |
| poe-dat (tables) | **Done** | 7 tables extracted: Stats, Tags, ItemClasses, BaseItemTypes, ModFamily, ModType, Mods |
| poe-data | **Done** | `GameData` struct with indexed tables, FK resolution, loader |
| poe-item | **Done** | PEST grammar + resolver, 75 tests, 41 fixtures |
| poe-eval | **Foundation** | Predicates, rules, evaluate, scoring profiles, tier analysis (26 tests) |
| app | **Phase 8b** | Tauri v2 overlay — Phases 1-7 done, 8a done (compound rules UI), 8b next (nested rules) |

**Side track:** poe-rqe (reverse query engine / demand marketplace) — working, independent of main pipeline.
**Future:** poe-craft (probabilistic crafting strategy engine) — see `docs/crafting-tiers.md`.

## Tech Stack (Planned)

- **Rust** (edition 2024, clippy pedantic, `unsafe_code = "forbid"`)
- **TypeScript** (strict mode)
- **Tauri v2** (pending research validation)
- **Cross-platform**: Windows, Linux (SteamOS), macOS

## Project Structure

```
crates/
  poe-dat/         — Read/parse .dat/.dat64 files and stat description files from GGPK
  poe-data/        — Game data types and lookup tables (depends on poe-dat)
  poe-item/        — Parse Ctrl+Alt+C item text into structured types (depends on poe-data)
  poe-eval/        — Evaluate parsed items against user-defined filter rules (depends on poe-item, poe-data)
  poe-bundle/      — (owned) ex-nihil/poe-bundle — Rust library for reading PoE GGPK bundles (Oodle FFI)
  poe-query/       — (owned) ex-nihil/poe-query — Query tool for .dat files using PQL + dat-schema
app/               — Tauri v2 overlay application (future)
fixtures/
  items/           — Shared Ctrl+Alt+C item text fixtures (used by poe-item, poe-eval, etc.)
docs/
  HYPOTHESIS.md    — Core vision and scope
  RESEARCH_SYNTHESIS.md — Consolidated research findings
  research/        — Detailed research outputs
  ggpk-data-inventory.md — Tables, stat descriptions, schema maintenance
_reference/        — (gitignored) Local clones of prior projects for reference
```

Each crate has its own `CLAUDE.md` with detailed scope, decisions, and plan.

## Architecture Decisions

- **Own the data pipeline**: Parse GGPK data directly (via poe-bundle) rather than depending on RePoE. Avoids 1000+ lines of reshaping code that v1 needed.
- **Own the schema**: Community dat-schema is a starting point, not a dependency. We must be able to reverse-engineer new fields ourselves on league launch (see `docs/ggpk-data-inventory.md`).
- **poe-bundle/poe-query are owned code**: Converted from submodules. Patched for PoE 3.21.2+ (MurmurHash64A, datc64, updated schema).
- **poe-dat owns DatFile**: The datc64 binary reader (`DatFile`) lives in poe-dat. poe-query depends on poe-dat and extends `DatFile` with spec-driven reading via `DatFileQueryExt` extension trait. One source of truth for the core reader.
- **poe-dat = "our queries"**: Not a new generic dat layer. Typed table extractions for the ~15 specific tables we need, with compile-time field offsets. Can be used as a library or CLI.
- **Section-first parser**: Split item text on `--------` separators → classify sections → parse with typed handlers.
- **Iterative build order**: poe-dat → poe-data → poe-item → poe-eval → app. Prove each layer before building the next.
- **`Arc<GameData>` pattern**: Single shared game data instance, loaded once, passed by reference.
- **Template-keyed lookups**: Stat translations indexed by template string (what appears in item text), not by stat ID.
- **PEST grammar for stat descriptions**: The `stat_descriptions.txt` format is complex (ranges, transforms, multi-stat, all languages inline). Must use formal grammar, not ad-hoc parsing. See `docs/research/stat-description-file-format.md`.
- **poe-data owns ALL PoE domain knowledge**: All game-specific constants, mechanic rules, mapping tables, and classification logic live in `crates/poe-data/` — either extracted from GGPK (`game_data.rs`) or hardcoded with documentation (`domain.rs`). Higher-layer crates (`poe-item`, `poe-eval`, `app`) have zero PoE knowledge. The `domain-knowledge-reviewer` agent enforces this.

## Dependency Graph

```
poe-bundle (GGPK extraction, Oodle FFI)
    ↓
poe-dat (datc64 binary reader + typed table extraction + stat descriptions)
    ↑ depends on             ↓
poe-query (spec-driven       poe-data (game-domain types + indexed lookups)
  reader, PQL queries)            ↓
                              poe-item (Ctrl+Alt+C parser)    poe-eval (rules + scoring)
                                  ↓                               ↓
                                  └──────── app (Tauri overlay) ──┘
```

## Conventions

### Rust
- Edition 2024, MSRV aligned with latest stable
- `clippy::pedantic` enabled, `unsafe_code = "forbid"`
- `cargo fmt` before commit
- Use `thiserror` for error types

### TypeScript
- Strict mode, no `any` without justification
- Biome or similar for formatting/linting

### Documentation
- Decisions and research go in `docs/`
- Keep docs concise and actionable — avoid over-planning (lesson from poe-inspect v1)

## Build Notes

- **poe-bundle/poe-query** are excluded from the workspace — build from their own directories
- **cmake required** for poe-bundle (Oodle C++ lib). VS BuildTools cmake path must be on PATH
- **dat-schema** must be copied to `target/debug/dat-schema/` next to the poe_query binary for dev testing

## Related Projects (Reference Only)

These repos contain useful research and patterns but are NOT dependencies:

| Repo | What's useful |
|------|--------------|
| poe-inspect (v1) | Format analysis, architecture thinking |
| poe-item (TS) | TypeScript item parser, type definitions |
| poe-item-rust | Rust item struct design |
| poe-item-filter | Data pipeline (repoe-fork), economy integration |
| poe-agents | poe.ninja API, PoB CLI, agent patterns |

## Key Data Sources

- **GGPK (primary)**: 911 datc64 tables + 41 stat description files. See `docs/ggpk-data-inventory.md`
- **dat-schema**: Community-maintained schemas at `poe-tool-dev/dat-schema` (GraphQL SDL)
- **repoe-fork (fallback)**: `https://repoe-fork.github.io/{file}.json` — pre-processed game data
- **poe.ninja**: Economy data + builds API (rate limit: 12 req / 5 min)
- **GGG public API**: Character data at `/character-window/get-*` (no auth needed)

## Current League

- **PoE1 League**: Mirage (3.28) — launched March 6, 2026
- **poe.ninja slug**: `Mirage`
