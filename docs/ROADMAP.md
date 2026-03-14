# Roadmap

Current priorities, ordered. Updated 2026-03-14.

---

## ~~1. Release Flow (GitHub Actions)~~ ✅

Done. Multi-platform CI in `.github/workflows/release.yml` — triggered on GitHub Release publish or manual dispatch. Builds Windows (.exe/.msi), macOS (.dmg), Linux (.deb/.AppImage) with signing support. Manual dispatch uploads as workflow artifacts.

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

## ~~3. Compact Overlay Mode~~ ✅

Done. Score-only pill for speed-scanning stash tabs.

- Compact inspect hotkey (default `Ctrl+Shift+I`) — small pill near cursor, click-through, auto-dismisses after 2.5s
- Shows item name + score % (color-coded) + watching profile dots
- Maps show DEADLY/CAUTION/SAFE/UNRATED verdict instead of score
- Press full inspect hotkey while pill showing → expands to full overlay (no re-parse)
- Independent compact position setting (cursor vs panel)
- DOM-measured panel positioning replaces hardcoded size estimates
- "Not bound" placeholder UX for unset hotkeys

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

## ~~7. Stash Tab Scrolling & Chat Macros~~ ✅

Done. Both features implemented with Settings UI.

### Stash Tab Scrolling
- `WH_MOUSE_LL` hook on dedicated thread intercepts scroll when PoE is focused
- Configurable modifier key (Ctrl/Shift/Alt/None), default Ctrl+scroll
- Stash area geofencing (like awakened-poe-trade) — lets PoE handle native scroll in tab header area
- Non-Windows: no-op stub (compiles, parked thread)

### Chat Macros
- Hotkey-bound chat commands (e.g., F5 → `/hideout`) in Settings > Chat Macros
- Clipboard-based injection: save clipboard → Enter → Ctrl+A → Ctrl+V → Enter → restore
- Per-macro send toggle (auto-send vs leave chat open)
- Conflict detection against core hotkeys and other macros

### PoE Focus Gate (prerequisite)
- All gameplay hotkeys (inspect, cycle, macros, scroll) gated on PoE foreground window check
- Toggleable in Settings > Behavior (for platforms where detection isn't implemented)
- Windows: `GetForegroundWindow` + window title match; non-Windows: always true (stub)

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
| Inspect + Trade Edit hotkey | app | New hotkey that opens full overlay with trade edit mode already active. Emits inspect-mode with trade-edit flag. |
