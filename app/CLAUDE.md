# app

Tauri v2 desktop overlay application. The user-facing shell that ties everything together.

## Scope

- Global hotkey capture (e.g. Ctrl+I) → send Ctrl+Alt+C to PoE → read clipboard
- Pass clipboard text to `poe-item` parser → `poe-eval` evaluator
- Render overlay near cursor with evaluation results (tier colors, scores, suggestions)
- Transparent, click-through, always-on-top overlay window
- Profile management UI (create, edit, import/export)
- Settings UI (hotkeys, display preferences, active profiles)
- Network integration: poe.ninja prices, GGG character API, trade API
- Auto-detection: PoE version, active character, current league

## Tech Stack

- **Tauri v2** — Rust backend + TypeScript/web frontend
- **Frontend**: Framework TBD (likely Solid, Svelte, or React)
- Must validate 7-point prototype checklist before committing:
  1. Global hotkey while PoE is focused
  2. Transparent overlay window
  3. Click-through behavior
  4. Always-on-top over fullscreen PoE
  5. Multi-monitor support
  6. Cursor-relative positioning
  7. Cross-platform (Windows primary, Linux/SteamOS secondary)

## Does NOT own

- Item parsing logic — that's `poe-item`
- Evaluation logic — that's `poe-eval`
- Game data — that's `poe-data`

## Dependencies

- All workspace crates (`poe-dat`, `poe-data`, `poe-item`, `poe-eval`)

## Status

**Future phase.** Core crates (poe-dat → poe-data → poe-item → poe-eval) must be proven first. The Tauri prototype validation is a prerequisite before building this out.

## Plan

1. Tauri v2 prototype: validate the 7-point checklist above
2. Minimal hotkey → parse → overlay loop (hardcoded profile, no persistence)
3. Profile management UI
4. Network integrations (poe.ninja, trade API)
5. Settings, auto-detection, polish
