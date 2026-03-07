# PoE Inspect 2 — Hypothesis

> Living document. Updated as we learn more.

## Problem

In Path of Exile, you pick up a lot of items. Each item has affixes (prefixes, suffixes, implicits, enchants, etc.) that determine its value — both to you and to others. Evaluating items is:

1. **Time-consuming**: You must read each affix, mentally check tiers, compare to your build needs, and consider crafting potential.
2. **Error-prone**: Even veterans miss good items because text parsing under time pressure is hard. People who struggle with fast-reading/pattern-matching are at a bigger disadvantage.
3. **Context-dependent**: "Is this good?" depends on your build, the current meta, trade value, and crafting possibilities — information scattered across multiple tools and websites.

Loot filters (Neversink, etc.) solve the *pickup* decision. Nothing adequately solves the *keep/sell/craft* decision after identification.

Tools like Awakened PoE Trade exist but are limited to basic price checks. They don't do affix tier analysis, crafting potential evaluation, build-aware filtering, or friend wishlists.

## Hypothesis

**A background desktop tool that, on a single hotkey press, captures an item's data, parses it, evaluates it against rich configurable profiles, and shows an overlay with actionable guidance — from simple tier coloring to deep crafting recommendations — will dramatically reduce the time and expertise needed to make good item decisions in PoE.**

## Core User Flow

```
1. User hovers over an identified item in PoE
2. User presses hotkey (e.g. Ctrl+I)
3. Tool sends Ctrl+Alt+C to PoE (advanced item copy) → item text lands on clipboard
4. Tool reads clipboard, parses the item text into structured data
5. Tool maps affixes to game data (tiers, ranges, mod groups) via RePoE/community data
6. Tool evaluates item against active profile(s) and rules
7. Overlay popup appears near cursor with results:
   - Affix tier coloring (T1 = green, T5 = red, user-configurable)
   - Build relevance score ("great for you" / "not useful")
   - Trade value estimate (poe.ninja + trade site)
   - Crafting potential ("open prefix → craft +life" or deeper meta-craft chains)
   - Friend wishlist matches ("Bob is looking for this type of staff")
8. User takes quick action: keep, sell, price, or dismiss
```

## Key Concepts

### Profiles

A profile defines "what matters" for item evaluation. Multiple levels of complexity:

- **Manual rules**: User configures via UI — "on helmets, I want: +life (weight: high), fire res (weight: medium), -cold res (weight: low)". Good dropdowns, searchable mod lists.
- **Build-derived** (future): Import from Path of Building to auto-generate weights for relevant stats.
- **Agent-assisted** (future): An AI agent helps configure the profile conversationally.

Profiles can be:
- **Global**: Default evaluation for all characters
- **Per-character**: Tied to a specific character (auto-detected or manually selected)
- **Watchlist/Friend**: "Look out for X type of item" — matches trigger alerts

### Evaluation Layers

Users choose how deep the analysis goes. Think of these as layers that can be toggled:

| Layer | What it does | Speed |
|-------|-------------|-------|
| **Tier coloring** | Color-code each affix by tier (T1-T7+). Instant visual scan. | Instant |
| **Profile matching** | Score item against active profile. "Good for you? Yes/No + why." | Instant |
| **Trade valuation** | Check poe.ninja / trade API for comparable items. | ~1-2s (network) |
| **Crafting potential** | Check open affixes, suggest deterministic crafts, estimate post-craft value. | Instant to ~1s |
| **Meta awareness** | Cross-reference poe.ninja builds — "used by 15% of Boneshatter Juggernauts." | ~1-2s (network) |
| **Friend wishlist** | Check against saved wishlists from friends/party. | Instant |

### Crafting Rules

Crafting knowledge is **user/community-configurable**, not hardcoded:

- Users or community contributors define craft chains: "If staff has T1 phys and open prefix → use cannot-roll-attack-mods + exalt → guaranteed +1 spell gems"
- The tool checks whether an item matches the preconditions of any known craft chain
- If a match is found: show the craft steps, estimated cost, and estimated post-craft trade value
- Craft rule format should be shareable (import/export, community repository)

## Scope

### PoE1 (Primary)

- Advanced item copy (Ctrl+Alt+C) provides tier information, mod tags, hybrid mod breakdown
- Simple item copy (Ctrl+C) as fallback (less information but always available)
- Mature data ecosystem: RePoE, poe.ninja, poewiki, poedb

### PoE2 (Future, don't block)

- Same engine, similar item structure, but different affix system
- Advanced item copy reportedly added
- Design the parser and data layer so PoE1/PoE2 are selectable (or auto-detected)
- Share as much code as possible between the two

### Auto-Detection

- Detect active PoE version via process name, log file, or window title
- Detect active character via PoE API (OAuth) or Client.txt log file parsing
- Detect current league from API or log

## Technology Preferences

- **Rust** (pedantic clippy, edition 2024) for backend/core logic
- **TypeScript** (strict) for frontend/UI
- **Tauri** (unless research reveals a better cross-platform GUI option) for desktop shell
- **Cross-platform**: Windows, Linux (SteamOS), macOS — all officially supported by PoE

## Prior Work & Learnings

### What to reuse (concepts, not code)

| Source | What's useful |
|--------|--------------|
| `poe-inspect` (old) | Format analysis doc, architecture thinking, parser type designs |
| `poe-item` (TS) | Item type definitions, section parsing approach, test fixtures |
| `poe-item-rust` | Rust struct design for items (Header, Stats, Modifiers, Footer) |
| `poe-item-filter` | **Data pipeline** (repoe-fork fetch → parse → cache), economy data integration, Rust backend patterns |
| `poe-agents` | poe.ninja API integration, PoB CLI, patch notes tooling, agent architecture |

### Lessons from poe-inspect (old)

The previous attempt got bogged down in over-planning and docs without implementation. Key mistakes:
- Too many docs written before any code
- Architecture designed top-down without validating core assumptions (parser, overlay behavior)
- No working prototype to iterate on

**This time**: Validate core assumptions early. Get a working hotkey → parse → overlay loop before expanding scope.

## Research Needed

Before building, we need to investigate:

1. **Existing tools**: What do Awakened PoE Trade, PoE Overlay, Sidekick, etc. actually do? What's their architecture? What are their limitations?
2. **Tauri v2 overlay capabilities**: Global hotkeys, transparent windows, click-through, always-on-top, multi-monitor — does it work well cross-platform?
3. **Item text format**: Comprehensive format spec for both Ctrl+C and Ctrl+Alt+C across item types (we have partial from old project)
4. **Mod/tier data**: repoe-fork `mods.json` structure — can we map parsed affix text → mod ID → tier → value range?
5. **Trade API**: GGG's official trade API for price checking (rate limits, auth, search by mods)
6. **PoE log file (Client.txt)**: What events are logged? Character login? Zone changes? Useful for auto-detection.
7. **Crafting data sources**: Where do deterministic craft recipes live? Community wikis? PoB data?
8. **PoE2 item format**: How different is it? Does advanced copy work?
9. **Overlay alternatives**: Are there better approaches than Tauri for game overlays? (e.g., raw Win32 overlay, Electron alternatives)
10. **Community sharing**: How could craft rules / profiles be shared? JSON format? Git repo? Web service?

## Success Criteria (MVP)

A working tool where:
1. User presses a hotkey while hovering an item in PoE1
2. Item data is captured and parsed correctly (rare items with 6 affixes)
3. Each affix is color-coded by tier
4. An overlay appears near the cursor showing the breakdown
5. Overlay dismisses cleanly (click away or escape)
6. Works on Windows (Linux/Mac can follow)

Everything else (profiles, trade, crafting, friends) is post-MVP.
