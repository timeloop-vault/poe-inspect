# Remaining Review Findings

Items identified in the 2026-03-21 codebase review that were not part of the
structural refactoring. Tracked here for future work.

## Bugs

### Foulborn grammar (poe-item) — BLOCKING 1 TEST
- **File:** `crates/poe-item/src/grammar.pest` (mod_header rule)
- **Issue:** `{ Foulborn Unique Modifier }` header not recognized — "Foulborn" appears
  before the slot keyword, which the grammar doesn't allow.
- **Impact:** 1 test failure (`unique-staff-foulborn-searing-touch.txt`)
- **Fix:** Extend `mod_header` to allow mod names before slot keyword, or add a
  dedicated foulborn rule.

## Enhancements

### poe-dat: Add synthetic unit tests
- **Files:** `src/dat_reader.rs`, `src/stat_desc/parser.rs`, `src/stat_desc/reverse.rs`
- **Issue:** Unit test coverage is minimal — integration tests depend on extracted GGPK
  files. Adding synthetic tests for `parse()`, `ReverseIndex::lookup()`, and error cases
  would improve CI reliability.

### poe-dat: Add module-level docs to lib.rs
- Quick win: add a doc example showing parse → reverse index → lookup flow.

### poe-dat: ReverseIndex version field
- If `NUMBER_PATTERN` changes, deserialized indices will silently use old pattern.
  Adding a version field would catch mismatches.

### poe-dat: Debug logging for unknown transforms
- `TransformKind::Other(_)` silently catches unknown transforms. Adding a
  `tracing::debug!` would help troubleshoot new transform additions.

### poe-item: Extract property name constants
- Magic strings like `"Quality"`, `"Attacks per Second"`, `"Item Level"` could be
  constants. Not urgent but reduces typo risk.

## Platform Gaps

### Linux: Chat macro not implemented
- **File:** `app/src-tauri/src/commands/chat_macro.rs`
- **Issue:** Linux `execute_chat_macro` logs a TODO. Needs clipboard write
  (`wl-copy`/`xclip`) + keystroke injection (XTest/Wayland).

### macOS/Linux: Cursor position fallback
- **File:** `app/src-tauri/src/windows.rs`
- **Issue:** `get_cursor_position` returns `(100, 100)` on macOS and non-Hyprland Linux.
  Needs Core Graphics on macOS, XLib on X11 Linux.

## Frontend

### useTradeFilters hook complexity
- **File:** `app/src/hooks/useTradeFilters.ts` (~350 lines)
- **Issue:** `buildFilterConfig` at the end does manual mapping that could be extracted.
  Below split threshold but worth monitoring.

### Error boundary for settings tabs
- **File:** `app/src/SettingsApp.tsx`
- **Issue:** No error boundary — a throwing settings component crashes the window.
  Low priority but good resilience practice.

### CSS variable defaults
- **File:** `app/src/overlay.css`
- **Issue:** Quality color CSS variables (`--quality-best`, etc.) have no defaults
  in CSS — set via inline styles. Adding `:root` defaults would prevent broken
  rendering if inline styles fail.
