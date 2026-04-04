# Roadmap

Current priorities, ordered. Updated 2026-03-15.

---

## Done

### ~~1. Release Flow (GitHub Actions)~~ ✅

Multi-platform CI in `.github/workflows/release.yml` — triggered on GitHub Release publish or manual dispatch. Builds Windows (.exe/.msi), macOS (.dmg), Linux (.deb/.AppImage) with signing support. Manual dispatch uploads as workflow artifacts.

### ~~3. Compact Overlay Mode~~ ✅

Score-only pill for speed-scanning stash tabs.

- Compact inspect hotkey (default `Ctrl+Shift+I`) — small pill near cursor, click-through, auto-dismisses after 2.5s
- Shows item name + score % (color-coded) + watching profile dots + demand badge
- Maps show DEADLY/CAUTION/SAFE/UNRATED verdict instead of score
- Press full inspect hotkey while pill showing → expands to full overlay (no re-parse)
- Independent compact position setting (cursor vs panel)
- DOM-measured panel positioning replaces hardcoded size estimates
- "Not bound" placeholder UX for unset hotkeys

### ~~7. Stash Tab Scrolling & Chat Macros~~ ✅

Both features implemented with Settings UI.

- **Stash Tab Scrolling**: `WH_MOUSE_LL` hook, configurable modifier key, stash area geofencing
- **Chat Macros**: Hotkey-bound chat commands, clipboard-based injection, per-macro send toggle, conflict detection
- **PoE Focus Gate**: All gameplay hotkeys gated on PoE foreground window check, toggleable in Settings

### ~~10. Hotkeys Swallow Keys Outside PoE~~ ✅

`WH_KEYBOARD_LL` hook (`hotkey_hook.rs`) replaces `tauri-plugin-global-shortcut` for gameplay hotkeys on Windows. Keys pass through to other apps when PoE isn't foreground. Settings hotkey (`Ctrl+Shift+S`) remains globally registered. Toggleable via "Require PoE Focus" setting.

### ~~Local Stat ID Resolution (Phases 1–4)~~ ✅

Base-type-anchored stat_id resolution. Items, suggestions, and evaluation all carry real (possibly local) stat_ids from the GGPK Mods table. Domain filtering for jewels vs equipment. Phase 5 (trade `(Local)` suffix) is roadmap item #4 below.

### ~~Multi-line Stat Lookups~~ ✅

`try_multi_line_resolution()` joins consecutive unresolved stat lines with `\n` for reverse index lookup. Handles stats spanning two visual lines (e.g., flask immunity mods).

### ~~RQE Marketplace~~ ✅

Full pipeline: decision DAG engine, domain-free server, client crate, app integration (login gate, query CRUD, query builder with edit, demand badge in overlay + compact pill, styled tooltip, badge color picker, auto-refresh on tab revisit). Only GGG OAuth remains (long-term, needs approval).

---

### ~~`(Local)` Trade Stat Suffix~~ ✅

Trade stats index maps local GGPK stat_ids to `(Local)` trade numbers via `GameData.all_stat_ids_for_template()` + `is_local` filter. Items with local stat_ids from base-type-anchored resolution now produce correct trade queries.

### ~~Parse More Stat Description Files~~ ✅

Added atlas, sanctum relic, heist equipment, and expedition relic stat descriptions. Reverse index: 15,500 → 17,624 patterns (+13.7%). Trade match rate: 87.4% → 94.8%. Remaining 603 unmatched are niche (grafts, wombgifts, legacy, Forbidden).

### ~~HasStatText Deprecation~~ ✅

Removed both `HasStatText` and `HasStatId` predicate variants. All evaluation logic and tests migrated to `StatValue` with proper stat_ids.

---

## Active Priorities

### 2. Trade Edit Search Improvements

Pain points from real gameplay. The Edit Search UI needs more filters to match what the official trade site offers. Many of these were designed in `docs/trade-query-builder-design.md` (Phase 6c) but not yet implemented.

1. **Rarity toggle** — filter by rarity. Designed in query builder doc §7 (`rarity: "nonunique"` / omit). Small toggle near header
2. **Level/attribute requirements** — level req, str/dex/int req, item level. Designed in §2-3 (`ilvl.min` filter). Attr reqs display-only for now (no trade API filter)
3. **Defenses & sockets** — quality, armour/evasion/energy shield, socket/link filters. Quality + sockets partially implemented already
4. **Open prefixes** — filter for items with open prefix slots. Designed in §4 (`pseudo.open_prefix`/`pseudo.open_suffix`). Needs trade API pseudo stat ID research
5. **"Base item" button** — strips all stat filters but keeps base type + fractured mods. Quick way to price-check the base itself
6. **Fractured affix support** — fractured mods should be recognized and filterable in Edit Search. Partially designed in §8 (auto-detected, make toggleable)

---

### 3. Map Danger Escalation

User-configurable threshold where accumulating lower-tier dangers escalates to a higher tier.

- E.g., 3+ caution mods → treat as deadly (or "caution++")
- Configurable per-profile: number of mods at tier X that escalate to tier Y
- Affects both full overlay verdict and compact pill display
- Settings UI in Map Danger section

---

### 4. Auto-Updater

App should check for updates and offer to install them. CI infrastructure is already in place (`release.yml` generates signed `latest.json`). Design doc: `docs/auto-updater.md`.

**What's done:** tauri.conf.json config (pubkey + endpoint), release workflow (signing + `includeUpdaterJson`), design doc.

**What's needed:**
1. Add `tauri-plugin-updater` (Cargo.toml + package.json)
2. Initialize plugin in `lib.rs`
3. Add updater capability permissions
4. Startup: silent check → toast if update available
5. Settings > General: "Check for Updates" button
6. Update dialog: version + release notes, Install / Later
7. Schema versioning for profile migration (`schemaVersion` field)

---

### 5. Data Extraction / Update Flow (GitHub Actions)

Automated pipeline to download GGPK from the PoE patch server, extract game data, and regenerate derived files.

**What it does:**
1. Download GGPK (or diff patches) from GGG's patch server
2. Track the last extracted version (store hash/version in repo or artifact)
3. Run extraction: poe-bundle → poe-dat → poe-data generation
4. Regenerate: stat description reverse index, typed tables, any derived files
5. Optionally identify new/changed tables or fields we don't handle yet
6. Commit updated data files (or open a PR)
7. `--force` flag to re-extract even if version hasn't changed

**Open questions:**
- How does the GGPK download work? (patch server protocol, authentication, delta patches vs full download)
- Where to cache the GGPK between runs? (GitHub Actions artifact, external storage)
- Which generated files are committed vs built at app compile time?
- Can we detect new stat description patterns or new dat table fields automatically?

---

### 6. Toast Position Config

Let user choose toast screen position (top/bottom, left/center/right). Currently hardcoded to top-center.

- Add `toastPosition` to `GeneralSettings` (e.g., "top-left", "top-center", "top-right", "bottom-left", "bottom-center", "bottom-right")
- Settings UI: radio button group or dropdown in General section
- Parameterize position calculation in `show_toast()`

---

## Backlog (Unordered)

Known gaps and future features, not currently prioritized.

| Item | Layer | Notes |
|------|-------|-------|
| Reference image overlays | app | User-loaded cheat sheet images (syndicate, atlas, shipping, etc.) as pinnable always-on-top overlays. Hotkey to show/hide, temporary or pinned mode, settings UI to add/remove images, edit mode for positioning/resizing on screen. |
| Pseudo stats | poe-eval | Sum matching stat lines (e.g., "pseudo total life >= 140"). New `PseudoStatValue` predicate. |
| Anointment trade stats | poe-trade | Option-based stats (`Allocates #` with dropdown). Needs `StatFilter.option` field. |
| Memory Strands | poe-data/item/eval | New Mirage 3.28 mechanic. Surface as first-class field, factor into evaluation. |
| Count-of-N combinator | poe-eval + app | `Rule::Count(n, Vec<Rule>)` — "at least N of these conditions". |
| Reusable condition templates | app | Save/insert condition templates across rules and profiles. |
| Craft suggestions | poe-data + app | Extract `CraftingBenchOptions` from GGPK, show "bench-craft X" in overlay. |
| Rule text DSL | poe-eval + app | Text format compilable to Profile JSON. VS Code extension. |
| Ctrl+C fallback parser | poe-item | Only Ctrl+Alt+C supported; Ctrl+C has less data but is more common. |
| `{ Foulborn Unique Modifier }` | poe-item | Grammar doesn't handle mod name before "Unique" keyword. |
| macOS cursor position | app | Currently hardcoded `(100, 100)`. Needs Core Graphics API. |
| CSS split (overlay vs settings) | app | Separate entry points. Low priority — class-scoping works. |
| Overlay sprites | app | Foil/Quest/Prophecy headers, Div Card separator, influence overlays. |
| Item browser for rule building | app | Browsable poe-data database for looking up base types, mods. |
| Profile/rules separation | app | Rethink profile identity vs role (primary/watching/off) — see memory. |
| GGG OAuth for RQE | app + rqe-server | Replace mock auth with real PoE account OAuth. Needs community mass for approval. |
| DPS calculations | poe-eval | Use `is_local` from Stats table for weapon/armour base value calculations. |
