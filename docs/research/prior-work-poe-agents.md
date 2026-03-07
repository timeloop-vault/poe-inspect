# Prior Work: poe-agents Repository

Reference location: `_reference/poe-agents/`

A Claude Code agent system for PoE 1 build planning. Uses Path of Building as a calculation engine via a Lua bridge, poe.ninja for economy data, and multi-agent orchestration for build advisory. Studied for patterns, API details, and reusable components relevant to poe-inspect-2.

---

## 1. poe.ninja Integration (`tools/poe_ninja.py`)

### API Details

- **Base URL:** `https://poe.ninja/api/data/`
- **Rate limit:** 12 requests per 5 minutes (~1 request per 25 seconds). The client uses a conservative 5-second minimum interval between requests.
- **User-Agent:** `poe-agents/0.1` (custom, not browser-spoofing).
- **No authentication required** -- fully public API.

### Endpoints

**Currency overview:**
```
GET /currencyoverview?league={LEAGUE}&type={TYPE}
```
Types: `Currency`, `Fragment`

Response shape: `{ lines: [{ currencyTypeName, chaosEquivalent, pay, receive, sparkLine }], currencyDetails: [...] }`

**Item overview:**
```
GET /itemoverview?league={LEAGUE}&type={TYPE}
```
Types (comprehensive list):
- Currency/crafting: `Oil`, `Incubator`, `Scarab`, `Fossil`, `Resonator`, `Essence`, `DeliriumOrb`, `Omen`
- Cards: `DivinationCard`
- Gems: `SkillGem`
- Base items: `BaseType`
- Maps: `Map`, `UniqueMap`, `BlightedMap`, `BlightRavagedMap`
- Uniques: `UniqueWeapon`, `UniqueArmour`, `UniqueAccessory`, `UniqueFlask`, `UniqueJewel`, `UniqueRelic`
- Jewels: `ClusterJewel`
- Other: `Beast`, `Vial`, `Invitation`, `Memory`, `Coffin`, `AllflameEmber`

Response shape: `{ lines: [{ id, name, baseType, chaosValue, divineValue, listingCount, links, variant, sparkline, explicitModifiers, implicitModifiers, corrupted, ... }] }`

**Price history:**
```
GET /CurrencyHistory?league={LEAGUE}&type={TYPE}&currencyId={ID}
GET /ItemHistory?league={LEAGUE}&type={TYPE}&itemId={ID}
```

**Builds overview:**
```
GET /data/{snapshotId}/getbuildoverview?overview={LEAGUE_SLUG}&type={SORT}&language=en
```
- `snapshotId` is a cache-buster (any random string works)
- `overview` uses lowercase league slug (e.g., `settlers`, `settlershc`)
- `type`: `exp` (ladder rank) or `depth` (delve depth)
- Optional `timemachine` param for historical snapshots: `day-1`, `day-5`, `week-10`
- Response contains aggregate stats: class distribution, active skill usage, item popularity, keystone usage

### League Name Format (Gotcha)

Two different formats depending on endpoint:
- **Economy endpoints** (`league=`): Short display name, first word. "Settlers of Kalguur" = `Settlers`, "Keepers of the Flame" = `Keepers`.
- **Builds endpoint** (`overview=`): Lowercase slug. `settlers`, `settlershc`.

The builds API slug format may have changed with poe.ninja's SPA rewrite -- documented as a TODO/gotcha.

### Throttling Implementation

Simple global timestamp approach:
```python
_last_request_time = 0.0
_MIN_INTERVAL = 5.0

def _throttled_get(url):
    global _last_request_time
    elapsed = time.time() - _last_request_time
    if elapsed < _MIN_INTERVAL:
        time.sleep(_MIN_INTERVAL - elapsed)
    # ... make request ...
    _last_request_time = time.time()
```

### Reusability for poe-inspect-2

**Directly reusable:**
- The full list of item type constants (`ECONOMY_ITEM_TYPES`, `CURRENCY_TYPES`) is a canonical reference.
- The API URL structure and response shapes are well-documented.
- Price lookup pattern: iterate through `UniqueWeapon`, `UniqueArmour`, `UniqueAccessory`, `UniqueFlask`, `UniqueJewel` to find a unique by name.
- The throttling approach is simple and effective.

**Adaptation needed:**
- poe-inspect-2 will need a Rust/TypeScript client rather than Python.
- For an overlay, we need background caching rather than on-demand fetching. Pre-fetch relevant item type categories and cache locally.
- The 5-second throttle is conservative. The actual limit is 12/5min = one per 25 seconds. For bulk pre-fetching at startup, we could batch requests with 25-second intervals and cache aggressively.

### Reference Sources for poe.ninja API

Documented in `docs/research/poe-ninja-api.md`:
- [5k-mirrors/misc-poe-tools](https://github.com/5k-mirrors/misc-poe-tools/blob/master/doc/poe-ninja-api.md)
- [ayberkgezer/poe.ninja-API-Document](https://github.com/ayberkgezer/poe.ninja-API-Document)
- [Davenads/poeninjaAPI-2025](https://github.com/Davenads/poeninjaAPI-2025)
- [moepmoep12/poe-api-ts](https://github.com/moepmoep12/poe-api-ts)

---

## 2. Path of Building Integration

### PoB Build Code Codec (`tools/pob_codec.py`)

Build share codes (the strings pasted into PoB's import dialog) use this encoding:

```
base64url(deflate(xml_content))
```

Specifics:
- URL-safe base64: `+` replaced with `-`, `/` replaced with `_`, padding `=` stripped.
- Compression: raw deflate (no zlib header) OR zlib-wrapped format. Auto-detection: if first byte is `0x78`, it's zlib-wrapped; otherwise raw deflate.
- Decoding uses `zlib.decompress(data, -zlib.MAX_WBITS)` for raw deflate.
- Encoding uses `compressobj(level, DEFLATED, -MAX_WBITS)` for raw deflate, then URL-safe base64.

**Reusability for poe-inspect-2:** If we ever need to import/export PoB codes (e.g., user pastes a PoB code to define their build profile), this codec is trivially portable to any language with zlib support.

### PoB Lua Bridge (`tools/pob_bridge.py`)

Architecture:
```
Python process
  |
  spawns LuaJIT subprocess (HeadlessWrapper.lua)
  |
  communicates via newline-delimited JSON over stdio
  |
  PoB calculation engine runs in memory
```

Protocol:
- Requests: `{ "action": "get_stats", "params": { ... } }`
- Responses: `{ "ok": true, "stats": { ... } }` or `{ "ok": false, "error": "..." }`
- Startup: LuaJIT emits `{"ready": true}` when initialized.
- One request at a time (no concurrency).
- 30-second default timeout.
- Uses `ianderse/PathOfBuilding` fork (branch `api-stdio`) with custom API layer.

Available operations:
- `load_build_xml(xml, name)` -- load from XML string
- `get_stats(fields)` -- calculated build stats (DPS, life, resists, etc.)
- `get_build_info()` -- metadata (name, level, class, ascendancy)
- `get_skills()` -- gem/skill configuration
- `get_items()` -- equipped items
- `get_tree()` / `set_tree()` -- passive tree state
- `get_config()` / `set_config()` -- build configuration (boss type, flask states)
- `export_build_xml()` -- export modified build
- `add_item(text)` -- add item from PoE clipboard format
- `search_nodes(query)` -- search passive tree nodes

### Key PoB Stats Available

**Offense:** TotalDPS, HitDPS, TotalDot, BleedDPS, PoisonDPS, IgniteDPS, CritChance, CritMultiplier, HitChance, AttackRate, CastRate, Speed

**Defense:** Life, Mana, EnergyShield, Ward, TotalEHP, PhysMaxHit, Armour, Evasion, FireResist, ColdResist, LightningResist, ChaosResist, BlockChance, SpellBlockChance, SpellSuppressionChance

**Attributes:** Str, Dex, Int

### PoB XML Schema

Root element: `<PathOfBuilding>` containing:
- `<Build>` -- level, targetVersion, mainSocketGroup, bandit, pantheon
- `<Spec>` -- passive tree: treeVersion, classId, ascendClassId, comma-separated node hashes, mastery effects
- `<Skills>` -- gem links grouped by `<SkillSet>` and `<Skill>` (socket groups with slot assignments)
- `<Items>` -- gear in raw PoE clipboard format (same as Ctrl+C in game), with `<Slot>` assignments
- `<Config>` -- key-value pairs (boolean, number, string) matching ConfigOptions.lua
- `<Notes>` -- free text

Slot names: `"Weapon 1"`, `"Weapon 2"`, `"Body Armour"`, `"Helmet"`, `"Gloves"`, `"Boots"`, `"Amulet"`, `"Ring 1"`, `"Ring 2"`, `"Belt"`, `"Flask 1-5"`, `"Jewel 1-N"`

### Reusability for poe-inspect-2

**Build-derived evaluation profiles:** The PoB bridge approach could power "evaluate items for my build" features:
1. User provides a PoB code.
2. We decode it to XML, extract the build's class, ascendancy, skill gems, and current gear.
3. From this, we derive what stats the build values (e.g., a crit build values crit chance/multi; a DoT build values DoT multi).
4. This becomes the evaluation profile for the overlay's item scoring.

**Practical consideration:** Running LuaJIT as a subprocess from a Tauri app is feasible but adds a dependency. A lighter approach for poe-inspect-2 would be to parse the PoB XML directly (the schema is well-documented) and extract build intent without running the full calc engine. We only need to know *what stats matter*, not compute exact DPS.

**PoB code import flow:** Decode PoB code -> parse XML -> extract gems (to identify build archetype) -> extract current gear (to identify upgrade needs) -> derive stat weights.

### pob-mcp Reference

An existing MCP server (TypeScript, by `ianderse`) that wraps PoB. Key insights:
- 44 tools total, 8 XML-only (no deps), 36+ requiring LuaJIT + custom PoB fork.
- **Known limitations:** tool gating (27/44 tools require explicit "continue" prompt), response truncation at 8000 chars, Lua bridge instability, single-request-at-a-time, dormant since Nov 2025.
- Other community projects: `coldino/pob_wrapper` (Python, simpler), `hsource/pobfrontend` (C++/Qt5 cross-platform).

---

## 3. Patch Notes Parser (`tools/patch_notes.py`)

### How It Works

1. **Fetch:** `curl` to download the PoE forum thread HTML.
2. **Extract:** Regex to find the first `<div class="content">` (post body).
3. **HTML-to-text:** Simple regex-based conversion: `<br>` to newline, `<h1-4>` to `## `, `<li>` to `- `, strip remaining tags, decode HTML entities.
4. **Section parsing:** Split on `## ` headings into named sections.
5. **Cache:** JSON file at `data/patch_notes.json` with URL, title, raw text, and sections dict.

### Skill Change Classification

The `classify_change()` function is notable -- it classifies each change line as BUFF, NERF, MIXED, REWORK, or CHANGE:

1. **Numeric comparison (primary):** Parses `X (previously Y)` and `X%-Y% (previously X2%-Y2%)` range patterns. Compares midpoints of new vs old ranges. Understands "less damage" inversion (reducing a penalty = buff).
2. **Signal words (fallback):** Scans for buff signals ("more damage", "increased", "now has", "faster") and nerf signals ("reduced", "decreased", "no longer", "removed").
3. **Priority:** Numeric comparison wins over signal words.

### Reusability for poe-inspect-2

**Limited direct relevance** -- poe-inspect-2 is an item evaluation overlay, not a patch notes analyzer. However, the pattern of fetching + parsing + caching game data from web sources is broadly applicable. The HTML-to-text approach is useful if we ever need to scrape the wiki or forum.

---

## 4. GGG Character API (`tools/poe_character.py`)

A tool not in the original research scope but highly relevant:

### API Details

- **Base URL:** `https://www.pathofexile.com/character-window/`
- **No authentication needed** -- but profile Characters tab must be set to public.
- **Rate limit:** 2-second delay between calls (conservative).
- **User-Agent:** Browser-spoofing string required.

### Endpoints

```
GET /get-characters?accountName={ACCOUNT}&realm=pc    -- list all characters
GET /get-items?accountName={ACCOUNT}&character={CHAR}&realm=pc    -- equipped items
GET /get-passive-skills?accountName={ACCOUNT}&character={CHAR}&realm=pc    -- passive tree
```

Account names with `#` need URL encoding (`#` -> `%23`).

### Response Shapes

- Characters list: `[{ name, class, level, league, ... }]`
- Items: `{ character: { class, level, league }, items: [{ inventoryId, name, typeLine, frameType, sockets, socketedItems, implicitMods, explicitMods, enchantMods, ... }] }`
- Passives: `{ hashes: [nodeId, ...], masteryEffects: [nodeId, effectId, ...] }`

### Item Data Format

- `frameType`: 0=Normal, 1=Magic, 2=Rare, 3=Unique
- `sockets`: array of `{ group, attr }` where attr is S(tr)=R, D(ex)=G, I(nt)=B, G(lobal)=W
- `socketedItems`: gems with `{ typeLine, properties, support }` where properties contains Level
- `inventoryId` maps to slot: `Helm`, `BodyArmour`, `Gloves`, `Boots`, `Weapon`, `Offhand`, `Amulet`, `Ring`, `Ring2`, `Belt`, `Flask`
- Item name has markup `<<set:MS>><<set:M>><<set:S>>` that needs stripping.

### Reusability for poe-inspect-2

**Directly relevant:** This API could provide the "current character" context for item evaluation. If we know what the user's character has equipped, we can evaluate dropped items against current gear. The endpoint structure and gotchas (account name encoding, public profile requirement, name markup stripping) are documented.

---

## 5. Agent Architecture (`.claude/agents/`)

### Agent Roster

| Agent | Role | Key Pattern |
|-------|------|-------------|
| **build-advisor** | Orchestrator, user-facing | Routes to specialists, synthesizes results, never does research itself |
| **meta-scout** | Data researcher | poe.ninja + patch notes + web search |
| **pob-analyst** | Build calculator | PoB Lua bridge for numerical analysis |
| **gear-planner** | Gear strategist | Economy-aware upgrade recommendations |

### Domain Knowledge Patterns

**build-advisor** encodes:
- User intent classification: new league start vs mid-league reroll vs optimize existing vs explore a skill.
- Key clarification questions: SC/HC, budget, playstyle (mapper/bosser/all-rounder), class/skill preference.
- Build recommendation format: key stats, core items with prices, budget estimate, leveling plan, PoB code.
- Team lifecycle: create -> spawn agents -> create tasks -> monitor -> synthesize -> iterate -> shutdown.

**meta-scout** encodes:
- Data freshness awareness: end-of-league prices are inflated vs league start; economy data doesn't exist for unreleased leagues.
- Cross-referencing: patch note buffs/nerfs against economy data and community recommendations.
- Output structure: always cite source, include concrete numbers, provide actionable insights.

**pob-analyst** encodes:
- Stat categorization: offense vs defense vs attributes, with specific stat keys.
- Config awareness: default PoB config often has unrealistic settings; always note boss type when reporting DPS.
- Gotchas: DPS=0 if no weapon or no active skill; stats with value 0 or None are not returned.

**gear-planner** encodes:
- Budget tiering: league start (0-5 div), mid budget (5-30 div), endgame (30+ div).
- Upgrade prioritization metrics: DPS-per-divine, EHP-per-divine, QoL.
- Crafting vs buying analysis framework.
- Early-league vs late-league price awareness.

### Reusability for poe-inspect-2

The domain knowledge patterns are valuable references for item evaluation logic:
- **Budget tiering** informs how we weight items differently at different progression stages.
- **Stat categorization** (offense/defense/attributes with specific stat keys) maps directly to evaluation criteria.
- **Build archetype awareness** (what stats each archetype values) is exactly what we need for build-specific evaluation profiles.

---

## 6. Additional Tools

### Wiki Scrapers

**Unique item scraper** (`tools/unique_scraper.py`): Scrapes PoE wiki pages for all unique items by slot. Handles complex HTML parsing (wiki tables, item-box divs), deduplication of wiki's doubled item names, mod extraction. Caches to JSON + markdown.

**Eldritch implicit scraper** (`tools/eldritch_scraper.py`): Scrapes Searing Exarch and Eater of Worlds implicit modifiers from the wiki. Parses spawn weights to determine which slots each mod applies to. Groups by modifier with tier breakdown.

### Reusability for poe-inspect-2

The scrapers demonstrate how to extract game data from the wiki (poewiki.net). The HTML parsing patterns (handling nested tables, display:none elements, wiki-specific markup) could be adapted if we need to scrape mod pool data or crafting information that isn't available through APIs.

---

## 7. Key Gotchas and Lessons Learned

### API Gotchas

1. **poe.ninja league name format differs by endpoint.** Economy uses short display name (`Keepers`), builds uses lowercase slug (`keepers`).
2. **poe.ninja builds API slug may have changed** with their SPA rewrite. Documented as unresolved.
3. **GGG character API requires public profile.** Returns 403 if Characters tab is private. No workaround without OAuth.
4. **GGG item names contain markup** (`<<set:MS>><<set:M>><<set:S>>`) that must be stripped.
5. **PoB codes can use either raw deflate or zlib-wrapped format.** Must auto-detect by checking first byte (0x78 = zlib header).

### Architecture Gotchas

1. **PoB Lua bridge is single-threaded.** One request at a time, no parallel calls.
2. **PoB startup takes 3-5 seconds** (loading passive tree data). Use persistent connections.
3. **PoB HeadlessWrapper stubs Inflate/Deflate.** Must pre-decode PoB codes in the host language before passing XML to Lua.
4. **pob-mcp is dormant** (no activity since Nov 2025, requires custom PoB fork). Not reliable as a dependency.

### Design Patterns Worth Adopting

1. **Aggressive local caching** -- poe.ninja data, patch notes, wiki scrapes all cached to JSON files. For an overlay, this is critical to avoid API calls during gameplay.
2. **CLI-first tool design** -- each tool is a standalone CLI script with subcommands. Easy to test, debug, and compose. Even if poe-inspect-2 is a GUI overlay, the backend services should be independently testable.
3. **Throttling as a first-class concern** -- every external API call goes through a throttle layer. For an overlay making background requests, this prevents rate limiting.
4. **Separating data gathering from intelligence** -- meta-scout gathers data, build-advisor interprets it. In poe-inspect-2, the pricing/data layer should be cleanly separated from the evaluation/scoring layer.

---

## 8. Summary: What to Carry Forward to poe-inspect-2

### Directly Reusable
- poe.ninja API endpoint catalog, item type constants, response shapes, and rate limiting details.
- PoB build code encoding/decoding logic (trivially portable).
- PoB XML schema documentation (for extracting build intent from PoB codes).
- GGG character API endpoints and response formats.
- Stat categorization (offense/defense/attributes with specific stat key names).

### Patterns to Adopt
- Local caching of all external data with timestamps.
- Throttled API access as a default.
- Budget-tiered evaluation (league start / mid / endgame contexts affect item value).
- Build archetype -> stat weight mappings as the core of evaluation profiles.

### Things to Avoid
- Depending on LuaJIT/PoB as a runtime dependency for an overlay (too heavy; parse XML directly instead).
- Depending on dormant community projects (pob-mcp) as critical infrastructure.
- Making synchronous API calls during overlay rendering.
