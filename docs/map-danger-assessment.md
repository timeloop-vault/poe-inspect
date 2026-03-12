# Map Danger Assessment

Design notes for the map danger evaluation feature.

## Core Concept

When inspecting a map, show each mod with a **danger classification** — deadly (red/skull), warning (orange/triangle), good (green/check), or unclassified. The user defines what's dangerous per profile, because riskiness is build-dependent (reflect kills elemental builds, no-leech kills leech builds, etc.).

This is **not a new evaluation engine**. It reuses poe-eval's existing profile and predicate system with a different UX treatment.

## UX Model

### Overlay

- Trigger: dedicated "Map Inspect" hotkey, and/or auto-detect when inspected item class is Map
- Each map mod line is colored by its danger classification:
  - **Deadly** (red, skull icon) — mod that will kill this build
  - **Warning** (orange, triangle icon) — mod that makes the map harder/rippier
  - **Good** (green, check icon) — mod the build benefits from or doesn't care about
  - **Unclassified** (neutral) — user hasn't tagged this mod yet
- Click-to-cycle on each mod: deadly → warning → good → seen → unset (same as awakened-poe-trade)
- Overall map verdict at a glance: if any mod is deadly → skull header; if warnings but no deadly → caution header; all good → safe header

### Settings Page

- Full searchable list of all area/map mod stats (sourced from poe-dat stat descriptions)
- Per-mod radio buttons: deadly / warning / good (per profile)
- Filter: "only show selected" toggle, search box
- "Show new mods" toggle — highlights mods you haven't classified yet (important on league launch when new mods appear)

### Profiles

- Map danger classifications are stored **per profile**, not globally
- Each profile can have different danger tags for the same mod
- This ties into the future **character-aware profile switching** — each character auto-loads its own map danger config

## Architecture

### What Already Exists

| Component | Status | Notes |
|-----------|--------|-------|
| Map item parsing | Done | poe-item parses maps fully — 11 fixtures, mods resolved with stat_ids |
| Map properties | Done | Map Tier, Item Quantity, Item Rarity, Monster Pack Size all parsed |
| poe-eval predicates | Done | `StatValue`, `ItemClass`, `Rarity`, `ModCount` etc. all work on maps |
| Profile system | Done | Create/edit/duplicate/delete, compound rules, import/export |
| Overlay rendering | Done | Tooltip with tier badges, affix lines, scoring |

### What Needs Building

| Component | Crate/Layer | Work |
|-----------|-------------|------|
| Area mod stat list | poe-dat | Extract all area mod stat descriptions (some already in reverse index, may need map/atlas description files) |
| Map danger profile type | app (TS) | New profile variant or section: per-stat danger classification storage (`{ matcher: string, decision: string }`) |
| Map overlay component | app (TS) | New Vue component: colored mod list, click-to-cycle, verdict header |
| Map settings page | app (TS) | Searchable stat list with per-profile radio buttons |
| Map inspect hotkey | app (Rust+TS) | Dedicated hotkey binding, or auto-detect map item class on normal inspect |
| MapTier predicate | poe-eval | Optional — allows filtering/scoring by map tier |

### Data Flow

```
User presses Map Inspect hotkey
    → Ctrl+Alt+C clipboard capture
    → poe-item parses map (already works)
    → Resolved mods matched against user's danger classifications for active profile
    → Overlay renders colored mod list + verdict
    → User can click-to-cycle to update classifications inline
```

## Reference Implementation

Awakened PoE Trade's map-check: `_reference/awakened-poe-trade/renderer/src/web/map-check/`

Key files:
- `common.ts` — `MapCheckConfig`, `MapCheckStat`, decision cycling logic (`d`/`w`/`g`/`s`/`-`)
- `MapCheck.vue` — Overlay component, profile switcher (3 slots: I/II/III)
- `MapStatButton.vue` — Click-to-cycle button per mod, colored by danger level
- `settings-maps.vue` — Settings page with virtual-scrolled stat list, search, filter
- `SettingsMatcherEntry.vue` — Per-stat row with radio buttons (warning/deadly/good)

Their storage model: each stat's decision is a 3-char string like `"dw-"` — one char per profile slot (deadly in profile 1, warning in profile 2, unset in profile 3).

## Companion Feature: Character-Aware Profile Switching

Map danger is most useful when it auto-switches per character. Design:

1. **client.txt watching** — PoE writes login events to `client.txt` in the game directory. Parse lines like `Connecting to instance server...` and character name from `Entering area...` events
2. **Profile binding** — Settings UI to bind profiles (eval + map danger) to character names
3. **Auto-switch** — On character login detection, activate the bound profiles
4. **Implementation** — Tauri fs-watch or `notify` crate for file watching; tail `client.txt` for new lines

This is a separate feature but architecturally related — the profile system needs a "bound to character" concept.

## Design Decisions

- **No hardcoded danger lists** — The app never decides what's dangerous. All classification is user-driven, per profile, per build.
- **Reuse poe-eval** — Map danger profiles are conceptually the same as item eval profiles, just with a simpler UX (tag per mod vs. compound rules). Whether they literally use the same profile format or a simplified one is a build-time decision.
- **Click-to-cycle in overlay** — Critical for discoverability. First time you inspect a map, mods are unclassified. You click each one to teach the system. After that, it remembers.
- **"New mod" detection** — On league launch, GGG adds new map mods. The system should highlight mods you haven't classified yet so you can evaluate them before running.
