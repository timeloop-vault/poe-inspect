# Development Log

Status snapshots for picking up where we left off. Newest first.

---

## 2026-03-12 — End-of-Session Status

### What Got Done Today

- **"Unscalable Value" handling** (poe-item): Strip `— Unscalable Value` suffix before reverse index lookup. Flag stats on unique items (e.g., Doryani's Prototype) now resolve stat_ids. `is_unscalable: bool` on `ResolvedStatLine` for downstream consumers.
- **Domain-based mod resolution** (poe-data/poe-item): Correct local vs non-local stat_ids using GGPK mod domain + base type tags. Abyss jewel evasion resolves to `base_evasion_rating` (non-local), armour evasion resolves to `local_base_evasion_rating`.
- **New predicates** (poe-eval): `SocketCount`, `LinkCount`, `Quality` — all wired into schema, profile editor, and tests.
- **Quality filter** (poe-trade + app): Trade queries include quality min filter, Edit Search UI control.
- **"Superior" prefix stripping** (poe-item): Base type names no longer carry the quality prefix.
- **Doryani's Prototype fixture** added with assertion test (6 mods, 4 unscalable).
- **Abyss jewel fixture** with local/non-local stat_id assertion test.

### Project-Wide Status

#### Pipeline Crates

| Crate | Status | Tests | Notes |
|-------|--------|-------|-------|
| poe-bundle | Done | — | GGPK extraction, Oodle FFI, patched for 3.28 |
| poe-query | Done | — | Generic dat reader + schema, PQL queries |
| poe-dat | Done | — | PEST parser, reverse index (15.5k patterns), 7 tables extracted |
| poe-data | Done | — | `GameData` with indexed tables, FK resolution, domain.rs constants |
| poe-item | Done | 98 | PEST grammar + resolver, 68 fixtures, local/non-local stat_ids |
| poe-eval | Foundation | 45 | Predicates, rules, scoring profiles, tier analysis, `evaluate_item()` |
| poe-trade | Phase 3 | 33 | Stats index (87.4% match), query builder, rate-limited HTTP, price check |
| app | Phase 8e | — | Tauri v2 overlay — all phases through 8e done |

#### App — What's Complete

- **Overlay**: Full PoE-native tooltip — tier badges, roll quality bars, affix summary, influences, scoring, watching indicators
- **Settings**: 4 sections (General, Hotkeys, Profiles, Trade) with persistence
- **Profile system**: Create/edit/duplicate/delete/import/export, compound scoring rules (AND/OR/nested), drag-and-drop reorder, watching profiles with click-to-swap
- **Trade**: Price check, Edit Search (per-stat min override, socket/quality/type scope filters), Open Trade URL, rate limiting, POESESSID, stats index caching, league selection
- **Cross-platform**: Windows fully working, Linux (Wayland layer-shell + X11 fallback), macOS placeholder (cursor position hardcoded)

#### App — Partially Done

| Feature | What's Missing |
|---------|---------------|
| Phase 8c (rule editor UX) | Collapsible groups, depth-colored borders, count-of-N combinator, progressive disclosure, reusable condition templates |
| macOS support | Core Graphics cursor position (currently hardcoded `(100, 100)`) |

#### App — Not Started (Future Features)

| Feature | Description | Blocked By |
|---------|-------------|------------|
| Pseudo stats | Sum matching stat lines (e.g., "pseudo max life ≥ 140") | New poe-eval predicate type |
| Compact overlay mode | Score-only pill for speed-scanning stash tabs | App-only (CSS + hotkey) |
| Stash tab scrolling | Mouse wheel to cycle stash tabs | App-only (research AWP approach) |
| Chat macros | Custom hotkeys for chat commands | App-only (enigo) |
| Map danger assessment | Per-mod danger tagging (deadly / warning / good) with traffic-light overlay. User classifies each map mod per profile — no hardcoded danger list since riskiness is build-dependent. Click-to-cycle in overlay, full searchable mod list in settings. Dedicated hotkey and/or shown in normal overlay when item class is Map. Reference: `_reference/awakened-poe-trade/renderer/src/web/map-check/` | App UX + settings UI; poe-eval profiles already support the evaluation; needs area mod stat list from poe-dat |
| Character-aware profile switching | Parse PoE `client.txt` live to detect character login events, auto-switch active + watching profiles (including map profiles) per character. Enables "set once, forget" workflow where each character has its own danger/eval config. | File watching (notify crate or Tauri fs-watch), client.txt format research, profile-to-character binding UI |
| Rule text DSL | Textual rule format compilable to Profile JSON | Grammar design, VS Code ext |
| CSS split | Separate entry points for overlay vs settings | Low priority, class-scoping works |
| Craft suggestions | Deterministic craft advice from open affixes | `CraftingBenchOptions` table in poe-data |

#### Crate-Level Gaps

| Gap | Crate | Impact |
|-----|-------|--------|
| `(Local)` suffix in trade stat text | poe-trade | Trade queries for local stats may not match correctly |
| 12.6% unmapped trade stats | poe-dat → poe-trade | Stats from unparsed description files (atlas, sanctum, heist) |
| `CraftingBenchOptions` table | poe-data | Blocks craft suggestion feature |
| Multi-line stat lookups | poe-item | Some stats span two lines in item text, not resolved |
| Ctrl+C fallback parser | poe-item | Only Ctrl+Alt+C format supported, Ctrl+C has less data |
| `{ Foulborn Unique Modifier }` | poe-item | Mod name before "Unique" keyword — grammar doesn't handle this header pattern |

### Suggested Next Steps (Pick Any)

1. **Pseudo stats** — High value for profile usability. Add `PseudoStat` predicate to poe-eval that sums matching stat values across all mods.
2. **Compact overlay** — Quick win, app-only. Score pill + expand hotkey.
3. **Phase 8c UX polish** — Collapsible rule groups, better visual hierarchy for compound rules.
4. **Parse more stat description files** — Bumps trade match rate above 87.4%. Files: atlas, map, sanctum, heist.
5. **`(Local)` trade suffix** — Ensure local stat_ids get the `(Local)` suffix when building trade queries.
6. **Craft suggestions** — Extract `CraftingBenchOptions` from GGPK, surface "you can bench-craft X" in overlay.
7. **Map danger assessment** — High value for HC/mapping safety. Per-mod deadly/warning/good tagging using existing profile+eval system. Dedicated hotkey or auto-detect map item class. UX reference: awakened-poe-trade's map-check (click-to-cycle, 3-profile slots, searchable settings list).
8. **Character-aware profile switching** — Parse `client.txt` to detect character logins, auto-switch active profiles (eval + map danger). Natural companion to map danger — makes per-character danger config seamless.

### Recent Git History (for context)

```
5711a98 Domain-based mod resolution for correct local/non-local stat_ids
a3ddad9 Add SocketCount, LinkCount, Quality predicates to poe-eval + fix biome
d47d53e Add quality filter to trade queries with Edit Search UI control
c8c1c79 Strip "Superior" quality prefix from base type names in poe-item
33bcaf8 Handle "— Unscalable Value" suffix in poe-item resolver
5f9c563 Add socket/link filter support to trade queries and Edit Search UI
4068ae9 Update domain-knowledge-reviewer agent for poe-trade boundary
62e4402 Fractured mod support, stat ID power-user toggle, drag-handle-only reorder
59503b7 Phase 4: stat_id → stat_ids for local/non-local equivalence
```
