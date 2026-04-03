# PoE Inspect 2

Real-time item evaluation overlay for Path of Exile.

## Project Status

**Phase: App Integration** — Core crates done, building trade features and overlay UX.

### Pipeline Progress

| Crate | Status | Notes |
|-------|--------|-------|
| poe-dat (stat_desc) | **Done** | PEST parser + reverse index (15.5k patterns, 100% hit rate) |
| poe-dat (tables) | **Done** | 7 tables extracted: Stats, Tags, ItemClasses, BaseItemTypes, ModFamily, ModType, Mods |
| poe-data | **Done** | `GameData` struct with indexed tables, FK resolution, loader |
| poe-item | **Done** | PEST grammar + resolver, 98 tests, 68 fixtures |
| poe-eval | **Done** | Predicates, rules, evaluate, scoring profiles, tier analysis (52 tests) |
| poe-trade | **Phases 1-5 done** | Trade API client, stats index, query builder, rate-limited HTTP, filter schema, 10 Tauri commands, TradePanel UI |
| app | **Phase 8f done** | Tauri v2 overlay — inline trade editing, price check, pseudo stats, socket filters |

**Side track:** poe-rqe (reverse query engine / demand marketplace) — working, independent of main pipeline.
**Future:** poe-craft (probabilistic crafting strategy engine) — see `docs/crafting-tiers.md`.

## Tech Stack (Planned)

- **Rust** (edition 2024, clippy pedantic, `unsafe_code = "forbid"`)
- **TypeScript** (strict mode)
- **Tauri v2** (validated, in production use)
- **Cross-platform**: Windows, Linux (SteamOS), macOS

## Project Structure

```
crates/
  poe-dat/         — Read/parse datc64 files and stat description files from GGPK
  poe-data/        — Game data types and lookup tables (depends on poe-dat)
  poe-item/        — Parse Ctrl+Alt+C item text into structured types (depends on poe-data)
  poe-eval/        — Evaluate parsed items against user-defined filter rules (depends on poe-item, poe-data)
  poe-trade/       — Trade API client: stats index, query builder, price lookup (depends on poe-item, poe-data)
app/               — Tauri v2 overlay application
pipeline/
  extract-game-data/ — Standalone crate to extract datc64 tables from GGPK (depends on poe-bundle)
  update-game-data.sh — Convenience script to extract and copy game data into the repo
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

- **Own the data pipeline**: Parse GGPK data directly rather than depending on RePoE. Avoids 1000+ lines of reshaping code that v1 needed. GGPK extraction uses [poe-bundle](https://github.com/ex-nihil/poe-bundle) (external dependency, not vendored).
- **Own the schema**: Community dat-schema is a starting point, not a dependency. We must be able to reverse-engineer new fields ourselves on league launch (see `docs/ggpk-data-inventory.md`).
- **poe-dat owns DatFile**: The datc64 binary reader (`DatFile`) lives in poe-dat. One source of truth for the core reader.
- **poe-dat = "our queries"**: Not a new generic dat layer. Typed table extractions for the ~15 specific tables we need, with compile-time field offsets. Can be used as a library or CLI.
- **Section-first parser**: Split item text on `--------` separators → classify sections → parse with typed handlers.
- **Iterative build order**: poe-dat → poe-data → poe-item → poe-eval → app. Prove each layer before building the next.
- **`Arc<GameData>` pattern**: Single shared game data instance, loaded once, passed by reference.
- **Template-keyed lookups**: Stat translations indexed by template string (what appears in item text), not by stat ID.
- **PEST grammar for stat descriptions**: The `stat_descriptions.txt` format is complex (ranges, transforms, multi-stat, all languages inline). Must use formal grammar, not ad-hoc parsing. See `docs/research/stat-description-file-format.md`.
- **poe-data owns ALL PoE domain knowledge**: All game-specific constants, mechanic rules, mapping tables, and classification logic live in `crates/poe-data/` — either extracted from GGPK (`game_data.rs`) or hardcoded with documentation (`domain.rs`). Higher-layer crates (`poe-item`, `poe-eval`, `app`) have zero PoE knowledge. The `domain-knowledge-reviewer` agent enforces this.
- **Data-first rule**: Before hardcoding any PoE game knowledge, check the GGPK data first. See `docs/ggpk-data-deep-dive.md` for the full inventory. The process is:
  1. Check `$GGPK_DATA_DIR/TABLE_INVENTORY.txt` for relevant tables (set `GGPK_DATA_DIR` to extracted datc64 directory)
  2. Check `ClientStrings` for display text (`ItemPopup*`, `ItemDisplay*`, `ModDescriptionLine*`)
  3. If the data exists in GGPK: extract it in poe-dat, expose it in poe-data
  4. If the data is a trade API convention (not in GGPK): hardcode it with a comment citing the source and date, e.g. `// Trade API convention, not in GGPK (verified 2026-03-15)`
  5. Extract core tables with: `./pipeline/update-game-data.sh <poe_path>`

## Dependency Graph

```
poe-bundle (external: github.com/ex-nihil/poe-bundle — GGPK extraction)
    ↓ (used by pipeline only, not linked into app)
pipeline/extract-game-data → crates/poe-data/data/*.datc64

poe-dat (datc64 binary reader + typed table extraction + stat descriptions)
    ↓
poe-data (game-domain types + indexed lookups)
   /     \
poe-item    |
 /    \     |
poe-eval  poe-trade (trade API client)
    \       /
     app (Tauri overlay)
```

## Conventions

### Rust
- Edition 2024, MSRV aligned with latest stable
- `clippy::pedantic` enabled, `unsafe_code = "forbid"`
- Use `thiserror` for error types
- **Always fix clippy warnings** — even if unrelated to your changes. Zero warnings policy.

### TypeScript
- Strict mode, no `any` without justification
- Biome for formatting/linting
- **Always fix biome errors** — even if unrelated to your changes. Run `npx biome check --write --unsafe .` from `app/` and fix any remaining issues before committing.

### TypeScript type generation (ts-rs)
Rust types with `#[derive(ts_rs::TS)]` generate TypeScript interfaces in `app/src/generated/`. To regenerate after changing Rust types:
```sh
cargo run --manifest-path pipeline/generate-ts-types/Cargo.toml -- -o app/src/generated
```
This is a standalone binary (not test-based). Root types in `poe-item`, `poe-eval`, `poe-trade`, `poe-data` are exported with all transitive dependencies.

### Pre-commit checklist (MANDATORY before every commit)
Run all of these and fix any errors before committing. A pre-commit hook enforces this automatically.
1. `cargo fmt` — format all Rust code
2. `cargo clippy --workspace --tests` — zero warnings policy
3. `cargo clippy --manifest-path app/src-tauri/Cargo.toml --tests` — Tauri app (excluded from workspace)
4. `cd app && npx tsc --noEmit` — TypeScript strict type checking
5. `cd app && npx biome check --write --unsafe .` — format + lint frontend

### Dependencies
- **Keep dependencies up to date** — run `cargo update` (workspace) + `cargo update --manifest-path app/src-tauri/Cargo.toml` (Tauri) + `cd app && npm update` regularly. Prefer updating deps over adding overrides/workarounds for vulnerabilities.
- **Commit all lock files** when dependencies change — both `Cargo.lock` files and `app/package-lock.json` are tracked

### Git / CLI
- **Never chain `cd` with other commands** (`cd path && cmd`) — chained commands don't match allowed permissions and trigger prompts. Use flags when possible (`cargo check -p poe-trade`, `cargo check --manifest-path app/src-tauri/Cargo.toml`). When a tool requires cwd (vite, biome), run `cd app` as a separate Bash call first, then the command in the next call (cwd persists between calls). Always `cd` back to repo root after.

### Documentation
- Decisions and research go in `docs/`
- Keep docs concise and actionable — avoid over-planning (lesson from poe-inspect v1)

## Build Notes

- **Pre-commit hook**: Run `git config core.hooksPath .githooks` after cloning to activate the pre-commit hook (Rust fmt/clippy + TS tsc/biome). The hook is tracked in `.githooks/`.
- **pipeline/extract-game-data** is excluded from the workspace — depends on [poe-bundle](https://github.com/ex-nihil/poe-bundle) (requires cmake for Oodle C++ lib)
- **Game data extraction**: Run `./pipeline/update-game-data.sh <poe_install_dir>` to update datc64 files

## Related Projects (Reference Only)

These repos contain useful research and patterns but are NOT dependencies:

| Repo | What's useful |
|------|--------------|
| poe-inspect (v1) | Format analysis, architecture thinking |
| poe-item (TS) | TypeScript item parser, type definitions |
| poe-item-rust | Rust item struct design |
| poe-item-filter | Data pipeline, economy integration |
| poe-agents | poe.ninja API, PoB CLI, agent patterns |

## Key Data Sources

- **GGPK (primary)**: 911 datc64 tables + 41 stat description files. See `docs/ggpk-data-inventory.md`
- **dat-schema**: Community-maintained schemas at `poe-tool-dev/dat-schema` (GraphQL SDL)
- **Trade API**: `https://www.pathofexile.com/api/trade/data/*` — stats, items, filters (unique→base_type mapping)
- **poe.ninja**: Economy data + builds API (rate limit: 12 req / 5 min)
- **GGG public API**: Character data at `/character-window/get-*` (no auth needed)

## Current League

- **PoE1 League**: Mirage (3.28) — launched March 6, 2026
- **poe.ninja slug**: `Mirage`
