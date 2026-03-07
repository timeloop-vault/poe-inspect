# Prior Work Analysis: poe-item-filter

Research extracted from `_reference/poe-item-filter/` for adaptation in poe-inspect-2.

---

## 1. Data Pipeline (Rust)

### How It Works

The backend fetches game data on first startup from two sources:

1. **repoe-fork.github.io** — three static JSON files:
   - `base_items.min.json` — all base items with properties (name, item_class, drop_level, requirements, dimensions, defences)
   - `item_classes.json` — ~60 item class definitions with categories and influence tags
   - `gems.min.json` — all gems with tags, attributes, and support status

2. **poe.ninja API** — economy data across 8 item categories:
   - Currency endpoints: `currencyoverview` for Currency and Fragment
   - Item endpoints: `itemoverview` for UniqueWeapon, UniqueArmour, UniqueAccessory, UniqueFlask, UniqueJewel, DivinationCard
   - Each endpoint uses a different JSON schema (currency uses `currencyTypeName`/`chaosEquivalent`/`receive.listing_count`; items use `name`/`chaosValue`/`listingCount`)

**Fetcher** (`backend/src/data/fetcher.rs`): Simple async download using reqwest + tokio::fs. Downloads to a `data/` directory. Writes a `metadata.json` with the source URL. No timestamp tracking — just presence-check: if the file exists, skip fetching.

**Parser** (`backend/src/data/parse.rs`):
- `parse_base_items()` filters by `domain == "item"` and `release_state == "released"` — critical to avoid internal/unreleased items
- `parse_gems()` filters by `release_state == "released"` and derives `primary_attribute` and `suggested_weapon_classes` from tag analysis
- `derive_weapon_classes()` maps gem tags to weapon types (e.g., bow -> Bows, melee+axe -> One Hand Axes/Two Hand Axes, spell+strength -> Sceptres/Staves)

**Data model** (`backend/src/data/mod.rs`): `GameData` struct organizes everything:
- `items_by_class: HashMap<String, Vec<BaseItem>>` — items sorted by drop_level within each class
- `gems_by_name: HashMap<String, GemInfo>` — indexed by lowercase name
- `item_classes: Vec<ItemClassInfo>`
- `character_classes: Vec<CharacterClass>` — hardcoded 7 classes with attributes and ascendancies
- `economy: Option<EconomyData>`

### What Can Be Directly Adapted

- **repoe-fork data fetching**: The same 3 JSON files are useful for poe-inspect-2. The fetcher pattern (check if cached, download if missing, write to data dir) is simple and reusable.
- **Base item filtering logic**: The `domain == "item"` and `release_state == "released"` filters are essential — without them you get hundreds of internal/test items.
- **Economy data fetching**: The poe.ninja integration with league-aware fallback is directly reusable. The dual-endpoint format handling (currency vs item) is already solved.
- **`GameData` as a shared `Arc` resource**: Loaded once at startup and shared via `Arc<GameData>` across all handlers.

### Data Structures to Reuse

```rust
pub struct BaseItem {
    pub name: String,
    pub item_class: String,
    pub drop_level: u32,
    pub req_str: u32,
    pub req_dex: u32,
    pub req_int: u32,
    pub req_level: u32,
    pub width: u32,
    pub height: u32,
    pub armour: Option<u32>,
    pub evasion: Option<u32>,
    pub energy_shield: Option<u32>,
    pub ward: Option<u32>,
}

pub struct EconomyItem {
    pub name: String,
    pub base_type: Option<String>,
    pub chaos_value: f64,
    pub divine_value: Option<f64>,
    pub listing_count: u32,
    pub item_type: EconomyItemType,
    pub links: Option<u32>,
}

pub enum EconomyItemType {
    Currency, Fragment, UniqueWeapon, UniqueArmour,
    UniqueAccessory, UniqueFlask, UniqueJewel, DivinationCard,
}
```

For poe-inspect-2, the `BaseItem` struct needs extension for PoE 2 properties (if targeting PoE 2) or is usable as-is for PoE 1. The `EconomyItem` struct is directly reusable — the chaos_value and listing_count fields are the key data points for item valuation.

### Pitfalls and Lessons

- **League timing for economy data**: Early in a league, poe.ninja has sparse data. The project implements a fallback strategy: if the primary league has <10 confident items (listing_count >= 5), it merges with a fallback league's data. The `LEAGUE` and `FALLBACK_LEAGUE` env vars control this.
- **Currency vs item endpoint differences**: poe.ninja uses completely different JSON field names for currency endpoints vs item endpoints. The code handles both formats in `economy.rs`.
- **No staleness tracking**: The project only checks if files exist, not how old they are. For poe-inspect-2 (a live overlay), economy data freshness matters more — consider adding TTL-based refresh.
- **Error handling uses `Result<(), String>`**: Simple but not idiomatic. The project acknowledges this is acceptable for a fetch-on-startup pattern but wouldn't scale for runtime error handling.

---

## 2. Cached Data Files

### What's Cached

All cached data lives in `backend/data/`:

| File | Source | Content |
|------|--------|---------|
| `base_items.min.json` | repoe-fork | All released base items (very large, ~180K tokens) |
| `item_classes.json` | repoe-fork | ~60 item classes with categories (752 lines) |
| `gems.min.json` | repoe-fork | All released gems with tags |
| `economy.json` | poe.ninja | Merged economy data across 8 categories |
| `metadata.json` | generated | Source URL only (`{"source": "https://repoe-fork.github.io"}`) |

### item_classes.json Structure

```json
{
  "Sceptre": {
    "category": "One Hand Mace",
    "category_id": "One Hand Mace",
    "name": "Sceptres",
    "influence_tags": [...]
  }
}
```

Includes recent additions: VaultKey, Tincture, AnimalCharm, Gold, BrequelGraft, BrequelFruit — these reflect PoE 1 patches through 3.28 (Mirage league).

### What Can Be Directly Adapted

- **The cached data files themselves** can be used as reference data for poe-inspect-2 if targeting PoE 1. The item_classes.json is particularly useful as a complete enumeration.
- **The caching pattern**: Download once, load from disk thereafter. Simple and effective.

### What poe-inspect-2 Needs Differently

- **Runtime refresh**: An overlay tool needs fresher economy data than a filter builder. Consider periodic background refresh (e.g., every 30-60 minutes for economy data).
- **Additional data sources**: For item evaluation, poe-inspect-2 likely needs mod tier data, affix weightings, and crafting data that aren't in repoe-fork's 3 files. The research doc (`docs/research/data-sources.md`) identifies poewiki.net's Cargo API as a supplementary source for mod/affix data.
- **Metadata should include timestamps**: The current metadata.json only tracks source, not fetch time. Adding timestamps enables staleness checks.

---

## 3. Filter Engine Architecture

### AST Design

The filter engine has four layers: **Parser -> AST -> Evaluator -> Serializer**.

**AST** (`backend/src/ast.rs`):
- `Filter { blocks: Vec<Block> }`
- `Block { block_type: BlockType, conditions: Vec<Condition>, actions: Vec<Action>, has_continue: bool, line: Option<usize>, comments: Vec<String> }`
- `BlockType`: Show, Hide, Minimal
- `Condition` enum: 37+ variants covering numeric (ItemLevel, DropLevel, Quality, Sockets, LinkedSockets, AreaLevel, etc.), string/list (Class, BaseType, HasExplicitMod, etc.), rarity, and boolean (Identified, Corrupted, Mirrored, SynthesisedItem, FracturedItem, Foulborn, Imbued, etc.)
- `Action` enum: 16 variants (SetBorderColor, SetTextColor, SetBackgroundColor, SetFontSize, PlayAlertSound, PlayAlertSoundPositional, MinimapIcon, PlayEffect, CustomAlertSound, CustomAlertSoundOptional, DisableDropSound, EnableDropSound, DisableDropSoundIfAlertSound, SetStackSize, SetSpecialActionSound, SetDoubleBorderColor)
- `Operator { op: OpType, value: i64 }` with `OpType`: Eq, Lt, Gt, LtEq, GtEq, Exact

**Parser** (`backend/src/parser.rs`): Line-by-line, uses `BlockBuilder` pattern. Preserves comments. Handles inline comments while respecting quoted strings. First word of each line is the keyword; tries condition parse first, then action, then error.

**Evaluator** (`backend/src/evaluate.rs`): First-match-wins with Continue support. The `Item` struct has 40+ fields with serde defaults. Key behaviors:
- `string_match()` is case-insensitive; exact mode (triggered by `==`) requires full equality, default mode uses substring matching
- `socket_group_matches()` checks if all pattern characters exist in a linked group
- `apply_actions()` merges actions into `ItemStyle` with last-wins semantics
- Continue blocks apply their actions and keep searching for the next match

**Serializer** (`backend/src/serializer.rs`): Reverse of parser. Handles alpha=255 omission, default volume=100 omission. Roundtrip tests verify parser -> serializer -> parser identity.

### Relevance to poe-inspect-2

The filter engine itself is less directly relevant to an item evaluation overlay — poe-inspect-2 evaluates items against value heuristics, not filter rules. However:

- **The `Item` struct** from `evaluate.rs` is a comprehensive model of all item properties that PoE exposes. This is valuable as a reference for what properties poe-inspect-2 needs to capture from the game client or clipboard.
- **The condition evaluation logic** (operator comparisons, string matching, socket group matching) could be adapted for item matching rules in an evaluation engine.
- **The `ItemStyle` struct** (colors, font size, minimap icon, sound) is relevant if poe-inspect-2 wants to display items similarly to how the game filter would show them.

### Session Management Pattern

`FilterSession` uses stable `RuleId` (monotonically increasing u64) to reference rules across insertions and deletions. This pattern is useful for any system where external references (LLM tool calls, UI element IDs) need to survive collection mutations.

```rust
pub struct FilterSession {
    rules: Vec<Rule>,
    next_id: u64,
}
pub struct Rule {
    pub id: RuleId,
    pub block: Block,
}
pub enum InsertPosition { First, Last, Before(RuleId), After(RuleId), AtIndex(usize) }
```

---

## 4. Research Documents

The `docs/research/` directory contains four substantial research documents:

### data-sources.md
- Complete analysis of PoE data sources: repoe-fork (primary), poewiki.net Cargo API (supplementary), poe.ninja (economy), poedb.tw (avoid — no API, scraping-hostile)
- Exact URL patterns and JSON response structures for all sources
- Complete data mapping table showing which source provides which data
- **Key insight for poe-inspect-2**: poewiki.net's Cargo API can provide mod tier data, affix info, and other item property data not in repoe-fork. URL pattern: `https://www.poewiki.net/api.php?action=cargoquery&tables=...`

### poe1-filter-syntax.md
- 984-line comprehensive filter syntax reference
- All block types, conditions (with value ranges), actions, operators
- Evaluation rules and pseudo-EBNF grammar
- Complete item class list (~60 classes)
- **Useful for poe-inspect-2**: The complete item class list and condition value ranges serve as a reference for item property validation

### league-changes.md
- Filter-affecting changes from PoE 3.23 through 3.28
- **Key lessons**:
  - 3.25 socket rework was the most disruptive change ever (LinkedSockets deprecated, Sockets became total count)
  - 3.28 map system overhaul broke map BaseType filtering
  - Filter syntax is mostly stable (1-3 new conditions per league)
  - GGG's "Item Filter Information" forum posts are the canonical source
- **Relevance for poe-inspect-2**: Understanding which item properties changed between leagues helps build a robust item model

### llm-integration.md
- Extensive Rust LLM ecosystem research (March 2026 vintage)
- Multi-provider crates: `llm` (most complete), `genai`, `rig-core`
- Single-provider: `async-openai`, `misanthropic`, `gemini-rust`
- Local LLM: Ollama (recommended for dev), llama.cpp, vLLM
- Tool-use wire format differences across providers (OpenAI vs Anthropic vs Gemini)
- **Directly applicable to poe-inspect-2** if it incorporates LLM-based item evaluation or chat features

### frontend-setup.md
- Preact 10.x (not 11 beta — it was unstable)
- Preact Signals for state management
- Tailwind CSS v4 (new CSS-based config, no tailwind.config.js)
- Biome for linting/formatting (replaces ESLint + Prettier)
- `rust-embed` for single-binary deployment (embed frontend assets in Rust binary)
- Custom chat UI pattern: 5-6 components, native WebSocket API, marked + DOMPurify for markdown rendering

### plan-orchestration.md and test-prompts.md
- Documents LLM behavior issues: Qwen 2.5 Coder 32B stops after one tool call instead of chaining multiple calls to build a complete filter
- Recommends template tool + prompt tuning approach
- Test prompts show concrete failures: LLM used `Sockets >= 4` instead of `LinkedSockets >= 4`, no attribute filtering, no drop level awareness
- **Lesson for poe-inspect-2**: If using LLM for item evaluation, validate outputs rigorously — LLMs make subtle domain errors

---

## 5. Frontend Tech Stack

### Stack Details

| Technology | Version | Purpose |
|-----------|---------|---------|
| Preact | 10.28.4 | UI framework (3KB alternative to React) |
| @preact/signals | 2.8.1 | Reactive state management |
| TypeScript | 5.7.0 | Type safety |
| Vite | 6.0.0 | Build tool and dev server |
| Tailwind CSS | 4.0.0 | Utility-first CSS (v4 = CSS-based config) |
| @preact/preset-vite | 2.10.3 | Vite integration for Preact |
| @tailwindcss/vite | 4.0.0 | Tailwind v4 Vite plugin |
| Biome | 2.4.6 | Linter + formatter (replaces ESLint + Prettier) |

### What Can Be Directly Adapted

- **Preact + Signals + Tailwind v4 + Vite + Biome** is a proven, lightweight stack for a PoE tool frontend. poe-inspect-2 can reuse this exact combination.
- **Tailwind v4's CSS-based configuration** (no `tailwind.config.js`) simplifies setup.
- **Biome** as a single tool replacing ESLint + Prettier reduces devDependencies and config files.

### What poe-inspect-2 Needs Differently

- **No chat UI needed** (unless adding an LLM assistant). The overlay likely needs:
  - Item tooltip rendering
  - Value/tier display
  - Settings panel
  - Possibly a floating/overlay window (if using Tauri or similar)
- **WebSocket may still be useful** for real-time communication between a Rust backend (clipboard monitoring, screen capture analysis) and the frontend overlay.

---

## 6. Cargo.toml Crates

### Full Dependency List

**Runtime dependencies:**
| Crate | Version | Feature Flags | Purpose |
|-------|---------|---------------|---------|
| `thiserror` | 2 | — | Derive macro for error types |
| `serde` | 1 | derive | Serialization framework |
| `serde_json` | 1 | — | JSON parsing/generation |
| `axum` | 0.8 | ws | Web framework with WebSocket support |
| `tokio` | 1 | full | Async runtime |
| `tower-http` | 0.6 | cors, trace | HTTP middleware (CORS, request tracing) |
| `uuid` | 1 | v4, serde | UUID generation for session IDs |
| `tracing` | 0.1 | — | Structured logging |
| `tracing-subscriber` | 0.3 | env-filter | Log filtering via RUST_LOG env var |
| `reqwest` | 0.12 | json, stream | HTTP client for data fetching |
| `futures-util` | 0.3 | — | Stream utilities (used with WebSocket) |

**Dev dependencies:**
| Crate | Version | Feature Flags | Purpose |
|-------|---------|---------------|---------|
| `pretty_assertions` | 1 | — | Better assertion diff output in tests |
| `tower` | 0.5 | util | Test utilities for Axum handlers |

### What Can Be Directly Adapted

Nearly all of these crates are applicable to poe-inspect-2:

- **Core**: `serde`, `serde_json`, `thiserror`, `tokio`, `tracing`, `tracing-subscriber` — universal Rust application crates
- **HTTP**: `axum` (if poe-inspect-2 has a web UI), `reqwest` (for data fetching from poe.ninja, repoe-fork)
- **WebSocket**: `axum` with `ws` feature + `futures-util` for streaming communication between backend and frontend overlay

### Additional Crates poe-inspect-2 Likely Needs

- **Clipboard monitoring**: `clipboard-rs` or `arboard` for reading PoE item data from clipboard (Ctrl+C on items)
- **Screen capture / OCR**: If doing overlay-based inspection rather than clipboard parsing
- **Tauri**: If building as a desktop overlay application
- **Image processing**: For parsing item screenshots or overlaying information
- **Hotkey / global input**: For triggering inspection from outside the app window

### Lint Configuration Worth Reusing

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
struct_excessive_bools = "allow"
too_many_lines = "allow"
```

This is a good baseline: pedantic clippy catches real issues, the allows are pragmatic exceptions for game-data code (lots of numeric casts, bool-heavy structs).

---

## 7. CLAUDE.md Conventions

### What the CLAUDE.md Contains

- Project name and one-line description
- Tech stack summary (Rust backend, Preact + TypeScript frontend, LLM provider-agnostic)
- Architecture summary (REST/WebSocket API, typed filter AST, LLM tool-use pattern)
- Key design decisions (swappable LLM provider, structured tool calls not raw text, league-aware data)
- Project directory structure (`/docs/`, `/backend/`, `/frontend/`)
- How to run the backend (`cargo run --bin poe-filter-server`)
- Environment variables table (PORT, OLLAMA_URL, OLLAMA_MODEL, DATA_DIR, LEAGUE, FALLBACK_LEAGUE)
- Data re-fetching instructions
- Development conventions: Rust edition 2024, MSRV 1.85, `cargo fmt` + `cargo clippy`, clippy pedantic, `unsafe_code = "forbid"`, Biome for frontend

### Patterns to Adopt for poe-inspect-2

1. **Start with project identity**: Name, one-line description, what it does
2. **Tech stack as a bullet list**: Quick reference for anyone (human or AI) opening the project
3. **Architecture in 3-4 sentences**: How the pieces connect
4. **Key design decisions**: Non-obvious choices that would be wrong to reverse without discussion
5. **How to run**: Copy-pasteable commands
6. **Environment variables**: Table format with variable, default, and description
7. **Development conventions**: Edition, MSRV, linting, formatting — the things that cause CI failures if violated

### What's Missing (and poe-inspect-2 Should Add)

- **Testing instructions**: How to run tests, what test coverage looks like
- **Contributing guidelines**: PR process, branch naming, commit message format
- **Known limitations**: What doesn't work yet, what's intentionally out of scope
- **Dependency update policy**: How often to update, what to pin vs float

---

## Cross-Cutting Insights

### Architecture Patterns Worth Adopting

1. **`Arc<GameData>` shared state**: Load all reference data once at startup, wrap in Arc, share across handlers. Simple, fast, no runtime locking for reads.

2. **Trait-based provider abstraction**: The LLM integration uses a trait so providers can be swapped. Apply the same pattern to any external dependency in poe-inspect-2 (data sources, display backends, input methods).

3. **Tool-use pattern for LLM integration**: If poe-inspect-2 uses LLM for item evaluation, the `Tool` trait + `ToolRegistry` + `ToolDefinition` with JSON Schema parameters is a clean, tested pattern. The 15 tools in poe-item-filter demonstrate good granularity.

4. **Stable IDs for mutable collections**: `FilterSession`'s monotonically increasing `RuleId` pattern avoids index invalidation. Useful for any collection where external references must survive insertions/deletions.

5. **First-match-wins evaluation with Continue**: The evaluator's control flow (walk blocks top-to-bottom, stop at first match unless Continue is set) is a generally useful pattern for rule-based evaluation systems.

### Data Source Priorities for poe-inspect-2

Based on the research documents:

| Priority | Source | Data | URL Pattern |
|----------|--------|------|-------------|
| 1 | poe.ninja | Economy values (chaos/divine prices) | `https://poe.ninja/api/data/{currencyoverview\|itemoverview}?league={league}&type={type}` |
| 2 | repoe-fork | Base items, item classes, gems | `https://repoe-fork.github.io/{file}.json` |
| 3 | poewiki.net | Mod tiers, affixes, crafting data | `https://www.poewiki.net/api.php?action=cargoquery&tables=...` |
| 4 | poedb.tw | Avoid — no API, scraping-hostile | N/A |

### Key Pitfalls Documented

1. **poe.ninja data sparsity early in leagues**: Use fallback league strategy or graceful degradation
2. **repoe-fork includes unreleased/internal items**: Always filter by `release_state == "released"` and `domain == "item"`
3. **LLMs make subtle domain errors**: Confusing Sockets vs LinkedSockets, ignoring attribute requirements, not understanding drop levels. Any LLM-generated evaluation rules need validation.
4. **PoE filter syntax changes each league**: 1-3 new conditions per league, occasionally major reworks (3.25 socket changes, 3.28 map changes). Build for extensibility.
5. **Currency vs item endpoints on poe.ninja use different JSON schemas**: Must handle both formats in the parser.
6. **Preact 11 was unstable** (as of the research date): Stick with Preact 10.x.
7. **Tailwind v4 uses CSS-based config**: No `tailwind.config.js` — configuration is done in CSS files. This is a significant change from v3.

### Files of Highest Value for Reference

| File | Why |
|------|-----|
| `backend/src/data/models.rs` | Core data structures for items, gems, economy |
| `backend/src/data/economy.rs` | poe.ninja integration with fallback strategy |
| `backend/src/data/parse.rs` | repoe-fork JSON parsing with filtering logic |
| `backend/src/data/mod.rs` | GameData organization and lookup methods |
| `backend/src/evaluate.rs` | Item struct (40+ fields) and matching logic |
| `backend/Cargo.toml` | Proven crate versions and lint configuration |
| `docs/research/data-sources.md` | Comprehensive data source analysis |
| `docs/research/llm-integration.md` | Rust LLM ecosystem evaluation |
| `frontend/package.json` | Proven frontend dependency versions |
