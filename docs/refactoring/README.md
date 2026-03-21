# Refactoring Plans

Structural improvements identified from a full codebase review (2026-03-21).
Each file in this directory is a self-contained plan for one refactoring effort.

## Priority Order

| # | Plan | Target | Effort | Status |
|---|------|--------|--------|--------|
| 1 | [app-lib-split](app-lib-split.md) | `app/src-tauri/lib.rs` (1966 lines) | ~4h | DONE |
| 2 | [app-component-splits](app-component-splits.md) | `ItemOverlay.tsx`, `ProfileSettings.tsx` | ~8h | DONE |
| 3 | [poe-data-domain-split](poe-data-domain-split.md) | `domain.rs` (930 lines) | ~2h | DONE |
| 4 | [quick-wins](quick-wins.md) | Small dedup/helpers across crates | ~1h | DONE |
| 5 | [future-splits](future-splits.md) | poe-item resolver, poe-trade query | — | WATCH |
| — | [remaining-findings](remaining-findings.md) | Bugs, enhancements, platform gaps from review | — | BACKLOG |
