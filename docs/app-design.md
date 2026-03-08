# App Design — Tauri v2 Overlay

> Plan for the UI/overlay layer. Independent of backend pipeline work.

## Tech Stack

- **Tauri v2** — Rust backend shell + web frontend
- **Preact** (TypeScript, strict mode) — frontend framework (~3KB vs React's ~40KB, critical for game overlay footprint)
- **Vite** — build tool + dev server with HMR (uses Rollup under the hood for production builds; Tauri's standard scaffolding)
- **Biome** — linting + formatting (Rust-based, fast, single tool for both)
- **CSS Modules or Tailwind** — styling (TBD during Phase 2)
- **enigo** — cross-platform keystroke sending (Rust side)

## Phases

### Phase 1: Tauri Prototype Validation

Validate the 7-point checklist before building anything else. If any of these fail on Windows, we fall back to egui.

| # | Requirement | How to test |
|---|------------|-------------|
| 1 | **Global hotkey while PoE is focused** | Register `Ctrl+I` via `tauri-plugin-global-shortcut`. Open PoE, press hotkey, confirm callback fires. |
| 2 | **Transparent overlay window** | Create undecorated window with `transparent: true`, HTML background transparent. Confirm game is visible behind it. |
| 3 | **Click-through behavior** | Set `ignore_cursor_events(true)`. Confirm clicks pass through to PoE. Toggle off — confirm overlay becomes interactive. |
| 4 | **Always-on-top over PoE** | Set `always_on_top: true`. Confirm overlay renders above PoE in borderless windowed mode. |
| 5 | **Cursor-relative positioning** | On hotkey press, get cursor screen position, place overlay window there. Confirm it appears near the hovered item. |
| 6 | **Focus behavior** | Show overlay without stealing focus from PoE. Player should still be able to move/click in-game while overlay is visible. |
| 7 | **Keystroke sending** | From Tauri command, send `Ctrl+Alt+C` to PoE via `enigo`. Confirm item text lands on clipboard. Read it back. |

**Deliverable:** Minimal Tauri app that, on hotkey press, sends Ctrl+Alt+C to PoE, reads clipboard, and shows the raw text in a transparent overlay near the cursor. No parsing, no styling — just plumbing.

### Phase 2: Overlay UI with Mock Data

Build the item display overlay using hardcoded mock items. No backend dependency.

#### Visual Direction: PoE-Native + Augmented

The overlay should look like a PoE item tooltip — familiar to players, zero learning curve. We augment it with analysis that the game doesn't provide.

**Base layer (PoE-native feel):**
- Dark semi-transparent panel (similar to in-game tooltip background)
- Item name header colored by rarity (white/blue/yellow/orange/brown)
- Item art from PoE CDN (`web.poecdn.com/image/Art/2DItems/...`)
- Base type, requirements, item level — styled like in-game
- Separator lines between sections (mimicking the `--------` separators)

**Augmented layer (our value-add):**
- **Affix tier coloring** — each mod line color-coded by tier:
  - T1: bright highlight (green or gold)
  - T2-T3: decent (blue-ish or white)
  - T4+: dim/muted (gray/red)
  - Colors and thresholds user-configurable later
- **Tier badge** — small `T1`, `T2` etc. label next to each mod
- **Roll quality** — how good the roll is within its tier range (e.g., 89/80-99 = 89th percentile). Could be a mini bar or percentage.
- **Prefix/Suffix indicators** — `[P]` / `[S]` badge on each mod, or a visual prefix/suffix grouping
- **Open affix slots** — "1 open prefix, 1 open suffix" shown clearly
- **Score/summary bar** — overall item quality score at top or bottom (when profiles are active)

**Layout sketch:**

```
+------------------------------------------+
|  [Item Art]   Brood Thirst               |  <- rarity-colored name
|               Vaal Regalia               |  <- base type
|------------------------------------------|
|  +88 to maximum Life            T1 [P]   |  <- green, high tier
|  +43% Fire Resistance           T2 [S]   |  <- blue-ish
|  +31% Cold Resistance           T3 [S]   |  <- white
|  +12% increased Spell Damage    T5 [P]   |  <- dim/gray
|  +22 to Intelligence            T4 [S]   |  <- dim
|------------------------------------------|
|  Prefixes: 2/3    Suffixes: 3/3          |
|  iLvl 84    6-Link                       |
|------------------------------------------|
|  [Craft suggestion: bench +mana]         |  <- future, Phase 3+
+------------------------------------------+
```

**Mock data:** Create 3-4 representative items covering different cases:
1. Well-rolled rare armor (the common case)
2. Unique item (simpler display — no tier analysis)
3. Weapon with DPS-relevant mods
4. Jewel (compact layout, different mod pool)

**Deliverable:** Polished overlay component rendering mock items with full tier visualization. Looks like a PoE tooltip but better.

### Phase 3: Settings & Profile UI

Separate from the overlay — a proper windowed UI for configuration.

**Settings:**
- Hotkey configuration
- Tier color scheme customization
- Display preferences (what to show/hide)
- PoE version selection (PoE1/PoE2)

**Profile management:**
- Create/edit/delete evaluation profiles
- Per-mod weight sliders (e.g., "+Life: High", "+Fire Res: Medium")
- Searchable mod list with autocomplete
- Import/export profiles as JSON
- Per-character profile binding

**This phase can use mock data too** — the profile editor doesn't need real game data to build. It just needs a list of mod names (which we can hardcode from known data).

## Item Art from PoE CDN

PoE's 2D item art is available at predictable CDN URLs:

```
https://web.poecdn.com/image/Art/2DItems/Armours/BodyArmours/BodyStr1.png
https://web.poecdn.com/image/Art/2DItems/Weapons/TwoHandWeapons/Bows/Bow1.png
```

The art path for each base type is stored in `BaseItemTypes.dat` (field: `InheritsFrom` → visual identity). Once poe-dat extracts BaseItemTypes, we can map base type name → art URL. For mock data, we'll hardcode a few known URLs.

Note: Unique items have their own art. Item art URLs can also be obtained from the official trade API responses and poe.ninja data.

## Architecture Notes

**Overlay window:** Created hidden on app start. On hotkey → position near cursor → populate with data → show. Dismiss on Escape, click-away, or timer.

**Settings window:** Separate Tauri window (not the overlay). Standard windowed UI. Opened from system tray icon.

**IPC flow (Phase 1):**
```
Hotkey fires (Rust)
  → Send Ctrl+Alt+C via enigo (Rust)
  → Wait ~100ms
  → Read clipboard (Rust)
  → Emit event to frontend with raw text
  → Frontend displays in overlay
```

**IPC flow (future, with backend):**
```
Hotkey fires (Rust)
  → Send Ctrl+Alt+C via enigo (Rust)
  → Wait ~100ms
  → Read clipboard (Rust)
  → Parse item text via poe-item (Rust)
  → Evaluate via poe-eval (Rust)
  → Return structured EvaluatedItem to frontend
  → Frontend renders tier-colored overlay
```

## Dependencies (npm / cargo)

**Frontend (npm):**
- preact
- @preact/preset-vite (Vite plugin for Preact)
- @tauri-apps/api (v2)
- @tauri-apps/plugin-global-shortcut
- @tauri-apps/plugin-clipboard-manager
- typescript, vite
- @biomejs/biome (linting + formatting)

**Backend (cargo):**
- tauri (v2)
- tauri-plugin-global-shortcut
- tauri-plugin-clipboard-manager
- enigo (keystroke sending)
- serde, serde_json (IPC serialization)

## Confirmed Decisions

- **Dismiss:** Escape + click-away + next hotkey press all dismiss the overlay.
- **System tray:** Yes, include in Phase 1 scaffold. Shows app is running, provides settings access.
- **Animations:** Fast/snappy, no animations for now. Can add flare later.
- **Theming:** Dark-only to match PoE aesthetic.
- **Cross-platform:** Windows primary, Linux (XWayland) and macOS secondary. All Rust-side code (hotkeys via `tauri-plugin-global-shortcut`, keystrokes via `enigo`) is cross-platform.

## Open Questions

- **Multiple items:** Show one overlay at a time, or allow pinning/stacking? Start with one, consider pinning later.
- **CSS approach:** CSS Modules vs Tailwind — decide during Phase 2 when building actual components.
