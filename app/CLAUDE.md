# app

Tauri v2 desktop overlay application. The user-facing shell that ties everything together.

## Tech Stack

- **Tauri v2** (2.10.x) — Rust backend + web frontend
- **Preact** (10.x) — lightweight React alternative (~3KB)
- **Vite** (6.x) — build tool + HMR dev server
- **Biome** — linting + formatting
- **TypeScript** — strict mode + extra checks (`noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`)
- **enigo** (0.2.x) — cross-platform keystroke sending (Rust side)

## Project Structure

```
app/
  src/                    # Frontend (Preact + TypeScript)
    main.tsx              # Entry point
    App.tsx               # Root component, event listeners
    styles/overlay.css    # PoE-themed overlay styling
  src-tauri/              # Rust backend
    src/lib.rs            # Tauri setup: tray, hotkey, inspect handler
    src/main.rs           # Entry point
    tauri.conf.json       # Window config (overlay: transparent, always-on-top, no decorations)
    capabilities/         # Permission declarations for plugins
    icons/                # App icons
  package.json            # npm deps + scripts
  biome.json              # Linter/formatter config
  vite.config.ts          # Vite + Preact preset
  tsconfig.json           # TypeScript config (strict)
```

## Scope

- Global hotkey capture (Ctrl+I) → send Ctrl+Alt+C to PoE → read clipboard
- Pass clipboard text to `poe-item` parser → `poe-eval` evaluator (future)
- Render overlay near cursor with evaluation results (tier colors, scores, suggestions)
- Transparent, click-through, always-on-top overlay window
- System tray icon with quit menu
- Profile management UI (create, edit, import/export)
- Settings UI (hotkeys, display preferences, active profiles)

## Does NOT own

- Item parsing logic — that's `poe-item`
- Evaluation logic — that's `poe-eval`
- Game data — that's `poe-data`
- **Profile evaluation rules** — that's `poe-eval`. The app provides a UI to build/edit profiles, but the profile format (predicates, rules, scoring weights) is defined by and serialized from poe-eval's types.

## Domain Boundary: Display vs Evaluation

The app owns **display settings**, poe-eval owns **evaluation profiles**. These must not be mixed.

| Concern | Owner | Examples |
|---------|-------|---------|
| What makes an item good | poe-eval `Profile` | Filter rules, scoring predicates, mod weights |
| How to show results | app display settings | Tier colors, badge visibility, overlay scale, dim/highlight |

**Current state (needs refactor on integration):** `store.ts` has a `Profile` type that mixes both concerns — `modWeights` is evaluation logic (belongs in poe-eval), while `tierColors`/`dimIgnored`/`highlightWeights` are display settings (stays in app). On integration, split into:
- `EvalProfile` — poe-eval's `Profile` serialized as JSON (stored/loaded via poe-eval types)
- `DisplaySettings` — app-owned visual preferences (tier colors, toggles)

## Build

```sh
cd app
npm install
npm run tauri dev     # Dev mode with HMR
npm run tauri build   # Production build
npm run lint          # Biome check
npm run format        # Biome format
```

Note: `app/src-tauri` is excluded from the root workspace (like poe-bundle, poe-query).
First build compiles ~412 crates. Subsequent builds are incremental.

## Key Decisions

- **Excluded from workspace**: Tauri has its own massive dependency tree, and enigo uses unsafe internally. Keeping it isolated avoids conflicts.
- **enigo for keystrokes**: Cross-platform, well-tested. `SendInput` on Windows, Core Graphics on macOS.
- **Win32 `GetCursorPos` for cursor position**: Direct FFI call, avoids pulling in the full `windows` crate.
- **Overlay window config**: Created hidden, positioned on hotkey, shown without stealing focus.
- **Dismiss on**: Escape key, close button, next hotkey press (re-shows with new item).

## Phase Plan

See `docs/app-design.md` for the full plan. Summary:
1. **Phase 1**: Prototype validation (7-point checklist) — IN PROGRESS
2. **Phase 2**: Overlay UI with mock data (tier colors, PoE-native styling)
3. **Phase 3**: Settings & profile management UI
