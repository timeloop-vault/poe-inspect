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

**Status: DONE.** Completed items:
- PoE tooltip art extracted from GGPK sprite atlases (headers + separators per rarity)
- Game fonts extracted (FrizQuadrata for item names, Fontin SmallCaps/Regular/Bold/Italic for body)
- Exact rarity colors from `ItemFrameType.datc64`
- Tier coloring, tier badges, prefix/suffix badges, roll quality bars, open affix counts
- 3 mock items: rare boots (mixed tiers, dual influence), rare body armour (enchant, open suffix), unique ring (Ventor's Gamble)
- Remaining nice-to-haves: weapon/jewel mock items, 2D item art display

### Phase 3: Settings & Profile UI

Separate window from the overlay — a proper windowed UI for configuration.

#### How to Open

- **Right-click system tray → "Settings"** (primary)
- **Hotkey:** `Ctrl+Shift+I` (complement to `Ctrl+I` inspect)
- Tray menu: `Settings` | `---` | `Quit`

#### Window Layout

Left sidebar navigation + content area. Sidebar is always visible, shows which section you're in. Scales better than tabs as sections grow.

```
+-------+------------------------------------------+
| NAV   |  CONTENT                                 |
|       |                                          |
| ● Gen |  Overlay Scale                           |
|   Hot |  [====|=========] 100%                   |
|   Pro |                                          |
|       |  PoE Version                             |
|       |  (●) PoE 1   ( ) PoE 2                  |
|       |                                          |
|       |  Startup                                 |
|       |  [x] Start minimized to tray             |
|       |  [x] Launch on system startup            |
|       |                                          |
|       |  Display                                 |
|       |  [x] Show roll quality bars              |
|       |  [x] Show tier badges                    |
|       |  [x] Show prefix/suffix labels           |
|       |  [x] Show open affix count               |
+-------+------------------------------------------+
```

#### Sections

**General**
- Overlay scale/zoom (slider, critical for different monitor sizes/DPIs)
- PoE version toggle (PoE1 / PoE2 — affects data pipeline)
- Startup behavior (start minimized, launch on boot)
- Display toggles (which overlay elements to show/hide)

**Hotkeys**
- Inspect item: `Ctrl+I` (configurable)
- Dismiss overlay: `Escape` (configurable)
- Open settings: `Ctrl+Shift+I` (configurable)
- Each row: action name + key capture input

```
+-------+------------------------------------------+
| NAV   |  CONTENT                                 |
|       |                                          |
|   Gen |  Hotkeys                                 |
| ● Hot |                                          |
|   Pro |  Inspect Item      [ Ctrl+I         ] ⟲  |
|       |  Dismiss Overlay   [ Escape         ] ⟲  |
|       |  Open Settings     [ Ctrl+Shift+I   ] ⟲  |
|       |                                          |
+-------+------------------------------------------+
```

**Profiles**
- List of saved profiles with active indicator
- Create / duplicate / delete / import / export buttons
- Clicking a profile opens the **Profile Editor** (inline or sub-view)

```
+-------+------------------------------------------+
| NAV   |  CONTENT                                 |
|       |                                          |
|   Gen |  Profiles                                |
|   Hot |                                          |
| ● Pro |  [+ New]  [Import]                       |
|       |                                          |
|       |  ★ RF Juggernaut        [Edit] [⋯]      |
|       |    Mapper (generic)     [Edit] [⋯]      |
|       |    Crafter (prefixes)   [Edit] [⋯]      |
|       |                                          |
|       |  ★ = active profile                      |
|       |  [⋯] = duplicate, export, delete         |
+-------+------------------------------------------+
```

#### Profile Editor

Opens when clicking [Edit] on a profile. Two sub-tabs within the editor:

**Mod Weights** — what matters for this build
- Searchable/filterable mod list
- Each mod gets a weight: Ignore / Low / Medium / High / Critical
- Weight affects the overlay score and could influence tier color intensity
- Group by category (Life, Resistances, Damage, Speed, etc.)

```
+--------------------------------------------------+
|  ← Back to Profiles    "RF Juggernaut"           |
|                                                  |
|  [Mod Weights]  [Display]                        |
|                                                  |
|  Search: [fire res____________]                  |
|                                                  |
|  Life & Defence                                  |
|    +# to maximum Life          [■■■■□] High      |
|    +#% to Armour               [■■■□□] Medium    |
|    +# to maximum Energy Shield [■□□□□] Low       |
|                                                  |
|  Resistances                                     |
|    +#% to Fire Resistance      [■■■■■] Critical  |
|    +#% to Cold Resistance      [■■■□□] Medium    |
|    +#% to Lightning Resistance [■■□□□] Low       |
|                                                  |
|  Speed                                           |
|    #% increased Movement Speed [■■■■□] High      |
+--------------------------------------------------+
```

**Display** — how this profile renders the overlay
- Tier color scheme (which colors for T1/T2-3/T4-5/low)
- Color pickers for each tier level
- Preview of how a mod line looks with current colors
- Option to highlight mods that match profile weights

```
+--------------------------------------------------+
|  ← Back to Profiles    "RF Juggernaut"           |
|                                                  |
|  [Mod Weights]  [Display]                        |
|                                                  |
|  Tier Colors                                     |
|    T1 (best)    [■] #38d838  ← color picker      |
|    T2-T3        [■] #5c98cf                      |
|    T4-T5        [■] #c8c0b0                      |
|    T6+  (low)   [■] #8c7060                      |
|                                                  |
|  Preview                                         |
|  ┌──────────────────────────────────────┐        |
|  │ T1 P  +88 to maximum Life    ██ 95% │        |
|  │ T3 S  +31% Cold Resistance   █░ 50% │        |
|  │ T5 P  +12% Spell Damage      ▪░ 20% │        |
|  └──────────────────────────────────────┘        |
|                                                  |
|  [x] Highlight mods matching profile weights     |
|  [x] Dim mods with weight = Ignore               |
+--------------------------------------------------+
```

#### Technical Notes

- Settings window is a **separate Tauri window** (label: `settings`), not the overlay
- Standard window: decorations, resizable, not always-on-top, not transparent
- Settings persist to a JSON file (via `tauri-plugin-store` or manual serde)
- Profile data: JSON files in app data dir, one per profile
- Mod list for the weight editor: hardcoded initially, later from `poe-data`
- Settings changes apply immediately (no save button — live preview)

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

**Settings window:** Separate Tauri window (label: `settings`, not the overlay). Standard decorated window, resizable. Opened from tray right-click → "Settings" or `Ctrl+Shift+I`. Left sidebar nav with General / Hotkeys / Profiles sections.

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

### Phase 4: App Wiring (no backend needed)

Wire existing settings to actual behavior. All tasks are independent of the data pipeline (poe-item/poe-eval) and can be done with mock data.

| # | Task | Status | Notes |
|---|------|--------|-------|
| 1 | **Overlay scaling** | Done | `transform: scale()` on panel element. Fullscreen window, no resize needed. |
| 2 | **Dynamic hotkey wiring** | Done | `pause_hotkeys`/`resume_hotkeys`/`update_hotkeys` Tauri commands. Only modifier combos registered globally; dismiss is window-level. |
| 3 | **Display toggles → overlay** | Done | `DisplaySettings` passed from App → ItemOverlay. Settings reloaded on each inspect and debug overlay show. |
| 4 | **Start minimized / launch on boot** | Done | `startMinimized` read from store in Rust setup. `tauri-plugin-autostart` for launch on boot (cross-platform). UI scale for settings window. |
| 5 | **Profile import/export** | Done | JSON file save/load via `tauri-plugin-dialog` + `tauri-plugin-fs`. Export strips id/active; import assigns new id. |
| 6 | **Overlay positioning** | Done | Fullscreen overlay, CSS absolute positioning. Cursor mode: offset + flip on overflow. Panel mode: right-anchored beside inventory, left-anchored beside stash. |

### Phase 5: poe-eval Integration & Profile UI

Wire poe-eval's evaluation capabilities into the app. Profiles become real
(backed by poe-eval's Predicate/Rule/Profile types), scoring works end-to-end,
and the profile builder UI is driven by a schema from poe-eval.

**See `docs/phase-5-eval-integration.md` for the detailed step-by-step plan.**

**Status: DONE** — all 7 steps complete (display → scoring → schema → profiles → builder → weights → overlay).

### Phase 6: Stat ID Matching

Replace substring-based stat matching (`HasStatText`) with proper stat ID
matching (`HasStatId`). Mod weights store internal stat IDs resolved from
the reverse index, making matching language-independent and unambiguous.

**See `docs/phase-6-stat-id-matching.md` for the detailed step-by-step plan.**

**Status: DONE**

### Phase 7: Fullscreen Transparent Overlay

Replace the current resizable overlay window with a single fullscreen transparent
window. The item panel is positioned within the fullscreen canvas using CSS, eliminating
all window-resize complexity.

**Status: DONE**

**What was implemented:**
- Overlay window expands to fill the monitor containing the cursor (`setup_fullscreen_overlay`)
- Backend captures cursor position (physical pixels), converts to CSS pixels via monitor scale factor
- Frontend receives cursor position via `overlay-position` event, positions panel with CSS `position: absolute`
- Fullscreen `overlay-backdrop` (fixed, inset 0) catches clicks outside the panel to dismiss
- Panel mode: right-anchored (CSS `right`) when beside inventory, left-anchored beside stash
- PoE panel width = `screen_height * 986/1600` (from PoE's UI layout, do not change unless GGG changes in-game)
- Overlay scale uses `transform: scale()` (not CSS `zoom`) to avoid layout interference
- Removed `useAutoResize`, `reposition_overlay`, `position_overlay`, `clamp_to_monitor`
- Removed `dismissOnFocusLoss` setting — backdrop click handles this inherently
- Wayland layer-shell: anchors all 4 edges for fullscreen coverage

### Phase 8: Compound Scoring Rules

Upgrade the scoring system from independent single-predicate lines to compound
rules (AND/OR groups). This lets users express "I want a wand with ilvl 83+ AND
+1 fire spell skills" as a single scored entry instead of four unrelated lines.

**Problem:** Currently each scoring rule is a single predicate evaluated independently.
"Has max life" scores +20 whether it's on a wand or a belt. Users can't express
"this combination of properties on a specific item type" — which is the most common
real-world use case (build-specific gear shopping).

**The rule engine already supports this** — `Rule::All` and `Rule::Any` exist.
The gap is the UI: the profile editor only creates `Rule::Pred` scoring entries.

#### Phase 8a: Compound Rule UI

**Status: DONE (not tested end-to-end)**

Allow scoring entries to use `Rule::All` / `Rule::Any` instead of just `Rule::Pred`.
The UI shows these as expandable groups:

```
[Wand + ilvl 83+ + fire spell skills]  weight: 50
  ├─ ItemClass = "Wands"
  ├─ ItemLevel >= 83
  └─ HasStatId = "fire_spell_skill_gem_level_+"

[Life + open prefix]                   weight: 30
  ├─ HasStatId = "base_maximum_life"
  └─ OpenMods Prefix >= 1
```

One weight per group, all conditions must match. Single-predicate rules still work
as before (they're just a group of one). The profile editor gets:
- "Add rule" → single predicate (current behavior)
- "Add rule group" → creates an `All` container, user adds predicates into it
- Toggle between Simple / All (AND) / Any (OR) on each group
- Mode selector in expanded body, header stays clean

The mod weight quick-add still works: adding "+# to maximum Life" creates a simple
single-predicate rule. But the scoring rules tab now shows and edits compound rules.

**What was implemented:**
- Type guards `isCompoundRule()` / `isPredRule()` in `types.ts`
- `defaultCompoundRule()` factory in `PredicateEditor.tsx`
- `PredicateRow` extracted as reusable component (type selector + fields + optional delete)
- `ScoringRuleEditor` refactored for Simple/All/Any modes with state transitions
- "+ Add Group" button in `CustomProfileView`
- CSS: `.compound-mode-selector`, `.compound-predicates` (left border indent),
  `.compound-separator` (AND/OR labels), `.compound-pred-row`

**Limitation:** Flat compound only — children must be `Rule::Pred`. Cannot nest
groups inside groups. Phase 8b addresses this.

#### Phase 8b: Nested Compound Rules (Tree Editor)

Extend compound rules to support arbitrary nesting. A compound rule's children
can be either a `Rule::Pred` or another `Rule::All`/`Rule::Any` group. This enables
expressions like `life > 100 AND (body armour OR helmet)`:

```
All:
  ├─ HasStatId >= 100 (life)
  └─ Any:
       ├─ ItemClass = "Body Armours"
       └─ ItemClass = "Helmets"
```

The backend already supports this — `Rule::All`/`Rule::Any` take `Vec<Rule>` which
can contain nested compound rules. This is purely a UI extension.

**UI changes:**
- "+ Add Sub-Group" button inside compound groups (alongside "+ Add Condition")
- Nested groups rendered with additional indent level (recursive `PredicateRow`)
- Each sub-group gets its own All/Any toggle
- Delete sub-group collapses its children or removes entirely

**Design constraints:**
- No hard depth limit, but visual indentation makes 3+ levels impractical
- Recursive component: `PredicateRow` renders either a Pred or a nested compound
- Performance: compound rules are small trees (10-20 nodes max), no concern

**Future consideration:** VS Code extension for writing rules in a text DSL outside
of poe-inspect. Power users and community rule-sharers may prefer a textual format
over a visual builder. The text format would compile to the same `Rule` JSON.

#### Phase 8c: Multi-Profile Stacking (Friend Wishlists)

Support multiple active profiles evaluated simultaneously. Primary profile is the
user's build. Secondary profiles are imported from friends or the community.

**Use case:** "I'm farming maps. My primary profile scores gear for my RF Jugg.
My friend imported their Cold DoT Occultist profile as secondary. When I find an
item that matches their wishlist, the overlay shows a secondary indicator so I can
set it aside for them."

**Design:**
- One **primary profile** — full overlay scoring, tier colors, the whole treatment
- Zero or more **secondary profiles** — lightweight indicators on the overlay:
  - Small colored tag/badge: "Alice's Cold DoT: 85%" or a friend icon
  - Distinct visual treatment (border, icon, muted color) so it doesn't clutter
    the primary display
  - Only shown when the item actually matches the secondary profile's filter
- Profile list in settings: each profile has a role selector (Primary / Secondary / Off)
- Import: receive a profile JSON from a friend → import → set as secondary
- Future: share profiles via a link or code (cloud sync, RQE integration)

**Overlay layout with secondaries:**
```
+------------------------------------------+
|  Brood Thirst — Vaal Regalia             |
|  ... normal tier-colored mods ...        |
|------------------------------------------|
|  ★ RF Juggernaut: 75/100 (75%)           |  <- primary score
|  👤 Alice (Cold DoT): Match!             |  <- secondary hit
+------------------------------------------+
```

**Connection to RQE:** Secondary profiles are the local version of what poe-rqe
does at scale. A friend's profile is their "want list". Locally, we evaluate items
against imported friend profiles. In the cloud, poe-rqe matches items against
thousands of registered want lists. The predicate model is shared.

## Future Features (Not Yet Phased)

### Stash Tab Scrolling

Mouse scroll wheel to cycle stash tabs left/right. Popular feature from Awakened PoE Trade.
Need to research their implementation — likely intercepts scroll events when cursor is over
the stash tab header area and sends arrow key presses or tab switch commands.

- Research: look at Awakened PoE Trade's stash scroll implementation for reference
- Detect cursor over stash tab header region
- Convert scroll up/down to stash tab left/right navigation

### Chat Macros

Custom keybindings that send commands to PoE's chat window. Example: F5 → press Enter,
type `/hideout`, press Enter — ports to hideout. Essentially macros restricted to chat
commands only (not arbitrary input).

- Configurable in Settings → Hotkeys (or a dedicated Macros section)
- Each macro: hotkey + chat command string
- Implementation: global shortcut → enigo sends Enter, types command, sends Enter
- Chat-only restriction keeps it within GGG's ToS (one server action per keypress)

### Map Mod Checker

Separate keybinding from item inspect (e.g., Ctrl+M). Evaluates map mods for danger
level: dangerous / warning / fine. Uses poe-eval with map-specific profiles.

- Different hotkey, different overlay presentation (color-coded mod danger, not tier analysis)
- Map profiles are separate from item profiles — different purpose, different predicates
- Shares the same poe-eval engine but with map-oriented rules (e.g., "reflect is dangerous",
  "cannot leech is warning for my build")
- Overlay could be simpler: list of map mods with red/yellow/green indicators
- Profile could include build-specific map mod preferences (RF doesn't care about reflect)

## Open Questions

- **Multiple items:** Show one overlay at a time, or allow pinning/stacking? Start with one, consider pinning later.
- **CSS approach:** CSS Modules vs Tailwind — decide during Phase 2 when building actual components.
