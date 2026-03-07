# Research Synthesis & Analysis

> Compiled from 7 research documents. See `docs/research/` for full details.

## Validation of Hypothesis

**The hypothesis is strongly validated.** No existing tool combines tier analysis + crafting potential + build awareness + trade valuation in a single overlay. The gap is real and significant.

### Competitive Landscape

| Capability | Awakened Trade | Sidekick | Craft of Exile | Path of Building | **poe-inspect-2** |
|---|---|---|---|---|---|
| In-game overlay | Yes | Yes | No | No | **Yes** |
| Price check | Yes | Yes | No | No | **Yes** |
| Affix tier display | No | Basic | Yes (web) | Manual | **Yes, with roll quality** |
| Crafting potential | No | No | Yes (web) | No | **Yes, in overlay** |
| Build-aware eval | No | No | No | Manual | **Yes, automated** |
| Cross-platform | Win/Mac/Linux | Windows only | Web | Win/Mac/Linux | **Win/Mac/Linux** |

**Our unique value**: The only tool that answers "what is this item worth **to you** and **what could it become**" — in an overlay, in real-time.

---

## Technology Decision: Tauri v2

**Recommendation: Proceed with Tauri v2.** Validated across research:

### Strengths
- All overlay features work on Windows (our primary target)
- Transparent + always-on-top + click-through + no-focus windows all supported
- Global hotkeys via `tauri-plugin-global-shortcut`
- Clipboard via `tauri-plugin-clipboard-manager`
- Rust backend is a natural fit for our core logic (parser, data pipeline, evaluation)
- Small binary (2-8 MB) vs Electron's 50-150 MB — critical for gaming
- HTML/CSS/TypeScript frontend for rich, customizable overlay UI

### Gaps to Fill
| Gap | Solution |
|-----|----------|
| Sending keystrokes to PoE | `enigo` crate (cross-platform) or direct `windows` crate `SendInput` |
| Global cursor position | `enigo` or platform-specific API |
| Wayland (SteamOS) | Target XWayland (games run under Proton/XWayland anyway) |
| macOS accessibility | Detect + prompt user (standard for overlay tools) |

### Prototype Checklist (validate before committing)
1. Transparent always-on-top window over PoE in borderless windowed mode
2. Global hotkey fires while PoE has focus
3. Click-through toggle works smoothly
4. Send Ctrl+Alt+C to PoE via `enigo`
5. Read clipboard after keystroke delay
6. Position overlay near cursor
7. Showing overlay does NOT steal focus from PoE

**Fallback**: egui (pure Rust, no webview) if Tauri proves problematic.

---

## Data Pipeline Architecture

### Game Data (Static, refreshed per patch)

All from **repoe-fork.github.io** — JSON, well-structured, actively maintained:

| File | Purpose | Size |
|------|---------|------|
| `mods.json` | 37k mods with tiers, spawn weights, groups, tags | ~20 MB |
| `stat_translations.json` | Map stat IDs to display text templates | ~11 MB |
| `base_items.json` | Base types with tags, classes, properties | ~500 KB |
| `crafting_bench_options.json` | Bench crafts with costs and restrictions | ~200 KB |
| `essences.json` | Essence → forced mod mapping | ~100 KB |
| `fossils.json` | Fossil weight modifiers | ~50 KB |
| `item_classes.json` | Item class definitions | ~50 KB |

**Prior art**: poe-item-filter has a working Rust pipeline (`backend/src/data/`) for fetching, parsing, and caching this data. Key filtering logic: `domain == "item"` and `release_state == "released"` to exclude internal/test items. Data loaded once at startup as `Arc<GameData>`, shared across all handlers — no runtime locking for reads.

### Economy Data (Dynamic, refreshed periodically)

**poe.ninja API** (rate limit: 12 req/5 min, no auth required):

| Endpoint | URL Pattern | Types |
|----------|-------------|-------|
| Currency | `currencyoverview?league={LEAGUE}&type={TYPE}` | Currency, Fragment |
| Items | `itemoverview?league={LEAGUE}&type={TYPE}` | UniqueWeapon, UniqueArmour, UniqueAccessory, UniqueFlask, UniqueJewel, DivinationCard, SkillGem, BaseType, Map, Scarab, Fossil, Essence, Oil, etc. |
| Builds | `data/{snapshotId}/getbuildoverview?overview={SLUG}&type=exp` | Aggregate class/skill/item popularity |

**Gotcha**: League name format differs by endpoint — economy uses short display name (`Mirage`), builds uses lowercase slug (`mirage`).

**Prior art**: poe-agents has a full Python CLI (`tools/poe_ninja.py`) with endpoint catalog and 5s throttle. poe-item-filter has Rust economy parsing with league fallback strategy (if <10 confident items in primary league, merge with fallback league data).

**Adaptation needed**: Background pre-fetching + local cache with TTL-based refresh (not just presence-check). Currency vs item endpoints use different JSON field names — must handle both.

### Trade Data (On-demand)

**GGG Trade API** (no auth required, rate limited ~12 req/6s):
- Two-step search/fetch pattern
- Search by mod stat IDs with value ranges
- Weight-based search for custom scoring
- Separate PoE1 (`/api/trade/`) and PoE2 (`/api/trade2/`) endpoints

**Key insight**: Tier-based searching requires converting tier ranges to value ranges ourselves (trade API doesn't support tier filtering directly).

### Character Data (Optional, post-MVP)

**Two access methods discovered:**

1. **GGG Public Character API** (no auth, profile must be public):
   - `GET /character-window/get-characters?accountName={ACCOUNT}&realm=pc`
   - `GET /character-window/get-items?accountName={ACCOUNT}&character={CHAR}&realm=pc`
   - `GET /character-window/get-passive-skills?accountName={ACCOUNT}&character={CHAR}&realm=pc`
   - Gotchas: account names with `#` need URL encoding, item names contain `<<set:MS>>` markup to strip, returns 403 if Characters tab is private

2. **GGG OAuth API** (PKCE flow, works for private profiles):
   - Character list, equipped items, passive tree, stash tabs
   - More reliable but requires auth flow

**Client.txt log tailing** (cross-platform via `notify` crate):
- Zone change detection (`"You have entered X"`)
- Active character inference
- File grows unbounded — must tail, never read whole file

---

## Item Parsing Pipeline

### Proven approach (from poe-inspect v1 codebase)

The old project has a **working Rust parser** in `packages/poe-parser/` with:

1. **Line-by-line state machine** — classifies lines into sections (Header, Properties, Requirements, Modifiers, etc.)
2. **Regex patterns** — lazy-static compiled, covers all format elements
3. **Format detection** — distinguishes Simple (Ctrl+C) vs Advanced (Ctrl+Alt+C) by presence of `{ }` headers
4. **Template extraction** — strips numeric values, creates lookup key for stat_translations
5. **ModDatabase** — loads mods.json, filters to rollable mods, builds tier tables per stat ID
6. **BaseItemDatabase** — resolves base type → tags for spawn weight filtering
7. **Tier calculation** — value → tier number + roll quality (0.0 to 1.0)

### What advanced format gives us (and why it matters)

With Ctrl+Alt+C, each mod comes with:
- **Mod name** ("Hale") → direct lookup in mods.json
- **Type** (Prefix/Suffix) → no guessing needed
- **Tier number** → verification against calculated tier
- **Value range** `(40-49)` → tier range inline, no database needed for display
- **Tags** → useful for filtering/display
- **Hybrid grouping** → multiple stat lines under one `{ }` header

This means **the advanced format short-circuits most of the complex lookup pipeline**. The database is still needed for:
- Finding ALL tiers (not just current)
- Open affix slot detection
- Crafting potential analysis
- Trade API stat ID mapping

### PoE2 compatibility

PoE2 advanced copy uses the same `{ }` header format. Differences are contained:
- Different requirements format (single line vs multi-line)
- `S` sockets instead of `R-G-B`
- `(rune)` markers
- Different mod pool (different mods.json)
- No eldritch/fractured/synthesised

**Architecture**: Abstract game version at the data layer. Parser handles both with minor branching.

---

## Crafting Data Assessment

### Fully automatable from RePoE data

| Analysis | Data Source | Complexity |
|----------|-----------|------------|
| Open prefix/suffix detection | Parse item + count mods by type | Low |
| Bench craft suggestions | `crafting_bench_options.json` + open slots | Low |
| Possible mods on this base | `mods.json` + `base_items.json` tag filtering | Medium |
| Crafting probability calculation | Spawn weight math | Medium |
| Fossil optimization | `fossils.json` weight modifiers | Medium |
| Essence forced mod lookup | `essences.json` | Low |

### Requires user/community configuration

| Analysis | Why | Format needed |
|----------|-----|---------------|
| Multi-step meta-craft recipes | Pure community knowledge, not in any data file | JSON/YAML rule definitions |
| "Best way to craft X" | Strategic/procedural knowledge | Step-by-step recipe format |
| Cost-benefit analysis | Requires economy data + recipe steps | Computed from rules + poe.ninja |

---

## Evaluation Layers (Revised)

Based on research findings, here's the refined layer architecture:

### Layer 1: Tier Coloring (MVP)
- Parse item → identify each affix → determine tier + roll quality
- Color-code by tier (T1=gold, T2=green, ... T7+=red, user-configurable)
- Show roll quality indicator (e.g., 89/100 within tier range)
- **Data needed**: mods.json, stat_translations.json, base_items.json
- **Latency**: Instant (all local data)

### Layer 2: Affix Breakdown (MVP)
- Prefix/suffix classification
- Open slot count (X/3 prefixes, Y/3 suffixes)
- Crafted/fractured/implicit markers
- Influence identification
- **Data needed**: Same as Layer 1
- **Latency**: Instant

### Layer 3: Profile Matching (Post-MVP v1)
- User-defined profiles: "I want life, fire res, spell damage on helmets"
- Score each affix against profile weights
- Overall item score: "85% match for your profile"
- **Data needed**: User profile + Layer 1 data
- **Latency**: Instant

### Layer 4: Trade Valuation (Post-MVP v1)
- Construct trade search from item's key mods
- Fetch comparable listings
- Show estimated price range
- **Data needed**: Trade API stat IDs + network access
- **Latency**: 1-3s (network + rate limiting)

### Layer 5: Crafting Potential (Post-MVP v2)
- Open slot analysis → bench craft suggestions
- Mod pool analysis → "what can still roll?"
- Probability estimates for chaos/fossil/essence crafting
- Community craft recipe matching
- Post-craft value estimation (craft + re-check trade)
- **Data needed**: crafting_bench_options.json, essences.json, fossils.json, community rules
- **Latency**: Instant for bench crafts, 1-3s for trade-based valuation

### Layer 6: Build Awareness (Post-MVP v2)
- OAuth import of character equipment + passives
- "This is a DPS upgrade" / "This doesn't help your build"
- **Data needed**: OAuth API + PoB-like calculations (complex)
- **Latency**: Instant (cached character data)

### Layer 7: Meta & Social (Post-MVP v3)
- poe.ninja builds: "Used by 15% of Boneshatter Juggernauts"
- Friend wishlists: match against shared profiles
- **Data needed**: poe.ninja builds API + social layer
- **Latency**: 1-3s (network)

---

## Prior Art: Reusable Patterns

### From poe-item-filter (Rust)

**Data structures** — directly adaptable:
```rust
// BaseItem: extend with tags for spawn weight filtering
pub struct BaseItem {
    pub name: String, pub item_class: String, pub drop_level: u32,
    pub req_str: u32, pub req_dex: u32, pub req_int: u32, pub req_level: u32,
    pub width: u32, pub height: u32,
    pub armour: Option<u32>, pub evasion: Option<u32>,
    pub energy_shield: Option<u32>, pub ward: Option<u32>,
}

// EconomyItem: chaos_value + listing_count are key valuation data
pub struct EconomyItem {
    pub name: String, pub base_type: Option<String>,
    pub chaos_value: f64, pub divine_value: Option<f64>,
    pub listing_count: u32, pub item_type: EconomyItemType,
}
```

**Architecture patterns**:
- `Arc<GameData>` shared state — load once at startup, share immutably across all handlers
- League fallback strategy — if primary league has sparse economy data, merge with fallback
- Filter by `domain == "item"` + `release_state == "released"` to avoid internal items
- Item struct with 40+ fields (from evaluator) — comprehensive reference for what properties to capture

**Clippy config** (proven pragmatic baseline):
```toml
[lints.rust]
unsafe_code = "forbid"
[lints.clippy]
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
must_use_candidate = "allow"
cast_possible_truncation = "allow"
cast_sign_loss = "allow"
cast_precision_loss = "allow"
```

### From poe-agents (Python)

**PoB integration strategy**: Don't embed LuaJIT. Parse PoB XML directly:
1. User provides PoB code → decode (base64url + deflate) → XML
2. Extract class, ascendancy, skill gems, current gear from XML
3. Derive stat weights (crit build → values crit chance/multi; DoT → values DoT multi)
4. This becomes the evaluation profile — no calc engine needed

**Domain knowledge** worth encoding:
- Budget tiering: league start (0-5 div), mid (5-30 div), endgame (30+)
- Stat categorization: offense (TotalDPS, CritChance, etc.), defense (Life, EHP, Resists), attributes
- Build archetype → stat weight mappings as core of evaluation profiles
- Early-league vs late-league price awareness

**Design principles**:
- Aggressive local caching — all external data cached to disk with timestamps
- Throttling as first-class concern — every API call through a throttle layer
- Separate data gathering from intelligence — pricing layer cleanly separated from evaluation/scoring

### Recommended Crate Stack

| Crate | Purpose |
|-------|---------|
| `serde` + `serde_json` | Serialization (core) |
| `thiserror` | Error types |
| `tokio` | Async runtime |
| `reqwest` | HTTP client (data fetching) |
| `tracing` + `tracing-subscriber` | Structured logging |
| `pretty_assertions` | Better test diffs (dev) |
| `enigo` | Cross-platform keystroke sending |
| `arboard` or `clipboard-rs` | Clipboard access |

### Recommended Frontend Stack

| Technology | Version | Purpose |
|-----------|---------|---------|
| Preact | 10.x | UI framework (3KB, proven stable — avoid 11 beta) |
| @preact/signals | 2.x | Reactive state |
| TypeScript | 5.x | Type safety |
| Vite | 6.x | Build tool |
| Tailwind CSS | 4.x | Styling (CSS-based config, no tailwind.config.js) |
| Biome | 2.x | Linter + formatter |

---

## MVP Scope (Refined)

Based on all research, the MVP is:

1. **Tauri v2 prototype** — validate the 7-point overlay checklist on Windows
2. **Item parser** — port/adapt the proven Rust parser from poe-inspect v1
3. **Data pipeline** — fetch mods.json, stat_translations.json, base_items.json from repoe-fork
4. **Tier engine** — map parsed affixes to tiers + roll quality
5. **Overlay UI** — color-coded affix display with tier info, prefix/suffix breakdown, open slots
6. **Hotkey flow** — Ctrl+I → send Ctrl+Alt+C → read clipboard → parse → show overlay

### What's explicitly NOT in MVP
- Trade integration (needs rate limit handling, stat ID mapping)
- Profiles / scoring
- Crafting suggestions
- Build awareness (OAuth, PoB integration)
- Meta / social features
- PoE2 support
- Linux / macOS support (Windows first, cross-platform after)

---

## Risk Register

| Risk | Severity | Mitigation |
|------|----------|------------|
| Tauri overlay doesn't work well with PoE | High | Build prototype first (7-point checklist). Fallback: egui |
| mods.json is 20MB, slow to load | Medium | Pre-process into optimized format at build/first-run time (like v1's template-stat-lookup.json) |
| Rate limits on trade API | Medium | Smart caching, debouncing, request queue with header-based throttling |
| Parser breaks on edge cases | Medium | Extensive test fixtures from real items. Graceful degradation |
| Wayland/SteamOS issues | Medium | Target XWayland. Pure Wayland is stretch goal |
| GGG changes item format | Low | Line-by-line state machine is resilient. Monitor patch notes |
| Community craft rules need a good format | Low | Start with JSON, iterate based on community feedback |

---

## Next Steps

1. **Create Tauri v2 prototype** — validate overlay capabilities on Windows
2. **Port parser** — adapt poe-inspect v1's Rust parser (state machine + regex + type system)
3. **Build data pipeline** — fetch + cache RePoE data, build tier lookup tables
4. **Build overlay UI** — TypeScript/HTML/CSS display for tier-colored affix breakdown
5. **Wire it all together** — hotkey → clipboard → parse → evaluate → display
6. **Test with real items** — gather test fixtures, verify tier accuracy

### Key Reference Files (from prior art repos)

| File | Why |
|------|-----|
| `_reference/poe-item-filter/backend/src/data/models.rs` | Core data structures (BaseItem, EconomyItem) |
| `_reference/poe-item-filter/backend/src/data/economy.rs` | poe.ninja integration with fallback |
| `_reference/poe-item-filter/backend/src/data/parse.rs` | repoe-fork JSON parsing + filtering |
| `_reference/poe-item-filter/backend/src/evaluate.rs` | Item struct (40+ fields), matching logic |
| `_reference/poe-item-filter/backend/Cargo.toml` | Proven crate versions + lint config |
| `_reference/poe-agents/tools/poe_ninja.py` | poe.ninja endpoint catalog + throttling |
| `_reference/poe-agents/tools/pob_codec.py` | PoB code encode/decode |
| `_reference/poe-agents/tools/poe_character.py` | GGG character API usage |
