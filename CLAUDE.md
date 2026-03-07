# PoE Inspect 2

Real-time item evaluation overlay for Path of Exile.

## Project Status

**Phase: Foundation** — Workspace scaffolded, research complete. Building core crates iteratively.

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
  poe-bundle/      — [git submodule] ex-nihil/poe-bundle — Rust library for reading PoE GGPK bundles (Oodle FFI)
  poe-query/       — [git submodule] ex-nihil/poe-query — Query tool for .dat files using PQL + dat-schema
app/               — Tauri v2 overlay application (future)
docs/
  HYPOTHESIS.md    — Core vision and scope
  RESEARCH_SYNTHESIS.md — Consolidated research findings
  research/        — Detailed research outputs
_reference/        — (gitignored) Local clones of prior projects for reference
```

Each crate has its own `CLAUDE.md` with detailed scope, decisions, and plan.

## Architecture Decisions

- **Own the data pipeline**: Parse GGPK data directly (via poe-bundle) rather than depending on RePoE. Avoids 1000+ lines of reshaping code that v1 needed (lookup inversion, runtime regex, fragile mod filtering, duplicate indexing).
- **Section-first parser**: Split item text on `--------` separators → classify sections (data-assisted) → parse each with typed handlers. NOT code-path parsing (v1's mistake of hardcoded ifs/thens).
- **Iterative build order**: poe-dat → poe-data → poe-item → poe-eval → app. Prove each layer with tests before building the next.
- **`Arc<GameData>` pattern**: Single shared game data instance, loaded once, passed by reference.
- **Template-keyed lookups**: Stat translations indexed by template string (what appears in item text), not by stat ID. The parser sees text, not IDs.
- **Submodules for GGPK tooling**: poe-bundle and poe-query are git submodules (by ex-nihil). May need updates for newer PoE versions / dat-schema changes.

## Dependency Graph

```
poe-bundle (submodule, GGPK extraction)
    ↓
poe-dat (parse .dat files + stat descriptions)
    ↓
poe-data (game-domain types + indexed lookups)
    ↓
poe-item (Ctrl+Alt+C parser)          poe-eval (rules + scoring)
    ↓                                      ↓
    └──────────── app (Tauri overlay) ─────┘
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

- **GGPK (primary)**: .dat files via poe-bundle, stat descriptions from `Metadata/StatDescriptions/*.txt`
- **dat-schema**: Community-maintained schemas at `poe-tool-dev/dat-schema` (GraphQL SDL)
- **repoe-fork (fallback)**: `https://repoe-fork.github.io/{file}.json` — pre-processed game data
- **poe.ninja**: Economy data + builds API (rate limit: 12 req / 5 min)
- **GGG public API**: Character data at `/character-window/get-*` (no auth needed)

## Current League

- **PoE1 League**: Mirage (3.28) — launched March 6, 2026
- **poe.ninja slug**: `Mirage`
