# Game Data Browser — Design & Plan

An item-centric game data explorer built on top of our existing data pipeline. Think poedb.tw but local, fast, integrated — and designed around the question: **"What can I do with this item?"**

## Vision

The browser is **item-centric, not page-centric**. You don't navigate to a "Helmets" page or a "Mods" page. You start with an item — search for a base type, pick one from a list, or send an inspected item from the overlay — and then explore everything about it: what mods can roll on it, what changes with influence, what the corruption outcomes are, what essences guarantee, what fossils boost.

The same applies to non-equipment entities: search for a gem and see its level progression, tags, support compatibility. Search for a scarab and see its effect. Search for a divination card and see its reward.

**Core principle**: Start from any entity → explore outward through relationships → connect to other tools (trade, eval, rqe).

## UX Flow

### Equipment Flow (the primary use case)

```
Search: "Vaal Regalia"
    │
    ▼
[Base Type View]
  Name: Vaal Regalia
  Item Class: Body Armours
  Drop Level: 68
  Base ES: 175-201
  Requirements: 194 Int
  Implicit: (none)
  Tags: body_armour, str_armour, int_armour, ...
  ┌─────────────────────────────────────────┐
  │ [Normal] [Magic] [Rare ●] [Unique]     │
  │ [□ Shaper] [□ Elder] [□ Crusader] ...  │
  │ [□ Synthesised] [□ Fractured]           │
  │                                         │
  │ ilvl: [86 ─────────────────── slider]   │
  └─────────────────────────────────────────┘
    │
    ▼
[Mod Pool Panel]
  Prefixes (12 available)                  Suffixes (15 available)
  ────────────────────────                 ────────────────────────
  T1 Tyrannical  169% incr Phys (ilvl 83)  T1 of the Gods  +55 Str (ilvl 82)
  T2 Merciless   ...                        T2 of Puissance  ...
  ...                                       ...

  [+ Filter by fossil] [+ Filter by catalyst] [+ Filter by essence]
  [Crafting bench mods] [Corruption implicits] [Enchantments]
```

### Interactive Building

```
[Mod Pool Panel]
  Click a mod → it's "slotted" onto your virtual item
  ┌──────────────────────────────────────────────┐
  │ Your Item (Rare Vaal Regalia, ilvl 86)       │
  │                                              │
  │ Prefix 1: T1 +120 max Life                  │
  │ Prefix 2: T1 169% incr Phys Dmg             │
  │ Prefix 3: (empty — click to add)             │
  │ Suffix 1: T1 +46% Cold Resistance           │
  │ Suffix 2: T1 +46% Lightning Resistance      │
  │ Suffix 3: (empty — click to add)             │
  │                                              │
  │ Open prefixes: 1 / 3                         │
  │ Open suffixes: 1 / 3                         │
  │                                              │
  │ [Send to Trade] [Save to Eval Rule]          │
  │ [Export to RQE Demand]                        │
  └──────────────────────────────────────────────┘
```

Mod pool updates live: after slotting "T1 +120 max Life", other life prefix mods are dimmed (same mod family). After filling 3 prefixes, remaining prefixes are greyed out.

### From Overlay → Browser

```
[Overlay shows inspected item]
  "Agony Coil — Vaal Regalia"
  +89 to maximum Life
  +42% Fire Resistance
  ... (3 prefixes, 2 suffixes filled)
    │
    │  [Open in Browser]
    ▼
[Browser loads the item]
  - Base type pre-selected: Vaal Regalia
  - Rarity pre-selected: Rare
  - Existing mods shown as "slotted"
  - Open slots highlighted
  - Mod pool filtered to what CAN still roll
  - User can add/remove mods to explore "what if"
```

### Non-Equipment Flows

**Gems:**
```
Search: "Spark" → level progression table, tags, quality effects,
  compatible supports, quest rewards, Vaal variant link, transfigured variants
```

**Currency/Scarabs:**
```
Search: "Gilded Ambush Scarab" → effect description, drop restrictions,
  related scarab tiers (Rusted → Polished → Gilded → Winged)
```

**Divination Cards:**
```
Search: "The Doctor" → reward (Headhunter), stack size, drop areas,
  link to Headhunter base type view
```

**Cluster Jewels:**
```
Search: "Large Cluster Jewel" → enchantment types, then drill into
  enchantment → see available notable passives with their stats
```

**Jewels (all types):**
```
Search: "Cobalt Jewel" → Rare selected → full prefix/suffix pool
  for jewels (much smaller than equipment), max 2/2 mods (not 3/3)
  Abyss Jewels: separate mod pool (abyss_ domain)
  Cluster Jewels: enchantment-driven (see above)
  Timeless Jewels: keystones + seed mechanics
```

**Essences:**
```
Search: "Deafening Essence of Hatred" → guaranteed mod per item class,
  show the full essence tier progression (Whispering → Deafening)
```

**Maps:**
```
Search: "Crimson Temple" → tier, boss, connected maps, div card drops,
  available map mods
```

## Architecture

### Where It Lives

```
app/
  src/
    components/
      browser/          ← New browser UI components
        BrowserPanel.tsx
        BaseTypeView.tsx
        ModPoolExplorer.tsx
        VirtualItem.tsx
        GemView.tsx
        CurrencyView.tsx
        SearchBar.tsx
        ...
  src-tauri/
    src/
      commands/
        browser.rs      ← New Tauri commands for browser data
```

- **Separate window** in Tauri, not the overlay — this is a reference tool with full-size UI
- **Same `Arc<GameData>`** shared with overlay — one data load, two consumers
- **Search-first UX** — global search bar at top, type anything, instant results
- **React + TypeScript frontend** — same stack as the overlay

### Data Flow

```
[GameData (Rust)]
    │
    ├── Tauri commands (browser_*)
    │   ├── browser_search(query) → Vec<SearchResult>
    │   ├── browser_base_type(name) → BaseTypeDetail
    │   ├── browser_mod_pool(item_class, ilvl, influences, ...) → ModPoolResult
    │   ├── browser_gem(name) → GemDetail
    │   ├── browser_essence_table() → EssenceTable
    │   ├── browser_cluster_enchantments(size) → Vec<Enchantment>
    │   └── ...
    │
    ▼
[Browser UI (TypeScript)]
    │
    ├── SearchBar → dispatches to typed views
    ├── BaseTypeView → ModPoolExplorer → VirtualItem
    ├── GemView → level table, supports, tags
    ├── CurrencyView → effect, tiers
    └── ...
```

### The Virtual Item

The "virtual item" is the in-browser representation of an item being built/explored. It mirrors `ResolvedItem` from poe-item but is constructed manually (not parsed from clipboard text):

```rust
/// A virtual item being explored in the browser.
/// Not parsed from text — built interactively by the user.
struct VirtualItem {
    base_type: String,          // e.g., "Vaal Regalia"
    item_class: String,         // e.g., "BodyArmour"
    rarity: Rarity,
    item_level: u32,
    influences: Vec<InfluenceKind>,
    corrupted: bool,
    fractured: bool,
    synthesised: bool,
    slotted_mods: Vec<SlottedMod>,  // user-selected mods
}

struct SlottedMod {
    mod_id: String,             // e.g., "IncreasedLife8"
    slot: ModSlot,              // Prefix / Suffix
    values: Vec<i32>,           // user can pick specific roll values
    locked: bool,               // fractured / cannot be changed
}
```

When an inspected `ResolvedItem` is "opened in browser", it converts to a `VirtualItem` with the inspected mods pre-slotted.

### Integration Points

| Tool | How Browser Connects |
|------|---------------------|
| **poe-trade** | "Send to Trade" builds a trade query from the virtual item's mods. Pre-fills the trade panel filters. |
| **poe-eval** | "Save as Eval Rule" converts the virtual item's mod selection into an eval rule predicate (e.g., "if item has T1 life AND T1 res..."). |
| **poe-rqe** | "Export as Demand" converts the virtual item into an RQE demand spec — "I want a Vaal Regalia with these mods at these tiers". |
| **Overlay** | "Open in Browser" sends the inspected item's resolved data to populate a virtual item. Bidirectional: browser results can inform what the overlay highlights. |

---

## Data Requirements

### What We Already Have (13 tables)

These support the **core mod pool explorer** — our highest-value feature:

| Table | Rows | Browser Use |
|-------|------|-------------|
| BaseItemTypes | 5,334 | Search, base type details, implicit mods |
| ItemClasses | 99 | Category grouping, capability flags |
| ItemClassCategories | ~10 | Top-level grouping (Weapons, Armour, etc.) |
| Mods | 39,291 | **The mod pool** — tiers, levels, spawn weights, families |
| ModFamily | 7,678 | Prevent duplicate rolls (same family = exclusive) |
| ModType | 3 | Prefix / Suffix / Unique classification |
| Stats | 22,749 | Stat IDs for display text resolution |
| Tags | 1,353 | Spawn weight filtering (fossil/catalyst interaction) |
| Rarity | 4 | Mod count limits per rarity |
| ArmourTypes | 481 | Base defence values |
| WeaponTypes | 365 | Base damage/crit/speed values |
| ShieldTypes | 98 | Base block values |
| ClientStrings | 8,264 | Display text (property names, UI labels) |

Plus: **stat description reverse index** (15.5k patterns), **domain.rs** (inherited tags, trade mappings, pseudo definitions).

### Phase 1 — MVP: Mod Pool Explorer + Search

**Goal**: Search any item, see its mod pool, build a virtual item interactively.

**No new table extractions needed.** Everything for equipment mod pools is already in our data.

New Rust work:
- Mod pool computation: given (item_class, ilvl, influences[], tags[]) → filter Mods table by domain, generation_type, spawn_weight > 0 for matching tags, level ≤ ilvl
- Tier grouping: group mods by family → order by level descending → assign tier numbers
- Virtual item state management
- Search index over BaseItemTypes + ItemClasses (simple name matching)

New Tauri commands:
- `browser_search(query)` — fuzzy search across all entity types
- `browser_base_type(name)` — full base type detail with defence/weapon stats
- `browser_mod_pool(item_class, ilvl, influences, rarity)` — computed mod pool
- `browser_slot_mod(virtual_item, mod_id)` / `browser_unslot_mod(...)` — virtual item manipulation

New UI:
- Search bar component
- Base type detail view
- Mod pool table (sortable, filterable by prefix/suffix/tag)
- Virtual item sidebar
- Rarity / influence / ilvl controls

### Phase 2 — Jewels & Cluster Jewels

**Goal**: Full jewel browsing — regular, abyss, cluster, timeless.

**New table extractions needed:**

| Table | File | Purpose | Rows (est.) |
|-------|------|---------|-------------|
| PassiveTreeExpansionJewels | `passivetreeexpansionjewels.datc64` | Cluster jewel base → enchantment type mapping | ~20 |
| PassiveTreeExpansionJewelSizes | `passivetreeexpansionjewelsizes.datc64` | Small/Medium/Large definitions | 3 |
| PassiveTreeExpansionSkills | `passivetreeexpansionskills.datc64` | Notable passive skills from cluster jewels | ~568 |
| PassiveTreeExpansionSpecialSkills | `passivetreeexpansionspecialskills.datc64` | Keystones from cluster jewels | ~20 |
| AlternatePassiveSkills | `alternatepassiveskills.datc64` | Timeless jewel keystone variants | 182 |
| AlternatePassiveAdditions | `alternatepassiveadditions.datc64` | Timeless jewel stat additions | 96 |
| UniqueJewelLimits | `uniquejewellimits.datc64` | Unique jewel socket restrictions | small |

Regular jewels (Cobalt, Crimson, Viridian) and Abyss jewels already work — they're item classes with mod pools in the existing Mods table. The new extractions are for **cluster jewel enchantment notables** and **timeless jewel mechanics**.

New Rust work:
- Cluster jewel data model: size → enchantment types → available notables
- Jewel-specific mod pool filtering (domain = 12 for abyss, jewel tags)
- Affix limit override: jewels are 2/2 not 3/3 (already tracked in domain.rs)

New UI:
- Cluster jewel enchantment picker
- Notable passive cards (name, stats, icon)
- Timeless jewel seed explorer (stretch goal)

### Phase 3 — Crafting Sources (Essences, Fossils, Bench, Catalysts)

**Goal**: Show how different crafting methods change the mod pool.

**New table extractions needed:**

| Table | File | Purpose | Rows (est.) |
|-------|------|---------|-------------|
| Essences | `essences.datc64` | Essence → guaranteed mod per item class | 106 |
| EssenceType | `essencetype.datc64` | Essence names (Hatred, Woe, etc.) | 26 |
| CraftingBenchOptions | `craftingbenchoptions.datc64` | Bench craft mods, costs, item class restrictions | ~800 |
| CraftingBenchTypes | `craftingbenchtypes.datc64` | Bench type (Artisan, Master, etc.) | ~10 |
| CraftingBenchSortCategories | `craftingbenchsortcategories.datc64` | UI grouping for bench | ~20 |
| DelveCraftingModifiers | `delvecraftingmodifiers.datc64` | Fossil → tag weight multipliers | ~200 |
| DelveCraftingTags | `delvecraftingtags.datc64` | Fossil tag definitions | ~60 |
| CurrencyItems | `currencyitems.datc64` | Currency base data (needed for catalyst types) | 1,925 |

New Rust work:
- Fossil weight multiplier calculation: for each fossil, multiply spawn weights by tag multipliers
- Catalyst quality effects: specific mod categories get weight boost
- Essence guaranteed mod lookup: essence + item class → specific mod forced
- Bench craft filtering: item class → available bench crafts with costs

New UI:
- Fossil selector (multi-select) → shows modified spawn weights in mod pool, boosted mods highlighted
- Catalyst selector → shows which mods are boosted
- Essence view: pick an essence → see the guaranteed mod highlighted + remaining rollable pool
- Bench craft section: available crafts for the item class with currency costs

### Phase 4 — Corruption, Enchantments, Synthesis

**Goal**: Show all the ways an item can be modified beyond regular crafting.

**New table extractions needed:**

| Table | File | Purpose | Rows (est.) |
|-------|------|---------|-------------|
| InfluenceTags | `influencetags.datc64` | Influence type → mod tag per item class | 144 |
| InfluenceExalts | `influenceexalts.datc64` | Conqueror exalt outcomes | ~50 |
| LabyrinthCraftOptions | `labyrinthcraftoptions.datc64` | Helmet/glove/boot enchantments | ~1,500 |
| LabyrinthCraftOptionTiers | `labyrinthcraftoptiontiers.datc64` | Enchantment tiers (cruel/merciless/eternal) | ~20 |
| SynthesisBrackets | `synthesisbrackets.datc64` | Synthesis implicit thresholds | ~200 |
| SynthesisGlobalMods | `synthesisglobalmods.datc64` | Synthesis implicit mod pool | ~200 |

The corruption implicit pool is already in the Mods table (generation_type = 5, domain = 2). We just need to filter and present it.

New Rust work:
- Corruption implicit filtering from existing Mods table
- Influence-specific mod pool (filter by influence tag)
- Enchantment lookup by item class + lab tier
- Synthesis implicit threshold computation

New UI:
- "Corrupt" button on virtual item → shows possible corruption implicits
- Influence toggle enriches mod pool with influence-exclusive mods
- Enchantment browser per slot (helmets have ~1,500 options)
- Synthesis implicit estimator

### Phase 5 — Gems, Currency, Cards, Maps

**Goal**: Non-equipment entity browsing.

**New table extractions needed:**

| Table | File | Purpose | Rows (est.) |
|-------|------|---------|-------------|
| SkillGems | `skillgems.datc64` | Gem base data | 840 |
| SkillGemInfo | `skillgeminfo.datc64` | Gem metadata | ~840 |
| GemTags | `gemtags.datc64` | Gem tag definitions (Fire, Spell, Projectile, etc.) | 53 |
| GemEffects | `gemeffects.datc64` | Alternate quality effects | 1,283 |
| GrantedEffects | `grantedeffects.datc64` | Skill definitions (what the gem grants) | ~2,000 |
| GrantedEffectsPerLevel | `grantedeffectsperlevel.datc64` | Per-level scaling (damage, mana cost, etc.) | very large |
| GrantedEffectStatSets | `grantedeffectstatsets.datc64` | Stat set definitions | ~2,000 |
| GrantedEffectStatSetsPerLevel | `grantedeffectstatsetsperlevel.datc64` | Per-level stat values | very large |
| DivinationCardArt | `divinationcardart.datc64` | Card metadata + rewards | 467 |
| Maps | `maps.datc64` | Map data (tier, series) | 491 |
| MapSeries | `mapseries.datc64` | Map series names | 27 |
| MapSeriesTiers | `mapseriestiers.datc64` | Tier progression per series | 162 |
| WorldAreas | `worldareas.datc64` | Zone data (for div card drop locations) | ~900 |
| Scarabs | `scarabs.datc64` | Scarab effects | 27 |
| ScarabTypes | `scarabtypes.datc64` | Scarab categories | ~10 |
| Flasks | `flasks.datc64` | Flask base data | ~100 |
| Omens | `omens.datc64` | Omen effects | ~50 |
| Tinctures | `tinctures.datc64` | Tincture data | ~30 |
| Incubators | `incubators.datc64` | Incubator outcomes | ~50 |

New Rust work:
- Gem data model: level progression, stat scaling, tag system
- Gem support compatibility (which supports can apply to which active skills)
- Divination card → reward mapping
- Map → area data, tier system
- Currency categorization (from BaseItemTypes + CurrencyItems)

New UI:
- Gem detail view with level progression table
- Currency/scarab browser with categorized grid
- Divination card browser with reward display
- Map browser with tier/series filtering

### Phase 6 — Oil Anointments & League Content

**Goal**: Blight oils, current league content, quest rewards.

**New table extractions needed:**

| Table | File | Purpose | Rows (est.) |
|-------|------|---------|-------------|
| BlightCraftingItems | `blightcraftingitems.datc64` | Oil definitions | ~13 |
| BlightCraftingRecipes | `blightcraftingrecipes.datc64` | 3-oil → result recipes | ~600 |
| BlightCraftingResults | `blightcraftingresults.datc64` | Anointment outcomes | ~600 |
| QuestRewards | `questrewards.datc64` | Per-class gem rewards | ~500 |
| QuestRewardOffers | `questrewardoffers.datc64` | Quest reward offers | ~200 |
| PassiveSkills | `passiveskills.datc64` | Full passive tree (for anointments) | 5,592 |
| PassiveSkillMasteryEffects | `passiveskillmasteryeffects.datc64` | Mastery options | ~200 |
| Mirage league tables | `brequel*.datc64` (11 tables) | Current league content | varies |

### Phase 7 — Passive Tree & Atlas (stretch)

Interactive passive tree and atlas visualizations. These are complex UI challenges (SVG rendering, node positioning) and may be better served by linking to existing tools than reimplementing.

**Tables**: PassiveSkills (5,592), PassiveSkillTrees, AtlasNode (166), AtlasNodeDefinition (211), AtlasMods (141).

---

## Table Extraction Pattern

Each new table extraction follows the same mechanical pattern:

### 1. Define row struct (`poe-dat/src/tables/types.rs`)

```rust
/// An essence from `Essences.datc64`.
#[derive(Debug, Clone)]
pub struct EssenceRow {
    pub essence_type: u64,       // FK to EssenceType
    pub level: i32,              // Essence tier level
    pub item_class_mods: Vec<(u64, u64)>,  // (ItemClass FK, Mod FK) pairs
}
```

### 2. Write extractor (`poe-dat/src/tables/extract.rs`)

```rust
pub fn extract_essences(dat: &DatFile) -> Vec<EssenceRow> {
    // Read each row at known byte offsets (from dat-schema)
    // Return Vec<EssenceRow>
}
```

### 3. Get byte offsets from dat-schema

Query `dat-schema` (community GraphQL SDL) for the table's field offsets. Verify against our datc64 file by checking total row size = file size / row count.

### 4. Load into GameData (`poe-data/src/game_data.rs`)

Add new field + index to `GameData`, load in `load()`.

### 5. Expose via Tauri command (`app/src-tauri/src/commands/browser.rs`)

Create command that queries GameData, returns serialized result.

### 6. Test against real data

Extract test uses real datc64 files from `_reference/ggpk-data-3.28/`.

---

## Full GGPK Table Inventory (Browser-Relevant)

All tables that exist in `_reference/ggpk-data-3.28/` grouped by browser feature:

### Already Extracted
- `stats.datc64`, `tags.datc64`, `mods.datc64`, `modfamily.datc64`, `modtype.datc64`
- `itemclasses.datc64`, `itemclasscategories.datc64`, `baseitemtypes.datc64`
- `rarity.datc64`, `armourtypes.datc64`, `weapontypes.datc64`, `shieldtypes.datc64`
- `clientstrings.datc64`

### Crafting & Mod Sources
- `essences.datc64`, `essencetype.datc64`, `essencestashtablayout.datc64`
- `craftingbenchoptions.datc64`, `craftingbenchtypes.datc64`, `craftingbenchsortcategories.datc64`
- `craftingbenchspecificoptionid.datc64`, `craftingitemclasscategories.datc64`
- `delvecraftingmodifiers.datc64`, `delvecraftingtags.datc64`, `delvecraftingmodifierdescriptions.datc64`
- `harvestcraftoptions.datc64`, `harvestcrafttiers.datc64`, `harvestcraftfilters.datc64`
- `influencetags.datc64`, `influenceexalts.datc64`, `influencemodupgrades.datc64`

### Gems & Skills
- `skillgems.datc64`, `skillgeminfo.datc64`, `gemtags.datc64`, `gemeffects.datc64`
- `grantedeffects.datc64`, `grantedeffectsperlevel.datc64`
- `grantedeffectstatsets.datc64`, `grantedeffectstatsetsperlevel.datc64`
- `grantedeffectqualitystats.datc64`

### Jewels & Passives
- `passivetreeexpansionjewels.datc64`, `passivetreeexpansionjewelsizes.datc64`
- `passivetreeexpansionskills.datc64`, `passivetreeexpansionspecialskills.datc64`
- `alternatepassiveskills.datc64`, `alternatepassiveadditions.datc64`
- `uniquejewellimits.datc64`
- `passiveskills.datc64`, `passiveskillstatcategories.datc64`
- `passiveskillmasteryeffects.datc64`, `passiveskillmasterygroups.datc64`
- `passivejewelradii.datc64`, `passivejewelslots.datc64`
- `passivejewelnodemodifyingstats.datc64`

### Currency & Consumables
- `currencyitems.datc64`, `currencyexchange.datc64`, `currencyexchangecategories.datc64`
- `scarabs.datc64`, `scarabtypes.datc64`
- `flasks.datc64`, `tinctures.datc64`, `omens.datc64`, `incubators.datc64`

### Cards, Maps, Areas
- `divinationcardart.datc64`, `divinationcardstashtablayout.datc64`
- `maps.datc64`, `mapseries.datc64`, `mapseriestiers.datc64`
- `worldareas.datc64`

### Enchantments & Lab
- `labyrinthcraftoptions.datc64`, `labyrinthcraftoptiontiers.datc64`

### Synthesis
- `synthesisbrackets.datc64`, `synthesisglobalmods.datc64`

### Oils / Anointments
- `blightcraftingitems.datc64`, `blightcraftingrecipes.datc64`
- `blightcraftingresults.datc64`, `blightcraftingtypes.datc64`

### Item Metadata
- `componentattributerequirements.datc64` (Str/Dex/Int per base)
- `itemframetype.datc64` (rarity colors)

### Quest & Rewards
- `questrewards.datc64`, `questrewardoffers.datc64`

### Atlas
- `atlasnode.datc64`, `atlasnodedefinition.datc64`
- `atlasmods.datc64`, `atlaspassiveskilltreegrouptype.datc64`

### Current League (Mirage 3.28)
- `brequel*.datc64` (11 tables — Foulborn/Bloodline content)

---

## Mod Pool Computation — The Core Algorithm

This is the heart of the browser. Given a virtual item configuration, compute which mods can roll on it.

### Inputs
- `item_class_id: &str` — e.g., "BodyArmour"
- `item_level: u32` — determines max mod tier
- `rarity: Rarity` — determines prefix/suffix count limits
- `influences: &[InfluenceKind]` — adds influence-specific tags
- `slotted_mods: &[SlottedMod]` — already-chosen mods (for family exclusion)
- `crafting_source: Option<CraftingSource>` — fossil/catalyst/essence filter

### Algorithm

```
1. Resolve effective tags for this item:
   - base_type.tags (from BaseItemTypes)
   - inherited tags (from domain.rs inherited_tags_for_parent)
   - influence tags (from InfluenceTags table, per item_class + influence type)

2. Filter Mods table:
   - domain matches item_class domain (1 = item mods, 12 = abyss, etc.)
   - generation_type in {1=prefix, 2=suffix} (or 5 for corruption implicits)
   - level <= item_level
   - max_level == 0 OR max_level >= item_level
   - For each mod, compute spawn_weight:
     Walk spawn_weight_tags/values, first tag that matches effective_tags → use that weight
     If fossil active: multiply weights by fossil tag multipliers
     If catalyst active: boost weights for matching mod categories
   - spawn_weight > 0 (mod can actually appear)

3. Group by family:
   - Mods sharing a ModFamily are tiered versions of the same effect
   - Sort within family by level descending → T1 is highest
   - If a slotted mod's family matches, mark the entire family as "taken"

4. Apply slot limits:
   - Count slotted prefixes/suffixes
   - If at max prefixes (from Rarity table), grey out remaining prefix families
   - Same for suffixes

5. Return grouped, annotated mod pool
```

### What makes this different from poedb

poedb shows a static mod pool page. We show a **live, interactive pool that changes as you build**:
- Slot a mod → its family is removed, slot count updates, mod pool shrinks
- Toggle influence → influence-only mods appear/disappear
- Drag ilvl slider → high-tier mods appear/disappear
- Apply fossil → spawn weights recalculate, some mods boost, some disappear

---

## Search Design

### Universal Search

One search bar that matches across all entity types:

```
Type to search...
┌──────────────────────────────────────────┐
│ 🗡 Vaal Regalia         Body Armour     │  ← BaseItemType
│ 🗡 Vaal Hatchet         One Hand Axe    │
│ 📜 Vaal Orb             Currency         │
│ 💎 Vaal Grace            Gem (Aura)      │
│ 🃏 The Valkyrie          Divination Card │
│ ...                                      │
└──────────────────────────────────────────┘
```

Search matches against:
- BaseItemTypes.name (all 5,334 base types)
- Mod names (39,291 mods — for "find items that can roll X")
- Stat descriptions (from reverse index — "maximum life" finds the stat)
- Gem names (from SkillGems once extracted)
- Currency/scarab/card names (from BaseItemTypes where item_class matches)

Search is **prefix + fuzzy** — "vaal reg" matches "Vaal Regalia", "max life" matches "+# to maximum Life".

### Result Routing

Each search result type routes to a different view:
- BaseItemType (equipment) → Base Type View + Mod Pool Explorer
- BaseItemType (gem) → Gem View
- BaseItemType (currency/scarab/card) → Currency/Card View
- Mod name → shows all item classes that can roll this mod
- Stat description → shows all mods that grant this stat

---

## Implementation Order

### MVP (Phase 1) — ~2-3 weeks of focused work

**Backend:**
1. Mod pool computation engine in poe-data (or new `poe-browser` crate)
2. Virtual item state type
3. Tauri commands: search, base_type, mod_pool, slot_mod, unslot_mod
4. Search index (simple trigram or prefix matching over entity names)

**Frontend:**
5. Browser window setup (new Tauri window)
6. SearchBar component
7. BaseTypeView (name, class, stats, requirements, controls)
8. ModPoolExplorer (prefix/suffix tables, sortable, tier-grouped)
9. VirtualItem sidebar (slotted mods, open slot count)
10. Rarity / ilvl / influence controls

**Integration:**
11. "Open in Browser" button on overlay item view

### Phase 2 — Jewels
12. Cluster jewel table extractions (4 tables)
13. Cluster enchantment → notable mapping
14. Jewel-specific UI (enchantment picker, notable cards)
15. Timeless jewel data (2 tables)

### Phase 3 — Crafting Sources
16. Essence extraction (2 tables) + guaranteed mod lookup
17. Fossil extraction (2 tables) + weight multiplier calc
18. Bench craft extraction (3 tables) + cost display
19. Catalyst weight boost logic
20. UI: crafting source selectors in mod pool view

### Phase 4 — Corruption, Enchants, Synthesis
21. Corruption implicit filtering (already in Mods table)
22. Influence tag extraction (1 table)
23. Enchantment extraction (2 tables)
24. Synthesis implicit extraction (2 tables)
25. UI: corruption/enchant/synthesis panels

### Phase 5 — Non-Equipment Entities
26. Gem extraction (5+ tables, large)
27. Currency/scarab metadata extraction
28. Div card extraction
29. Map extraction
30. UI: entity-specific views

### Phase 6+ — League Content, Oils, Atlas, Passive Tree
31. Oil anointment extraction
32. Quest reward extraction
33. Passive tree data (large)
34. Atlas data
35. Current league content

---

## Open Questions

1. **New crate or extend poe-data?** The mod pool computation logic is substantial. It could live in poe-data (extending GameData with query methods) or in a new `poe-browser` crate that depends on poe-data. Leaning toward keeping it in poe-data since it's fundamentally "querying game data."

2. **Disk caching for GameData?** Loading 13+ tables from datc64 is fast enough today (~200ms). With 40+ tables it might get slow. Consider serialized cache (bincode/MessagePack) that rebuilds when datc64 files change.

3. **dat-schema dependency?** We currently hardcode byte offsets. With 40+ tables, maintaining offsets manually is painful. Consider parsing dat-schema GraphQL to auto-generate offsets. But this adds a build-time dependency and complexity.

4. **Scope creep risk**: poedb has accumulated 10+ years of features. We should explicitly NOT try to replicate everything. The item-centric flow is our differentiator. Passive tree visualization and atlas maps are better served by linking to external tools.

5. **Virtual item persistence?** Should virtual items be saveable? Named? Shareable? This could be powerful for build planning but adds complexity. Start without persistence, add later based on demand.
