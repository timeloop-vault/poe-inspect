# Roadmap

Current priorities, ordered. Updated 2026-03-13.

---

## 1. Release Flow (GitHub Actions)

Build and publish release artifacts via GitHub Actions.

**Artifacts:**
- Windows: `.exe` installer (or `.msi`)
- macOS: `.dmg`
- Linux: `.deb` (and/or `.AppImage`)

**Requirements:**
- Triggered on git tag push (e.g., `v0.1.0`) or manual dispatch
- Build in release mode — all debug/mock features disabled
- Code-sign where possible (macOS notarization, Windows optional)
- Upload artifacts to GitHub Releases
- Tauri v2 has built-in bundler support for all three platforms (`tauri build`)

**Key decisions:**
- How to disable mock/debug features at build time (feature flag, env var, or `cfg`)
- Whether to cross-compile or use per-platform runners (macOS needs macOS runner)
- Versioning strategy (Cargo.toml + tauri.conf.json + package.json alignment)

---

## 2. Data Extraction / Update Flow (GitHub Actions)

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

## 3. Compact Overlay Mode

Score-only pill for speed-scanning stash tabs.

**UX:**
- Separate hotkey (or modifier key) triggers compact mode instead of full overlay
- Shows: profile name + score + tier color as a small pill near cursor
- Expand: press full-overlay hotkey to see detailed stats/trade/mods
- Use case: hovering items quickly in stash — just need pass/fail signal

**Implementation:**
- New hotkey binding in settings (e.g., `Ctrl+Shift+I` or configurable)
- Compact CSS layout (minimal pill component)
- Setting to choose default mode (compact vs full)
- Reuses existing evaluation — just a different rendering path

---

## 4. HasStatText Deprecation

Replace remaining `HasStatText` (substring matching) with `HasStatId` (stat ID matching).

- Audit all existing profiles/rules for `HasStatText` usage
- Migrate to `HasStatId` equivalents
- Consider removing `HasStatText` from the schema entirely, or marking deprecated
- Ensures all matching is language-independent and unambiguous

---

## 5. `(Local)` Trade Stat Suffix

Trade queries for local stats (e.g., `local_base_evasion_rating`) need the `(Local)` suffix appended to match the trade API's stat text format.

- poe-trade query builder: detect local stat_ids, append `(Local)` when building trade stat filters
- Verify against trade API stat index to confirm which stats need the suffix
- Test with local defense mods (armour, evasion, energy shield) and local attack mods (phys damage, attack speed)

---

## 6. Parse More Stat Description Files

Bumps trade stat match rate above 87.4%. Currently unparsed files:

- Atlas stat descriptions
- Map stat descriptions (partially done for map danger)
- Sanctum stat descriptions
- Heist stat descriptions

Each file adds stat patterns to the reverse index → more trade stats resolve to stat_ids → better trade query coverage.

---

## 7. Stash Tab Scrolling & Chat Macros

Quality-of-life features that complete the awakened-poe-trade replacement.

### Stash Tab Scrolling
- Intercept mouse scroll when cursor is over stash tab header area
- Convert scroll up/down to stash tab left/right navigation
- Research awakened-poe-trade's implementation for reference

### Chat Macros
- Custom hotkeys that send chat commands (e.g., F5 → `/hideout`)
- Configurable in Settings (hotkey + command string)
- Implementation: global shortcut → enigo sends Enter, types command, sends Enter
- Chat-only restriction stays within GGG's ToS (one server action per keypress)

---

## Backlog (Unordered)

These are known gaps and future features, not currently prioritized.

| Item | Layer | Notes |
|------|-------|-------|
| Pseudo stats | poe-eval | Sum matching stat lines (e.g., "pseudo total life >= 140"). New `PseudoStatValue` predicate. |
| Anointment trade stats | poe-trade | Option-based stats (`Allocates #` with dropdown). Needs `StatFilter.option` field. |
| Memory Strands | poe-data/item/eval | New Mirage 3.28 mechanic. Surface as first-class field, factor into evaluation. |
| Count-of-N combinator | poe-eval + app | `Rule::Count(n, Vec<Rule>)` — "at least N of these conditions". |
| Reusable condition templates | app | Save/insert condition templates across rules and profiles. |
| Craft suggestions | poe-data + app | Extract `CraftingBenchOptions` from GGPK, show "bench-craft X" in overlay. |
| Rule text DSL | poe-eval + app | Text format compilable to Profile JSON. VS Code extension. |
| Multi-line stat lookups | poe-item | Some stats span two lines in item text. |
| Ctrl+C fallback parser | poe-item | Only Ctrl+Alt+C supported; Ctrl+C has less data but is more common. |
| `{ Foulborn Unique Modifier }` | poe-item | Grammar doesn't handle mod name before "Unique" keyword. |
| macOS cursor position | app | Currently hardcoded `(100, 100)`. Needs Core Graphics API. |
| CSS split (overlay vs settings) | app | Separate entry points. Low priority — class-scoping works. |
| Overlay sprites | app | Foil/Quest/Prophecy headers, Div Card separator, influence overlays. |
| Item browser for rule building | app | Browsable poe-data database for looking up base types, mods. |
