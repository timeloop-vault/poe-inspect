# Crafting Data Sources Research

**Date**: 2026-03-07
**Status**: Research complete
**Purpose**: Identify where deterministic and semi-deterministic crafting knowledge lives in machine-readable formats, and what community tools do with it.

---

## 1. Crafting Bench Data

### Source: RePoE Fork `crafting_bench_options.json`

**URL**: `https://repoe-fork.github.io/poe1/crafting_bench_options.json`

The crafting bench has fixed, deterministic mods. RePoE exports this as `crafting_bench_options.json`.

**Expected data per entry**:
- **Mod ID**: Links to the mod definition in `mods.json` (domain `"crafted"`)
- **Cost**: Currency type and amount (e.g., 1 Exalted Orb, 4 Chaos Orbs)
- **Item class restrictions**: Which item classes the craft applies to (e.g., Body Armours, Helmets, One Hand Swords)
- **Required unlock**: Some bench crafts require unveiling or specific recipes (Pale Court, Delve, etc.)

**What this gives us**:
- Complete list of all benchcraftable mods with costs
- Item class restrictions for filtering ("can I bench craft X on this item?")
- Mod IDs cross-reference to `mods.json` for stat values and ranges

**In `mods.json`**: Bench crafts appear with `domain: "crafted"` (1,585 mods in our prior analysis). These are distinct from rollable mods (`domain: "item"`) and should never appear in tier calculations for rolled mods, but they are critical for crafting advice ("you have an open prefix -- you can bench craft +life").

**Structured data availability**: FULL. This is entirely machine-readable from RePoE.

---

## 2. Harvest Crafting

### Deterministic Harvest Crafts

Harvest crafting underwent major changes across patches. As of 3.25+ (post-Settlers), many harvest crafts are accessed through Horticrafting stations and have been significantly reworked.

**Deterministic harvest crafts include**:
- Reforge with guaranteed mod (e.g., "Reforge a Rare item with a Fire modifier" -- guarantees at least one fire-tagged mod)
- Augment crafts (add a mod of a specific tag) -- these were removed/heavily restricted in 3.19 (Lake of Kalandra) and have not returned as of recent patches
- Targeted annuls (remove a random modifier with a specific tag) -- also removed in 3.19
- Reforge keeping prefixes/suffixes -- deterministic in what they preserve
- Change resistance type (e.g., cold res to fire res)
- Enchant crafts (quality, socket-related)
- Fracture a mod (make one mod permanent) -- semi-deterministic (random which mod gets fractured unless only one eligible)

**Data sources**:
- **No dedicated `harvest.json` in RePoE**. Harvest crafts are not exported as a standalone file in the standard RePoE data export.
- **poewiki.net Cargo API**: The wiki documents harvest craft outcomes, but not in a structured crafting-simulation format.
- **poedb.tw**: Lists harvest crafts with their effects, but no API.
- **Game files**: Harvest craft definitions exist in the game's `.dat` files (accessible via `poe-tool-dev/dat-schema`), but extracting and interpreting them requires specialized tooling.

**Structured data availability**: PARTIAL. The "reforge with guaranteed tag" crafts can be modeled using the `implicit_tags` field in `mods.json` (every mod has tags like `fire`, `cold`, `attack`, `caster`). However, the actual list of available harvest crafts and their exact mechanics is not cleanly exported by RePoE. This is a gap.

---

## 3. Fossil and Essence Crafting

### 3a. Fossils

**Source**: RePoE Fork `fossils.json`

**URL**: `https://repoe-fork.github.io/poe1/fossils.json`

**Expected data per fossil**:
- **Added mod weights**: Fossils modify spawn weights for mods with specific tags. For example, a Scorched Fossil increases the weight of fire-tagged mods and decreases the weight of cold-tagged mods.
- **Blocked mod groups**: Some fossils block entire mod groups (e.g., Pristine Fossil blocks all `Life` group mods from the pool when socketed in a resonator -- wait, this is inverted: Pristine blocks non-life mods. The exact mechanic is that fossils add positive weight multipliers to desired tags and zero-weight multipliers to undesired tags.)
- **Forced mods**: Some fossils guarantee a specific mod (e.g., Faceted Fossil forces a gem-level mod).

**How fossil crafting works mechanically**:
1. Start with the base item's mod pool (filtered by domain, item tags, ilvl)
2. For each fossil in the resonator, apply weight multipliers:
   - Multiply spawn weights for mods matching the fossil's "more" tags
   - Set spawn weights to 0 for mods matching the fossil's "less" (blocked) tags
3. Some fossils add new mods to the pool that are not normally rollable (fossil-only mods)
4. Roll the item using the modified pool

**In `mods.json`**: Fossil-exclusive mods exist but can be identified by name patterns containing "Fossil" or by having spawn weights that are zero for all normal tags but positive for fossil-specific tags. The old project used name-based filtering (`name.contains("fossil")`) which is fragile.

**Structured data availability**: FULL. `fossils.json` provides the weight modification rules. Combined with `mods.json` spawn weights and `base_items.json` tags, fossil crafting outcomes can be fully simulated.

### 3b. Essences

**Source**: RePoE Fork `essences.json`

**URL**: `https://repoe-fork.github.io/poe1/essences.json`

**Expected data per essence**:
- **Forced mod by item class**: Each essence tier forces a specific mod on a specific item class. For example, Deafening Essence of Contempt on a weapon forces a specific flat physical damage mod, while on body armour it forces a different mod.
- **Essence tier**: Screaming, Shrieking, Deafening, etc.
- **Item class mapping**: Maps essence + item class to a specific forced mod ID

**How essence crafting works**:
1. The essence guarantees one specific mod (determined by essence type + item class)
2. Remaining mods are rolled normally from the item's mod pool
3. Essence-only mods (flagged with `is_essence_only: true` in `mods.json`) can ONLY appear via essence crafting, never from chaos/alchemy orbs

**In `mods.json`**: 434 mods have `is_essence_only: true`. An additional ~125 mods have "essence" in their name but are NOT essence-only. The `is_essence_only` field is more reliable than name matching (our old project discovered 11 essence-only mods that lacked "essence" in their name).

**Structured data availability**: FULL. `essences.json` maps essence + item class to forced mod. `mods.json` provides `is_essence_only` flag for pool filtering.

---

## 4. Meta-Crafting Recipes

Meta-crafting refers to strategic combinations of crafting methods to achieve deterministic or near-deterministic outcomes. Examples:

### Common Meta-Craft Patterns

1. **"Prefixes Cannot Be Changed" + Scouring Orb**
   - Bench craft the metamod, then scour: removes all suffixes, preserves all prefixes
   - Cost: 2 Divine Orbs (bench cost) + 1 Scouring Orb

2. **"Suffixes Cannot Be Changed" + Veiled Chaos Orb**
   - Preserves suffixes, rerolls prefixes, adds a veiled prefix
   - Allows targeted unveiling for specific prefix mods

3. **"Cannot Roll Attack Mods" + Exalted Orb**
   - Blocks all attack-tagged mods from the pool, then slams
   - On caster items, this can guarantee specific mods (e.g., +1 to spell gems on weapons)

4. **"Cannot Roll Caster Mods" + Exalted Orb**
   - Similar to above but for attack items

5. **Aisling T4 (Betrayal unveil)**
   - Removes a random mod and adds a veiled mod
   - Combined with metamods, can target specific slots

6. **Harvest "Reforge keeping prefixes/suffixes"**
   - Combined with metamods for further control

7. **Fracture + targeted crafting**
   - Fracture a key mod, then use other methods to fill remaining slots

### Data Source Assessment

**These recipes are NOT in any structured data file.** They are combinations of:
- Bench craft mechanics (from `crafting_bench_options.json`)
- Currency item effects (hardcoded game knowledge)
- Mod tag interactions (from `mods.json` implicit_tags)
- Unveiled mod pools (from `mods.json` domain `"unveiled"`)

**Where this knowledge lives**:
- **Community wikis**: poewiki.net has articles on meta-crafting but not in structured/queryable format
- **Reddit**: r/pathofexile crafting guides (unstructured text)
- **TFT Discord**: Crafting guides channel has detailed guides but no structured data export
- **YouTube/Twitch**: Crafting content creators explain techniques (unstructured)
- **poedb.tw**: Shows mod pools filtered by metamods (e.g., "with Cannot Roll Attack Mods"), which is useful for validation but not an API

**Structured data availability**: NONE in machine-readable format. This is purely community knowledge that would need to be encoded as user/community-configurable rules.

**Implication for our tool**: The HYPOTHESIS.md already anticipated this -- crafting rules should be user/community-configurable, not hardcoded. A rule format like:

```
IF item has [T1 physical damage] AND [open prefix]
AND item_class IN [One Hand Sword, Thrusting One Hand Sword]
THEN suggest: "Bench craft 'Cannot Roll Attack Mods' (cost: 1 Divine) + Exalted Orb → guaranteed +1 to Level of Socketed Gems"
```

This is post-MVP but the data model should support it.

---

## 5. Craft of Exile (craftofexile.com)

### What It Does

Craft of Exile is the premier community crafting simulator for Path of Exile. Features:

- **Mod pool browser**: Select a base item + ilvl, see all rollable mods with spawn weights
- **Crafting simulator**: Simulate chaos spam, fossil crafting, essence crafting, etc. with accurate probabilities
- **Probability calculator**: "What is the chance of hitting T1 life + T1 res on this base?"
- **Fossil optimizer**: "Which fossil combination maximizes the chance of hitting X mod?"
- **Cost estimator**: Expected cost in currency to hit a target mod combination
- **Emulator mode**: Roll items repeatedly to see realistic outcomes

### Does It Have an API?

**No public API.** Craft of Exile does not expose a documented API for external consumption. The site:
- Fetches its own processed game data (derived from RePoE / dat files)
- Processes everything client-side in JavaScript
- Has its own internal data format (not publicly documented)

### How It Models Crafting Outcomes

Craft of Exile uses the same underlying data we have access to:
1. `mods.json` equivalent data for mod pools and spawn weights
2. `base_items.json` equivalent for item tags
3. `fossils.json` equivalent for fossil weight modifiers
4. `essences.json` equivalent for essence forced mods

The key insight is that **the math is well-understood**: mod selection uses weighted random sampling from the eligible mod pool. The probability of hitting a specific mod = (mod's spawn weight) / (sum of all eligible mod spawn weights). Craft of Exile implements this math and presents it visually.

**We can replicate this**: Given `mods.json` + `base_items.json` + `fossils.json` + `essences.json`, we have all the data needed to compute the same probabilities. The algorithms are:
1. Filter mod pool by domain, item tags, ilvl, and any active restrictions (fossils, metamods)
2. Sum spawn weights for all eligible mods
3. Probability of hitting mod X = weight(X) / total_weight
4. For multi-mod targets, multiply independent probabilities (with corrections for mod group exclusion)

**Structured data availability**: We can build equivalent functionality from RePoE data. No need for a Craft of Exile API.

---

## 6. Path of Building Crafting Data

### What PoB Contains

Path of Building Community Fork (https://github.com/PathOfBuildingCommunity/PathOfBuilding) contains crafting-related data, primarily for item editing in the PoB item creator:

- **Mod data**: PoB maintains its own processed mod lists (in Lua table format) derived from game data
- **Base item data**: Item bases with their properties
- **Crafting bench data**: Bench craft options for the item editor

### Data Format

PoB stores data in Lua table format (not JSON), within files like:
- `src/Data/ModCache.lua` -- cached mod definitions
- `src/Data/Bases/` -- base item definitions by category
- `src/Data/CraftingBench.lua` -- bench craft options

### Can We Reuse It?

**Not directly.** PoB's data is:
1. In Lua format (would need conversion)
2. Processed/simplified for PoB's specific needs (build simulation, not crafting simulation)
3. Updated on PoB's release schedule (not necessarily in sync with game patches)

**Better approach**: Use the same source data (RePoE / dat-schema) that PoB itself derives from. This avoids a dependency on PoB's data format and update schedule.

**One useful PoB resource**: PoB's item editor validates mod combinations, which could be used as a reference implementation for testing our mod pool filtering logic.

---

## 7. Community Craft Guides / Databases

### Structured Repositories of Crafting Recipes

**There are no widely-adopted structured repositories of crafting "recipes" in machine-readable format.** The closest things are:

1. **poewiki.net crafting guides**: Written in wikitext, human-readable but not machine-parseable as recipes. The wiki does have a Cargo database with some structured mod/item data, but not crafting procedures.

2. **TFT Discord**: The "Forbidden Trove" Discord has crafting guide channels with detailed step-by-step guides for specific items. These are text posts with images, not structured data. TFT has no public API or data export.

3. **Reddit r/pathofexile**: Crafting guides posted as text/image posts. Completely unstructured.

4. **poedb.tw**: Shows mod pools and can filter by crafting method, but does not document multi-step crafting procedures. No API (scraping is technically possible but against their ToS intent).

5. **YouTube/Twitch crafters**: Content creators like Elesshar, SubRacist, etc. produce crafting guides that are essentially video walkthroughs. No structured data.

6. **maxroll.gg**: Has some PoE crafting guides with step-by-step instructions, but these are editorial content, not structured data.

### Gap Analysis

This is the biggest gap in the crafting data ecosystem. The community has deep knowledge of:
- "If you want +2 gems amulet, do X then Y then Z"
- "The cheapest way to get T1 life + T1 res on this base is..."
- "For this build, the optimal crafting sequence is..."

But none of this is in a format that a tool can consume. This confirms the HYPOTHESIS.md approach: **crafting rules must be user/community-configurable** with a shareable format (JSON/YAML recipe files, community repository).

---

## 8. Mod Pool and Spawn Weight Data

### What `mods.json` Provides (Verified from Prior Work)

The repoe-fork `mods.json` file contains **37,339 total mods** with the following fields per mod:

| Field | Description | Relevance to Crafting |
|-------|-------------|----------------------|
| `name` | Mod name (e.g., "of the Brute") | Display |
| `text` | Display text (e.g., "+(8-12) to Strength") | Display |
| `domain` | Item type scope: `item`, `crafted`, `flask`, `abyss_jewel`, `unveiled`, etc. | **Critical filtering** |
| `generation_type` | `prefix`, `suffix`, `unique`, `corrupted`, etc. | **Critical filtering** |
| `required_level` | Minimum ilvl for this mod to be eligible | **ilvl filtering** |
| `spawn_weights` | Array of `{tag, weight}` pairs | **Core mechanic for mod pool** |
| `groups` | Mod group names (exclusion system: only one mod per group) | **Mod conflict detection** |
| `stats` | Array of `{id, min, max}` | **Value ranges, tier identification** |
| `is_essence_only` | Boolean flag | **Pool filtering** |
| `generation_weights` | Conditional spawning (requires other mods present) | **Advanced pool filtering** |
| `adds_tags` | Tags this mod adds to the item (enables other mods) | **Mod dependency chains** |
| `implicit_tags` | Categorization tags (`fire`, `cold`, `attack`, `caster`, etc.) | **Harvest/metamod filtering** |

### How Spawn Weights Determine the Mod Pool

For a given item (e.g., ilvl 85 Astral Plate):

1. **Get base item tags** from `base_items.json`: `["body_armour", "str_armour", "armour"]`
2. **Filter mods** from `mods.json`:
   - `domain == "item"` (not crafted, flask, jewel, etc.)
   - `generation_type == "prefix"` or `"suffix"`
   - `required_level <= 85`
   - `is_essence_only == false`
   - Spawn weight > 0 for at least one of the item's tags
3. **Calculate effective weight** for each eligible mod:
   - Walk the `spawn_weights` array in order
   - First matching tag determines the weight
   - Last entry is typically `{"tag": "default", "weight": 0}`

### How Influences Modify the Pool

Influenced items (Shaper, Elder, Crusader, Hunter, Redeemer, Warlord) add tags to the item:
- Shaper: adds `shaper` tag
- Elder: adds `elder` tag
- etc.

Influence-specific mods in `mods.json` have spawn weights like:
```json
{"tag": "shaper", "weight": 800}
```
These mods have weight 0 for all non-influence tags, so they only appear in the pool when the item has the corresponding influence tag.

**Data availability**: FULL. The influence tag system is entirely modeled by spawn weights in `mods.json` + item tags in `base_items.json`.

### How Fossils Modify the Pool

Fossils from `fossils.json` provide weight multipliers:
```json
{
  "added_mods": ["FossilSpecificMod1", "FossilSpecificMod2"],
  "positive_mod_weights": [{"tag": "fire", "weight": 2000}],
  "negative_mod_weights": [{"tag": "cold", "weight": 0}]
}
```

To compute the fossil-modified pool:
1. Start with the normal mod pool
2. For each mod, check if its `implicit_tags` match any fossil weight modifier
3. Apply multipliers (positive = increased weight, negative = reduced or zeroed weight)
4. Add any fossil-exclusive mods (`added_mods`) to the pool

**Data availability**: FULL from `fossils.json` + `mods.json`.

### How Essences Modify the Pool

Essences force one specific mod (bypassing normal rolling for that slot) and fill remaining slots normally. The forced mod comes from `essences.json` which maps essence type + item class to a mod ID.

**Data availability**: FULL from `essences.json` + `mods.json`.

---

## Summary: Structured vs. Community Knowledge

| Data Category | Structured Data Source | Machine-Readable? | Completeness |
|--------------|----------------------|-------------------|-------------|
| **Mod pools & spawn weights** | `mods.json` (RePoE) | Yes | Complete |
| **Base item tags** | `base_items.json` (RePoE) | Yes | Complete |
| **Stat translations** | `stat_translations.json` (RePoE) | Yes | Complete |
| **Crafting bench options** | `crafting_bench_options.json` (RePoE) | Yes | Complete |
| **Fossil weight modifiers** | `fossils.json` (RePoE) | Yes | Complete |
| **Essence forced mods** | `essences.json` (RePoE) | Yes | Complete |
| **Influence mod pools** | `mods.json` spawn weights (RePoE) | Yes | Complete |
| **Unveiled mod pool** | `mods.json` domain `"unveiled"` (RePoE) | Yes | Complete |
| **Harvest craft list** | Game dat files / wiki | Partial | Partial -- needs extraction |
| **Harvest craft mechanics** | Community knowledge | No | Incomplete |
| **Meta-craft recipes** | Community knowledge only | No | None |
| **Multi-step craft procedures** | Community knowledge only | No | None |
| **Crafting cost estimates** | Derivable from above + economy data | Computable | N/A |
| **Crafting probability math** | Well-understood algorithm | Implementable | N/A |

### What We Can Automate

1. **"What can I craft on this item?"** -- Fully automatable from bench craft data + open affix detection
2. **"What mods can roll on this base?"** -- Fully automatable from mods.json + base_items.json
3. **"What are the odds of hitting X?"** -- Fully automatable (weighted probability math)
4. **"What fossils optimize for X?"** -- Fully automatable from fossils.json
5. **"What essence gives X on this base?"** -- Fully automatable from essences.json

### What Needs User/Community Configuration

1. **Multi-step crafting recipes** (meta-crafts, combined techniques)
2. **"Best way to craft X" guides** (procedural knowledge)
3. **Crafting strategies** (when to use which method, cost-benefit analysis)
4. **Harvest craft availability** (which crafts are currently in the game and how to access them)

### Recommended Approach for poe-inspect-2

**Phase 1 (MVP)**: Use `mods.json` + `base_items.json` for:
- Open prefix/suffix detection
- Bench craft suggestions (from `crafting_bench_options.json`)
- "This item has T1 life and an open prefix" style insights

**Phase 2**: Add probability calculations:
- Download and integrate `fossils.json`, `essences.json`
- "Chance of hitting T1 life with chaos spam on this base: X%"
- "Scorched Fossil increases fire mod weight by Y%"

**Phase 3**: Community-configurable craft rules:
- Define a JSON/YAML format for multi-step craft recipes
- Allow import/export of recipe collections
- Community repository for sharing recipes
- "This item matches craft recipe 'Budget +2 Amulet' -- estimated cost: 3 Divine Orbs"

### Key Data Files to Download

From `https://repoe-fork.github.io/poe1/`:

| File | Size (approx) | Purpose |
|------|---------------|---------|
| `mods.json` | ~20 MB | All mod definitions, spawn weights, tiers |
| `base_items.json` | ~500 KB | Item bases, tags, properties |
| `stat_translations.json` | ~11 MB | Mod text to stat ID mapping |
| `stats.json` | ~2 MB | Stat definitions |
| `crafting_bench_options.json` | ~200 KB | Bench craft definitions |
| `essences.json` | ~100 KB | Essence forced mod mappings |
| `fossils.json` | ~50 KB | Fossil weight modifiers |

All available at predictable URLs: `https://repoe-fork.github.io/poe1/{filename}.json`
