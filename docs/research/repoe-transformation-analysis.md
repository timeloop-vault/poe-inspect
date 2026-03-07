# RePoE Transformation Analysis: How Much Work Did v1 Do to Make RePoE Data Usable?

## Executive Summary

The v1 project required **substantial transformation** of RePoE data before the parser could use it. This involved:

1. A dedicated **`poe-data-processor` crate** (ETL pipeline) that transforms `stat_translations.json` into a template-keyed lookup table
2. A **`GameDataManager`** in the parser that compiles stat translations into regex patterns at runtime
3. A **`ModDatabase`** that loads all of `mods.json` (~20MB), filters to rollable mods, builds stat indexes, and calculates tier tables
4. A **`BaseItemDatabase`** that re-indexes base items from metadata-path keys to name-based keys
5. A **`BaseTypeManager`** (duplicate of BaseItemDatabase with slight differences) doing the same re-indexing

The core problem: **RePoE's data is organized by internal ID, but the parser needs data organized by what the player sees.** Every data source required at least one re-indexing transformation, and the mod filtering logic alone is ~60 lines of business rules accumulated over multiple development sessions.

---

## 1. The Transformation Pipeline

### Overview

```
RePoE stat_translations.json ──┐
                                ├─→ poe-data-processor (offline ETL)
                                │     └─→ template-stat-lookup.json (14,229 entries)
                                │
                                ├─→ GameDataManager (runtime, regex compilation)
                                │     └─→ compiled_patterns HashMap
                                │
RePoE mods.json ────────────────┼─→ ModDatabase (runtime)
                                │     ├─→ stat_index: HashMap<stat_id, Vec<mod_id>>
                                │     └─→ tier_cache: HashMap<stat_id, Vec<TierEntry>>
                                │
RePoE base_items.json ──────────┼─→ BaseItemDatabase (runtime, re-indexed by name)
                                └─→ BaseTypeManager (runtime, re-indexed by name)
```

### What the `loader` loads

The loader (`poe-data-processor/src/loader.rs`) is minimal -- it reads `stat_translations.json` and deserializes it into `Vec<StatTranslation>`. However, it **discards most of the RePoE fields** during deserialization:

```rust
// What RePoE provides per translation entry:
pub struct TranslationVariant {
    pub condition: Vec<TranslationCondition>,  // DISCARDED by data-processor
    pub format: Vec<String>,                    // DISCARDED by data-processor
    pub index_handlers: Vec<Vec<String>>,       // DISCARDED by data-processor
    pub string: String,                         // KEPT
    pub reminder_text: Option<String>,          // DISCARDED by data-processor
    pub is_markup: Option<bool>,                // DISCARDED by data-processor
}

// What the data-processor keeps:
pub struct TranslationEntry {
    pub string: String,  // That's it. Just the template string.
}
```

The data-processor's `StatTranslation` type ignores `condition`, `format`, `index_handlers`, `reminder_text`, `is_markup`, `trade_stats`, `hidden`, and all non-English language fields (`French`, `German`, `Japanese`, `Korean`, `Portuguese`, `Russian`, `Spanish`, `Thai`, `Traditional_Chinese`).

### What the `transformer` transforms, and WHY

The transformer (`poe-data-processor/src/transformer.rs`) performs one key transformation:

**Input (RePoE format):** An array of stat translation groups, each containing:
- `ids`: array of stat IDs (e.g., `["base_cold_damage_resistance_%"]`)
- `English`: array of translation variants, each with a `string` field

**Output (parser format):** A HashMap keyed by template string, each value containing:
- `stat_ids`: the stat IDs from the original group
- `value_count`: number of placeholders in the template

**Why this transformation is necessary:**

RePoE indexes by stat_id (grouped). The parser needs to go the **opposite direction**: given a template string extracted from clipboard text, find the stat_id(s). This is an inversion of the lookup direction.

```
RePoE:     stat_id → [template_string_1, template_string_2, ...]
Parser:    template_string → [stat_id_1, stat_id_2, ...]
```

The transformer also counts placeholders (`{0}`, `{1}`, etc.) using regex, which is metadata that doesn't exist in RePoE at all -- RePoE stores this implicitly in the template format but never as an explicit count.

### What the `exporter` produces

The exporter serializes the `TemplateLookupTable` (a `HashMap<String, StatLookupEntry>`) to JSON. The output file `template-stat-lookup.json` is 85,926 lines and contains 14,229 template entries.

### What is `template-stat-lookup.json` and why does it exist?

It is a pre-computed lookup table that maps human-readable modifier templates to internal stat IDs. Example entries:

```json
"{0}% to Cold Resistance": {
    "stat_ids": ["base_cold_damage_resistance_%"],
    "value_count": 1
},
"Adds {0} to {1} Chaos Damage to Spells and Attacks during any Flask Effect": {
    "stat_ids": [
        "spell_and_attack_minimum_added_chaos_damage_during_flask_effect",
        "spell_and_attack_maximum_added_chaos_damage_during_flask_effect"
    ],
    "value_count": 2
}
```

It exists because the parser needs **O(1) template-to-stat-id resolution** at runtime. The alternative (used by `GameDataManager`) is to compile thousands of regex patterns at startup, which is slower and more error-prone. The pre-processed lookup is `include_str!`'d at compile time via `template_lookup.rs`.

---

## 2. What RePoE Data Couldn't Be Used As-Is

### Transformation 1: stat_translations.json -- Lookup Direction Inversion

**RePoE input shape:**
```json
[
  {
    "English": [
      {
        "condition": [{"min": 1, "max": null, "negated": null}],
        "format": ["#"],
        "index_handlers": [[]],
        "string": "{0}% increased Rogue's Markers dropped by monsters"
      },
      {
        "condition": [{"min": null, "max": -1, "negated": null}],
        "format": ["#"],
        "index_handlers": [["negate"]],
        "string": "{0}% reduced Rogue's Markers dropped by monsters"
      }
    ],
    "ids": ["heist_coins_from_monsters_+%"],
    "trade_stats": null,
    "hidden": null,
    "French": null, "German": null, "Japanese": null, ...
  }
]
```

**Parser needs:**
```json
{
  "{0}% increased Rogue's Markers dropped by monsters": {
    "stat_ids": ["heist_coins_from_monsters_+%"],
    "value_count": 1
  },
  "{0}% reduced Rogue's Markers dropped by monsters": {
    "stat_ids": ["heist_coins_from_monsters_+%"],
    "value_count": 1
  }
}
```

**Why the transformation is necessary:**
- RePoE groups multiple display variants under one stat ID. The parser starts with display text and needs the stat ID. This is a fundamental key inversion.
- RePoE includes conditions, format specifiers, index handlers, reminder text, markup flags, trade stats, and 9 language translations -- none of which the template lookup needs.
- RePoE doesn't pre-compute the placeholder count; the transformer must extract it.

**Code dedicated:** ~50 lines in `transformer.rs`, ~30 lines in `types.rs`, ~20 lines in `loader.rs`, ~30 lines in `exporter.rs`, plus `main.rs` orchestration. Total: **~180 lines** in the data-processor crate (excluding tests).

### Transformation 2: stat_translations.json -- Runtime Regex Compilation

In addition to the offline ETL, the parser's `GameDataManager` (`data.rs`) performs a **completely separate** runtime transformation of the same `stat_translations.json`:

**What it does:**
1. Loads the full stat translation structure (including conditions, index_handlers)
2. For each translation variant, converts the template string into a regex pattern
3. Handles placeholder replacement: `{0}` becomes `([-+]?\d+)`, `{1}` becomes `(\d+)`
4. Detects negation via `index_handlers` containing `"negate"`
5. Stores compiled patterns indexed by stat_id

**Example transformation:**
```
Template: "{0}% increased Attack Speed"
Regex:    "^([-+]?\d+)% increased Attack Speed$"

Template: "{0}% reduced Attack Speed"  (with negate handler)
Regex:    "^([-+]?\d+)% reduced Attack Speed$"  (is_negative = true)
```

**Why this exists alongside the template lookup:**
The template lookup provides stat_id resolution, but it can't calculate tier/roll quality. The `GameDataManager` regex matching can extract the actual numeric value and its sign, then cross-reference with `mods.json` for tier calculation. These are **two parallel systems** doing overlapping work because neither alone was sufficient.

**Code dedicated:** ~140 lines in `data.rs` for `compile_translation_patterns()` and `compile_variant_pattern()`, plus ~80 lines for `match_modifier()` and `calculate_tier_and_quality()`. Total: **~220 lines** of runtime transformation.

### Transformation 3: mods.json -- Rollable Mod Filtering

**RePoE input shape (per mod):**
```json
{
  "ModName123": {
    "name": "of the Blizzard",
    "text": "+43% to Cold Resistance",
    "stats": [{"id": "base_cold_damage_resistance_%", "min": 42, "max": 45}],
    "required_level": 72,
    "generation_type": "suffix",
    "spawn_weights": [{"tag": "ring", "weight": 500}, {"tag": "default", "weight": 0}],
    "groups": ["ColdResistance"],
    "domain": "item",
    "is_essence_only": false,
    "generation_weights": [],
    "adds_tags": []
  }
}
```

**The filtering problem:**
RePoE's `mods.json` contains **ALL** mods -- unique item mods, essence mods, fossil mods, harvest mods, veiled mods, delve mods, synthesis mods, betrayal mods, legion mods, crafted bench mods, flask mods, jewel mods, and normal rollable mods. The parser only cares about **rollable item mods** for tier calculation.

**Filtering rules (accumulated over 5+ development sessions):**

```rust
fn is_rollable_mod(mod_def: &ModDefinition, base_tags: Option<&[String]>) -> bool {
    // 1. Only prefix and suffix (excludes "unique", "corrupted", etc.)
    if mod_def.generation_type != "prefix" && mod_def.generation_type != "suffix" {
        return false;
    }
    // 2. Must have spawn weights
    if mod_def.spawn_weights.is_empty() { return false; }
    // 3. At least one weight > 0
    if mod_def.spawn_weights.iter().all(|w| w.weight == 0) { return false; }
    // 4. Domain must be "item" (excludes flask, jewel, abyss_jewel, etc.)
    if mod_def.domain != "item" { return false; }
    // 5. Not essence-only
    if mod_def.is_essence_only { return false; }
    // 6. Name-based heuristic filtering (fragile!)
    let name = mod_def.name.to_lowercase();
    if name.contains("unique") || name.contains("essence") || name.contains("fossil")
        || name.contains("harvest") || name.contains("veiled") || name.contains("delve")
        || name.contains("synthesis") || name.contains("betrayal")
        || name.contains("legion") || name.contains("fractured") {
        return false;
    }
    // 7. Base tag filtering (optional)
    if let Some(tags) = base_tags {
        let can_spawn = mod_def.spawn_weights.iter()
            .any(|sw| tags.iter().any(|base_tag| base_tag == &sw.tag));
        if !can_spawn { return false; }
    }
    true
}
```

**Why this transformation is necessary:**
RePoE doesn't distinguish "normal rollable mods" from special mods in any clean way. The `generation_type` field helps (prefix/suffix vs unique/corrupted), the `domain` field helps (item vs flask vs jewel), and `is_essence_only` helps. But **there's no single "is_rollable" flag.** The name-based heuristic filtering (step 6) is particularly fragile -- it was added iteratively as investigation modules (`domain_investigation.rs`, `edge_case_survey.rs`, `generation_weights_analysis.rs`) discovered new categories of mods that were polluting tier tables.

**Code dedicated:** ~60 lines for `is_rollable_mod()` (duplicated with slight variations in both `mod_database.rs` and `data.rs`), plus ~70 lines of investigation/analysis code across 3 dedicated test modules. Total: **~190 lines** (including the duplication).

### Transformation 4: mods.json -- Stat Index Building and Tier Table Computation

**What the ModDatabase builds at initialization:**

```rust
pub struct ModDatabase {
    mods: HashMap<String, ModDefinition>,              // Raw mod data
    stat_index: HashMap<String, Vec<String>>,           // stat_id → Vec<rollable mod_id>
    tier_cache: HashMap<String, Vec<TierEntry>>,        // stat_id → sorted tier table
}
```

**Tier table construction algorithm:**
1. For each rollable mod, extract each stat and add the mod_id to `stat_index[stat_id]`
2. For each stat_id, collect all `(min, max)` ranges from rollable mods
3. Sort ranges by max value descending (highest = T1)
4. Deduplicate identical ranges
5. Assign tier numbers (1-based)

**Why this transformation is necessary:**
RePoE stores mods as flat records. The parser needs to answer: "Given stat X with value Y on base item Z, what tier is this?" This requires:
- A reverse index from stat_id to mods (RePoE indexes by mod_id)
- Filtering to only relevant mods
- Sorting by value range to determine tier ordering
- Base-tag-aware filtering for item-specific tier tables

**Additional complexity -- mod group filtering:**
The `get_tier_info_with_base()` method recalculates the tier table **on every call** when mod groups are provided. This is because different mod groups (e.g., `IncreasedAttackSpeed` vs `IncreasedAttackSpeedAndDoubleChance`) create different tier tables for the same stat_id. The algorithm:
1. Detect which mod group the value falls into via `get_mod_groups_for_stat_value()`
2. Recalculate the tier table filtering to only that group
3. Find the tier within that filtered table

**Code dedicated:** ~250 lines in `mod_database.rs` (excluding tests), ~80 lines in `tier_validation.rs`. Total: **~330 lines**.

### Transformation 5: base_items.json -- Re-indexing by Name

**RePoE input shape:**
```json
{
  "Metadata/Items/Weapons/TwoHandWeapons/TwoHandAxes/TwoHandAxe17": {
    "name": "Vaal Axe",
    "item_class": "Two Hand Axe",
    "tags": ["weapon", "two_hand_weapon", "axe", "twohand", ...],
    "domain": "item",
    "drop_level": 64,
    ...
  }
}
```

**Parser needs:** Look up by name (e.g., "Vaal Axe") to get tags.

**Transformation:** Iterate all entries, discard the metadata path key, re-insert into a new HashMap keyed by `name`. Also normalize tags to lowercase.

**Why this transformation is necessary:**
The parser extracts the base type name from clipboard text (e.g., "Vaal Axe"). RePoE indexes by internal metadata path (e.g., `Metadata/Items/Weapons/.../TwoHandAxe17`). The clipboard never contains metadata paths.

**Code dedicated:** This transformation is implemented **twice** in nearly identical code:
- `base_items.rs`: `BaseItemDatabase` (~60 lines)
- `base_types.rs`: `BaseTypeManager` (~45 lines)

Both load the same JSON, both re-index by name. `BaseItemDatabase` is used by the `ModDatabase` integration for tier filtering. `BaseTypeManager` is used by `GameDataManager` for spawn weight checking. Total: **~105 lines** of duplication.

---

## 3. The ModDatabase in Detail

### Building Tier Tables from mods.json

The `build_indexes()` method runs at initialization after loading the full ~20MB `mods.json`:

```rust
fn build_indexes(&mut self) {
    // Pass 1: Build stat_id → mod_ids index (only rollable mods)
    for (mod_id, mod_def) in &self.mods {
        if !is_rollable_mod(mod_def, None) { continue; }
        for stat in &mod_def.stats {
            self.stat_index.entry(stat.id.clone())
                .or_insert_with(Vec::new)
                .push(mod_id.clone());
        }
    }
    // Pass 2: Pre-calculate tier tables for all stats (unfiltered)
    for stat_id in self.stat_index.keys() {
        let tiers = self.calculate_tier_table(stat_id, None, None);
        self.tier_cache.insert(stat_id.clone(), tiers);
    }
}
```

### Filtering Applied (and Problems Skipping Causes)

The filtering evolved over 5+ sessions. The v1 commit history reveals a progression:

1. **Initial**: Only filtered by `generation_type` (prefix/suffix). **Problem**: Unique-item mods, essence mods, and crafted mods polluted tier tables, making T1 appear to have impossibly high ranges.

2. **Session 2**: Added spawn weight filtering (weight > 0). **Problem**: Still included flask mods, jewel mods, and influence-specific mods.

3. **Session 5**: Added `domain` filtering (must be "item") and `is_essence_only` flag. **Problem**: Still included fossil/harvest/delve mods which have domain "item" but aren't normally rollable.

4. **Final**: Added name-based heuristic filtering for fossil, harvest, veiled, delve, synthesis, betrayal, legion mods. **This is the fragile part** -- it depends on GGG's naming conventions and will break if they change naming patterns.

### Spawn Weights and Base Item Tags

The spawn weight system determines which mods can appear on which items:

```
Mod spawn_weights: [{"tag": "ring", "weight": 500}, {"tag": "amulet", "weight": 300}, {"tag": "default", "weight": 0}]
Base item tags:    ["ring", "default", "not_for_sale"]
```

Matching rule: A mod can spawn on a base if at least one spawn_weight tag matches a base tag AND that weight > 0.

In the example above: "ring" matches and weight=500, so the mod can spawn on this ring. The "default" tag also matches but weight=0, which means it's explicitly blocked from the default category.

### Mod-Group-Aware Tier Calculation

This was one of the hardest problems. Consider attack speed on weapons:

- `IncreasedAttackSpeed` group: mods with just attack speed (e.g., 8-27%)
- `IncreasedAttackSpeedAndDoubleChance` group: hybrid mods with attack speed + double damage chance

If you calculate tiers for `local_attack_speed_+%` without group filtering, you get a combined tier table mixing both groups. A value of 12% might be T3 in the combined table but T2 in the pure attack speed table.

The solution: `get_mod_groups_for_stat_value()` first identifies which mod group a value belongs to by finding a rollable mod where the value falls within range. Then `get_tier_info_with_base()` recalculates the tier table filtered to only that group.

```rust
// Step 1: Detect mod group
let mod_groups = mod_db.get_mod_groups_for_stat_value(stat_id, value.value, tags);

// Step 2: Calculate tiers within that group only
let tier_info = mod_db.get_tier_info_with_base(
    stat_id, value.value, modifier.tier, tags,
    mod_groups.as_deref(),  // Only mods from THIS group
);
```

---

## 4. The Template Lookup

### How stat_translations.json Gets Processed

The offline ETL (`poe-data-processor`) performs this transformation:

1. **Load** the JSON array (530,915 lines)
2. **For each** stat translation group:
   - Iterate English variants
   - For each variant, extract the `string` field (the template)
   - Count placeholders using regex `\{(\d+)\}`
   - Create entry: `template → {stat_ids, value_count}`
3. **Deduplicate**: If same template appears twice, keep the first (shouldn't happen in practice)
4. **Export** as a HashMap to JSON

### Edge Cases and Ambiguities

**Multiple variants per stat ID:**
A single stat ID can have multiple display variants. For example, `local_attack_speed_+%` maps to BOTH:
- `"{0}% increased Attack Speed"` (when value is positive)
- `"{0}% reduced Attack Speed"` (when value is negative, with `negate` handler)

The template lookup handles this by inserting both templates pointing to the same stat_id.

**Multi-stat templates:**
Some templates map to multiple stat IDs:
```json
"Adds {0} to {1} Lightning Damage": {
    "stat_ids": [
        "global_minimum_added_lightning_damage",
        "global_maximum_added_lightning_damage"
    ],
    "value_count": 2
}
```
Here `{0}` corresponds to `stat_ids[0]` (min) and `{1}` to `stat_ids[1]` (max).

**Zero-value templates:**
Some templates have no placeholders:
```json
"Cannot be Frozen": {
    "stat_ids": ["base_cannot_be_frozen"],
    "value_count": 0
}
```

**Template extraction from clipboard text:**
The `extract_template()` function in `patterns.rs` converts raw clipboard text into a template:
```
"+45(42-45)% to Cold Resistance"  →  "{0}% to Cold Resistance"
"Adds 7(3-9) to 108(102-117) Lightning Damage"  →  "Adds {0} to {1} Lightning Damage"
```

Rules:
- Strip leading `+` (positive is default in PoE text)
- Keep leading `-` (negative is meaningful)
- Replace each numeric value (including ranges like `42-45`) with `{N}` placeholder
- Normalize whitespace but preserve newlines

### Scale

- **14,229 template entries** in the processed lookup table
- **530,915 lines** of raw stat_translations.json (the entire file)
- Templates with 0 placeholders: common (binary mods like "Cannot be Frozen")
- Templates with 1 placeholder: most common
- Templates with 2 placeholders: damage ranges, dual-stat mods
- Templates with 3+ placeholders: rare but exist (e.g., hybrid descriptions with reminder text)

### Ambiguity Problem

The template lookup is a HashMap, so each template string must be unique. The data-processor uses `entry().or_insert()` which keeps the first occurrence. In practice, RePoE's stat_translations should not have duplicate templates, but the code defensively handles it.

The real ambiguity is **different stat IDs producing the same template string** -- this would cause incorrect stat_id assignment. The v1 code doesn't explicitly handle this case (it trusts that RePoE templates are unique across stat groups).

---

## 5. The Full Relationship Graph

### Data Flow: Clipboard Text to Tier Information

```
clipboard_text
    │
    ├─→ extract_template(text)
    │       │
    │       └─→ template_string (e.g., "{0}% to Cold Resistance")
    │               │
    │               └─→ template_lookup.lookup_stat_ids(template)
    │                       │
    │                       └─→ stat_ids (e.g., ["base_cold_damage_resistance_%"])
    │                           value_count (e.g., 1)
    │
    ├─→ GameDataManager.match_modifier(text, base_type, item_level)
    │       │
    │       ├─→ compiled_patterns[stat_id].regex.captures(text)
    │       │       └─→ extracted values, negation detection
    │       │
    │       └─→ calculate_tier_and_quality(stat_id, value, base_type, item_level)
    │               ├─→ base_type_manager.get_tags(base_type) → base_tags
    │               ├─→ filter mods by is_rollable + item_level + base_tags
    │               └─→ sort by max desc → tier assignment
    │
    └─→ ModDatabase.get_tier_info_with_base(stat_id, value, text_tier, base_tags, mod_groups)
            ├─→ get_mod_groups_for_stat_value(stat_id, value, base_tags)
            │       └─→ find first rollable mod where value ∈ [min, max]
            │           └─→ return mod_def.groups
            │
            └─→ calculate_tier_table(stat_id, base_tags, mod_groups)
                    ├─→ stat_index[stat_id] → Vec<mod_id>
                    ├─→ for each mod_id: is_rollable(mod, base_tags)?
                    ├─→ filter by mod_groups exact match
                    ├─→ collect (min, max) ranges
                    ├─→ sort by max desc, dedup
                    └─→ enumerate → tier assignments
```

### Data Flow: Base Type Resolution

```
clipboard_header
    │
    └─→ parse base_type text (e.g., "Vaal Axe")
            │
            ├─→ BaseItemDatabase.get_tags("Vaal Axe")
            │       └─→ ["weapon", "two_hand_weapon", "axe", "twohand", ...]
            │
            └─→ BaseTypeManager.get_tags("Vaal Axe")  [duplicate system]
                    └─→ ["weapon", "two_hand_weapon", "axe", "twohand", ...]
```

### All Joins/Lookups the Parser Performs

| # | Lookup | Source Data | Key | Returns |
|---|--------|------------|-----|---------|
| 1 | Template → stat_ids | template-stat-lookup.json (pre-processed) | template string | stat_ids[], value_count |
| 2 | Text → regex match | stat_translations.json (compiled to regex) | modifier text | stat_id, value, is_negative |
| 3 | Stat ID → rollable mods | mods.json (indexed by stat_id) | stat_id | Vec<mod_id> |
| 4 | Mod ID → mod definition | mods.json (raw HashMap) | mod_id | ModDefinition |
| 5 | Base name → tags | base_items.json (re-indexed by name) | base_type name | tags[] |
| 6 | Stat value + tags → mod group | mods.json (filtered scan) | (stat_id, value, base_tags) | groups[] |
| 7 | Stat ID + filters → tier table | mods.json (computed) | (stat_id, base_tags, mod_groups) | Vec<(tier, min, max)> |
| 8 | Value + tier range → roll quality | computed | (value, min, max) | f32 (0.0-1.0) |

### Data Sources Touched

| RePoE File | Size | Parser Modules Using It | Format |
|-----------|------|------------------------|--------|
| stat_translations.json | 530K lines | poe-data-processor, GameDataManager, template_lookup | Array of translation groups |
| mods.json | ~20MB (1 line) | ModDatabase, GameDataManager | Map of mod_id → ModDefinition |
| base_items.json | Large (1 line) | BaseItemDatabase, BaseTypeManager | Map of metadata_path → BaseItem |
| stats.json | ~5K lines | Not used by parser (only exists in data dir) | Map of stat_id → stat metadata |
| template-stat-lookup.json | 86K lines | template_lookup (include_str!) | Map of template → StatLookupEntry |

---

## 6. What Would Be Different with Raw GGPK Data?

### Transformations That Would Be UNNECESSARY with GGPK

1. **Lookup direction inversion (stat_translations)**: If we read GGPK's `StatDescriptions` directly, we could build the template→stat_id mapping as the primary index from the start, rather than inverting RePoE's stat_id→template structure.

2. **Stripping unused fields**: RePoE adds 9 language translations, trade_stats, and other metadata that we immediately discard. Reading from GGPK, we'd only extract what we need.

3. **Re-indexing base items by name**: GGPK's `BaseItemTypes.dat` can be read with name as the primary key directly, rather than loading metadata-path-keyed JSON and re-indexing.

4. **Fighting RePoE's mod structure**: RePoE's mod definitions include all the fields we need, but also much we don't. The `is_essence_only` field was added by RePoE Fork specifically -- it's not clear if vanilla RePoE has it. With GGPK, we'd read `Mods.dat` and `ModDomain.dat` directly and apply filtering at extraction time.

### Transformations That Would STILL Be Needed Regardless of Source

1. **Rollable mod filtering**: No matter where the data comes from, we need to distinguish normal rollable mods from unique/essence/fossil/crafted mods. This business logic is inherent to the problem.

2. **Tier table computation**: Sorting mods by stat range and assigning tiers is an algorithmic requirement regardless of data source.

3. **Mod group awareness**: The mod-group-aware tier calculation is a game mechanics issue, not a data format issue.

4. **Template extraction from clipboard text**: Converting "+45(42-45)% to Cold Resistance" into a lookupable template is parser logic, not data logic.

5. **Spawn weight / base tag matching**: Determining which mods can roll on which bases requires cross-referencing mod spawn weights with base item tags. This join is always needed.

6. **Roll quality calculation**: Simple math that's always needed.

### What Our Ideal Data Format Would Look Like

If we could design the data from scratch for our parser's needs:

```rust
// Pre-indexed by template string (our primary lookup key)
struct StatDatabase {
    /// template_string → StatEntry
    templates: HashMap<String, StatEntry>,
    /// base_name → BaseEntry
    bases: HashMap<String, BaseEntry>,
}

struct StatEntry {
    stat_ids: Vec<String>,
    value_count: usize,
    /// Pre-filtered, pre-sorted tier tables per base tag combination
    /// Key: sorted Vec<tag> as a cache key
    tier_tables: HashMap<Vec<String>, Vec<TierEntry>>,
    /// Mod groups that provide this stat
    mod_groups: Vec<ModGroupInfo>,
}

struct ModGroupInfo {
    groups: Vec<String>,
    tiers: Vec<TierEntry>,  // Tier table for this specific mod group
    spawn_tags: Vec<(String, u32)>,  // Which bases this group can appear on
}

struct TierEntry {
    tier: u8,
    min: f32,
    max: f32,
    required_level: u32,
    mod_name: String,
}

struct BaseEntry {
    name: String,
    item_class: String,
    tags: Vec<String>,
    implicits: Vec<String>,
}
```

Key differences from RePoE:
- **Template-keyed primary index** (not stat_id-keyed)
- **Pre-filtered to rollable mods only** (no runtime filtering)
- **Pre-computed tier tables per base tag combination** (no runtime tier calculation for common cases)
- **Mod group information embedded in stat entries** (no separate mod group detection step)
- **Base items indexed by name** (not metadata path)

### Summary: The Case for a Custom Data Pipeline

| Aspect | RePoE Approach | Custom GGPK Pipeline |
|--------|---------------|---------------------|
| Data size loaded at runtime | ~20MB mods.json + large base_items.json | Only what we need |
| Startup time | Parse multi-MB JSON + build indexes + compile regexes | Load pre-processed binary/compact format |
| Filtering logic | ~60 lines of heuristic rules, evolved over 5 sessions, fragile name-based filtering | Applied at extraction time, can be more precise with direct .dat access |
| Lookup direction | Inverted at build time (ETL) and runtime (regex compilation) | Built in the correct direction from the start |
| Duplication | BaseItemDatabase + BaseTypeManager (same data, two modules) | Single source of truth |
| Index building | Runtime HashMap construction from JSON | Pre-computed at build time |
| Tier calculation | Runtime per-query with filtering | Can pre-compute common cases |
| Offline tooling | Dedicated `poe-data-processor` crate needed | Part of the main build pipeline |
| Update friction | Wait for RePoE/RePoE Fork to update, then run ETL | Read GGPK directly after game patch |

**Total lines of transformation code in v1:** approximately **1,050+ lines** across the data-processor crate and parser data layer (excluding tests), performing work that largely exists because RePoE's data shape doesn't match what the parser needs.

The strongest argument for a custom pipeline: **RePoE is a general-purpose data export. Our parser has very specific needs.** Every general-purpose intermediary adds a translation layer. Going from GGPK → parser-optimized format in one step eliminates the RePoE translation layer entirely.
