# Item Text Format and Mod/Tier Data Ecosystem

Research into how Path of Exile represents items as text and how to map parsed affixes back to game data.

## Part 1: Item Text Format

PoE provides two clipboard copy formats. Both start with the same header/properties structure; they differ in how modifiers are presented.

### Overall Structure (Both Formats)

```
Item Class: <class>
Rarity: <rarity>
<item_name>                    (omitted for Normal rarity)
<base_type>
--------
[<item_base_archetype>]        (e.g., "Two Handed Axe", "Staff" -- only on weapons)
[Quality: +N% (augmented)]
[Physical Damage: N-N [(augmented)]]
[Elemental Damage: N-N [(augmented)][, N-N [(augmented)]]...]
[Armour: N [(augmented)]]
[Evasion Rating: N [(augmented)]]
[Energy Shield: N [(augmented)]]
[Critical Strike Chance: N.NN% [(augmented)]]
[Attacks per Second: N.NN [(augmented)]]
[Weapon Range: N [metres]]
[Block: N%]
--------
[Requirements:]               (PoE1: separate lines; PoE2: single line)
[Level: N]
[Str: N [(augmented)]]
[Dex: N [(augmented)]]
[Int: N [(augmented)]]
--------
[Sockets: <pattern>]          (PoE1: R-G-B-R R; PoE2: S S)
--------
Item Level: N
--------
[Talisman Tier: N]
--------
[enchant lines]               (enchant)
--------
[implicit lines]              (implicit)
--------
[explicit modifier lines]
--------
[influence / special markers]
--------
[flavor text]                 (unique items only)
--------
[Corrupted]
[Fractured Item]
[Synthesised Item]
[Mirrored]
[Split]
[<Influence> Item]            (Shaper, Elder, Warlord, Hunter, Redeemer, Crusader, etc.)
[Relic Unique]
[Unmodifiable]
[Foil (<type>)]
```

Sections are separated by `--------` (8 or more dashes). Not all sections are present on every item.

### Maps: Special Properties Section

Maps replace the standard weapon/armour properties with map-specific properties:

```
Map Tier: N
Item Quantity: +N% (augmented)
Item Rarity: +N% (augmented)
Monster Pack Size: +N% (augmented)
[More Currency: +N% (augmented)]
[More Maps: +N% (augmented)]
[More Scarabs: +N% (augmented)]
--------
Monster Level: N
--------
[Reward: <type>]              (T17+ maps)
[Chance for dropped Maps to convert to:]
[  Shaper Map: N%]
[  Elder Map: N%]
[  Conqueror Map: N%]
```

Map modifiers use the same prefix/suffix header format as regular items.

---

### Advanced Format (Ctrl+Alt+C in PoE1)

The advanced format adds **modifier header lines** enclosed in `{ }` before each modifier's stat lines. This is the key differentiator -- it tells you the mod name, type, tier, and tags.

#### Modifier Header Format

The general pattern is:

```
{ <ModType> [Modifier] ["<ModName>"] [(Tier: N | Rank: N | (<EldritchTier>))] [-- <tags>] }
```

Each element in detail:

**Modifier types observed:**

| Header pattern | Meaning |
|---|---|
| `Prefix Modifier "<name>" (Tier: N)` | Standard prefix affix |
| `Suffix Modifier "<name>" (Tier: N)` | Standard suffix affix |
| `Implicit Modifier` | Base implicit (no name, no tier) |
| `Unique Modifier` | Unique item modifier (no name, no tier) |
| `Master Crafted Prefix Modifier "<name>"` | Benchcrafted prefix |
| `Master Crafted Suffix Modifier "<name>"` | Benchcrafted suffix |
| `Master Crafted Suffix Modifier "<name>" (Rank: N)` | Benchcrafted with rank |
| `Eater of Worlds Implicit Modifier (<tier>)` | Eldritch implicit (EoW) |
| `Searing Exarch Implicit Modifier (<tier>)` | Eldritch implicit (SE) |

**Eldritch tier names:** `Lesser`, `Greater`, `Grand`, `Exceptional`, `Exquisite`, `Perfect`

**Tag string:** Comma-separated modifier tags after the em dash (`--`). Can be absent entirely (no dash at all). Examples:
- `Defences, Armour, Energy Shield`
- `Damage, Physical, Attack`
- `Elemental, Cold, Resistance`
- `Life`
- `Attribute`
- `Attack, Speed`
- `Aura`
- (empty/absent -- some modifiers have no tags, e.g., `{ Suffix Modifier "of the Skilled" (Tier: 2) }`)

**Regex for the header line content (inside the braces):**

```
# Standard prefix/suffix
^(Prefix|Suffix) Modifier "(.+?)" \(Tier: (\d+)\)(?: — (.+?))?$

# Master crafted
^Master Crafted (?:Prefix |Suffix )?Modifier "(.+?)"(?: \(Rank: (\d+)\))?(?: — (.+?))?$

# Implicit
^Implicit Modifier(?: — (.+?))?$

# Unique
^Unique Modifier(?: — (.+?))?$

# Eldritch influence
^(Eater of Worlds|Searing Exarch) Implicit Modifier \((.+?)\)(?: — (.+?))?$
```

**The `{ }` wrapper regex:**
```
^\{ (.+?) \}$
```

#### Modifier Value Format (Advanced)

In advanced format, each stat line includes the rolled value followed by the tier range in parentheses:

```
<rolled_value>(<min>-<max>)
```

**Patterns observed:**

| Example | Meaning |
|---|---|
| `+101(86-145) to Armour` | Value 101, tier range 86-145 |
| `80(80-91)% increased Armour and Energy Shield` | Value 80, tier range 80-91 |
| `85(64-97) to 141(97-145) Physical Thorns damage` | Two values, each with range |
| `+44(0-60) to maximum Life` | Value 44, range 0-60 (unique mod) |
| `-9(-25-50)% to Cold Resistance` | Negative value, range -25 to 50 |
| `0.34(0.2-0.4)% of Physical Attack Damage...` | Decimal value |
| `1(10--10)% reduced Quantity of Items found` | Range 10 to -10 (Ventor's) |
| `150(80)% faster start of Energy Shield Recharge` | Fixed value (single number in parens) |
| `Adds 19(14-21) to 34(32-38) Physical Damage` | "Adds X to Y" with two ranges |

**Value extraction regex:**
```
([-+]?\d+(?:\.\d+)?)(?:\((-?\d+(?:\.\d+)?)-(-?\d+(?:\.\d+)?)\))?
```

This captures: (value, optional_min, optional_max). Apply it globally to get all value groups in a line.

#### Hybrid Mods (Multiple Stat Lines Under One Header)

A single modifier header can be followed by **multiple stat lines**. This is how hybrid mods are represented in advanced format. The header appears once, and all stat lines below it (until the next header or separator) belong to that same mod.

**Example -- hybrid armour + stun recovery:**
```
{ Prefix Modifier "Mammoth's" (Tier: 1) -- Defences }
39(39-42)% increased Armour
16(16-17)% increased Stun and Block Recovery
```

**Example -- hybrid bleed mod:**
```
{ Suffix Modifier "of Haemophilia" (Tier: 2) -- Damage, Physical, Attack, Ailment }
Attacks have 25% chance to cause Bleeding
(Bleeding deals Physical Damage over time...)
38(31-40)% increased Damage with Bleeding
```

**Example -- hybrid added damage:**
```
{ Prefix Modifier "Gleaming" (Tier: 5) -- Damage, Physical, Attack }
Adds 19(14-21) to 34(32-38) Physical Damage (fractured)
```

Note: Lines in parentheses that begin with `(` are **reminder text** / tooltips, not separate stat lines. They should be associated with the preceding stat line but not parsed as numeric values.

#### Unscalable Values

Some stat lines end with `-- Unscalable Value`. This indicates the value is fixed and cannot be modified by passives or other scaling mechanics:

```
Can have up to 3 Crafted Modifiers -- Unscalable Value (crafted)
Hits can't be Evaded -- Unscalable Value (crafted)
All Monster Damage from Hits always Ignites -- Unscalable Value
```

#### Stat Line Suffixes (Special Type Markers)

Stat lines can end with a parenthetical marker indicating their special type:

| Marker | Meaning |
|---|---|
| `(fractured)` | Fractured modifier (cannot be changed) |
| `(crafted)` | Master crafted modifier |
| `(implicit)` | Implicit modifier |
| `(enchant)` | Lab enchantment or anointment |
| `(rune)` | PoE2 rune socket modifier |
| `(augmented)` | Value modified by other effects (on properties, not mods) |

These markers appear on the stat line itself, not the header. A fractured mod still has a normal `Prefix Modifier` header; the `(fractured)` marker is on the value line.

---

### Influenced Items

Influenced items have:
1. **Influence-specific modifier headers** (Eldritch only -- see above)
2. **Influence markers at the bottom** of the item text

```
Searing Exarch Item
Eater of Worlds Item
Shaper Item
Elder Item
Warlord Item
Hunter Item
Redeemer Item
Crusader Item
```

An item can have multiple influence markers (e.g., dual-influenced). Conqueror influences (Warlord, Hunter, Redeemer, Crusader) add special prefix/suffix mods that use standard `Prefix Modifier` / `Suffix Modifier` headers but may have influence-related tags.

Eldritch influences (Searing Exarch, Eater of Worlds) add implicit modifiers with their own header format and a tier name instead of a tier number.

### Fractured Items

Fractured items have:
1. A `(fractured)` marker on the stat lines of fractured mods
2. A `Fractured Item` marker at the bottom
3. The modifier header is still a normal `Prefix Modifier` or `Suffix Modifier` -- the header does NOT indicate fracturing

### Synthesised Items

Synthesised items have:
1. The base type name prefixed with "Synthesised" (e.g., `Synthesised Two-Stone Ring`)
2. Synthesised implicit modifiers (use standard `Implicit Modifier` header)
3. A `Synthesised Item` marker at the bottom

### Talisman Items

Talismans have:
1. A `Talisman Tier: N` line (its own section between separators)
2. An implicit modifier from the talisman base
3. Can be corrupted (common) and anointed

### Enchanted Items

Enchanted items have enchantments that appear as separate lines with `(enchant)` marker. These are in their own section before implicits. Lab enchantments and passive skill allocations (anointments) both use this format:

```
Quality does not increase Defences
(Defences are Armour, Evasion Rating and Energy Shield) (enchant)
Grants +1 to Maximum Life per 2% Quality (enchant)
```

```
Allocates Entropy (enchant)
```

```
Delirium Reward Type: Armour (enchant)
Players in Area are 20% Delirious (enchant)
```

### Unique Items

Unique items have:
1. `Unique Modifier` headers (no name, no tier number)
2. Tags may or may not be present
3. Flavor text after the last separator (multi-line, free-form)
4. Value ranges in advanced format show the unique-specific range (e.g., `+44(0-60)`)
5. Some unique mods have `Relic Unique` at the bottom (legacy items from reliquary keys)

---

### Simple Format (Ctrl+C)

The simple format is identical in structure for headers, properties, requirements, sockets, and item level. The differences are in the modifier section:

**What is MISSING in simple format:**
1. No `{ }` modifier header lines (no mod names, no tiers, no tags)
2. No value ranges (just the rolled value: `+101 to Armour` instead of `+101(86-145) to Armour`)
3. No modifier type classification (prefix vs suffix is not indicated)
4. Hybrid mods are NOT broken out -- in simple format, a hybrid mod like "increased Armour + increased Stun Recovery" is shown as a combined single line if that is how the game displays it, or the lines appear without grouping

**What IS still present in simple format:**
1. `(fractured)`, `(crafted)`, `(implicit)`, `(enchant)`, `(rune)` markers on stat lines
2. Influence markers at the bottom (`Shaper Item`, etc.)
3. `Fractured Item`, `Synthesised Item`, `Corrupted`, etc.
4. Flavor text for unique items
5. All the actual stat text (just without ranges)

**Example comparison:**

Advanced:
```
{ Prefix Modifier "Beatified" (Tier: 1) -- Defences, Armour, Energy Shield }
+101(86-145) to Armour
+46(29-48) to maximum Energy Shield
```

Simple:
```
+101 to Armour
+46 to maximum Energy Shield
```

In simple format, you cannot tell these two lines are from the same hybrid mod. You also cannot tell the tier or the roll range.

**What can be inferred from simple format:**
- Modifier type can be guessed from the stat text using game data (RePoE stat_translations)
- Tier can be approximately determined by looking up the value in the mod database
- Whether a mod is implicit, crafted, fractured, etc. (from markers)
- Base type and item class (from header)
- Whether influenced, corrupted, synthesised (from footer markers)

---

### PoE2 Differences

Based on the PoE2 advanced format sample in the test data:

1. **Advanced copy is available in PoE2** -- same `{ }` header format as PoE1
2. **Requirements format differs**: `Requires: Level 65, 60 (augmented) Str, 60 (augmented) Dex` (single line, comma-separated)
3. **Sockets use `S`**: `Sockets: S S` (spirit sockets, no color linking)
4. **Rune modifiers**: `+14% to Fire Resistance (rune)` -- appears in its own section before implicits
5. **Implicit modifiers**: Use the same `{ Implicit Modifier }` header format
6. **Standard prefix/suffix**: Same format as PoE1 with tier numbers and ranges
7. **Different mod pool**: PoE2 has different mods (e.g., Physical Thorns damage), different tier numbers
8. **`Weapon Range: 1.4 metres`**: May include unit suffix (PoE1 uses bare number)

PoE2 does NOT have:
- Eldritch influences (Searing Exarch / Eater of Worlds)
- Fractured / Synthesised items
- Complex socket linking (R-G-B-R patterns)
- Multiple influence types

---

## Part 2: Mod/Tier Data Mapping

### RePoE Fork Data Files

Available at `https://repoe-fork.github.io/{filename}.json`. Key files for mod mapping:

| File | Purpose | Size |
|---|---|---|
| `mods.json` | All item modifiers with stats, tiers, spawn weights, groups | ~20MB |
| `stat_translations.json` | Maps internal stat IDs to display text templates | ~5MB |
| `base_items.json` | Base item types with tags, classes, domains | ~3MB |
| `stats.json` | Raw stat definitions | |
| `crafting_bench_options.json` | Benchcraft options and costs | |
| `essences.json` | Essence crafting outcomes | |
| `fossils.json` | Fossil crafting modifiers | |
| `item_classes.json` | Item class definitions | |

### mods.json Structure

The file is a flat JSON object keyed by mod ID (metadata path). Each entry:

```json
{
  "ModId": {
    "name": "Hale",                        // Display name (used in advanced copy header)
    "text": "+# to maximum Life",          // Display text template (not always present)
    "stats": [                             // Stat values for this specific mod entry
      {
        "id": "base_maximum_life",         // Internal stat ID
        "min": 40,                         // Minimum roll value
        "max": 49                          // Maximum roll value
      }
    ],
    "required_level": 44,                  // Item level requirement to roll
    "generation_type": "prefix",           // "prefix", "suffix", "unique", "corrupted", etc.
    "spawn_weights": [                     // Which item types this can appear on
      { "tag": "default", "weight": 1000 },
      { "tag": "body_armour", "weight": 1000 },
      { "tag": "abyss_jewel", "weight": 0 }    // weight=0 means blocked
    ],
    "groups": ["IncreasedLife"],           // Mod group (items can only have one mod per group)
    "domain": "item",                      // "item", "flask", "abyss_jewel", "crafted", etc.
    "is_essence_only": false,              // Explicit flag for essence-only mods
    "generation_weights": [],              // Conditional spawning (e.g., influenced items)
    "adds_tags": []                        // Tags this mod adds to the item
  }
}
```

**Important:** Multiple mod entries can share the same stat ID but with different value ranges. These represent different tiers of the same stat. For example, there are ~10 entries for `base_maximum_life` with increasing ranges (1-9, 10-19, 20-29, ... 100-109), each with a different `name` ("Hale", "Healthy", "Sanguine", ... "Kaom's").

**The `name` field corresponds directly to the quoted name in the advanced format header:** `{ Prefix Modifier "Hale" (Tier: 7) }`.

### stat_translations.json Structure

An array of translation entries. Each entry maps one or more internal stat IDs to display text:

```json
{
  "English": [
    {
      "condition": [{ "min": 1, "max": null, "negated": null }],
      "format": ["#"],
      "index_handlers": [[]],
      "string": "{0}% increased Attack Speed",
      "reminder_text": null,
      "is_markup": null
    },
    {
      "condition": [{ "min": null, "max": -1, "negated": null }],
      "format": ["#"],
      "index_handlers": [["negate"]],
      "string": "{0}% reduced Attack Speed",
      "reminder_text": null,
      "is_markup": null
    }
  ],
  "ids": ["local_attack_speed_+%"],
  "trade_stats": null,
  "hidden": null
}
```

Key points:
- **`ids`**: Array of internal stat IDs. For single-stat mods, length 1. For multi-stat mods (like "Adds {0} to {1} Lightning Damage"), length 2+.
- **`string`**: Display template with `{0}`, `{1}` placeholders for numeric values
- **`condition`**: When this variant applies (positive values, negative values, etc.)
- **`index_handlers`**: Operations on placeholders. `"negate"` means the displayed value is the negation of the internal value (e.g., internal +10 = "10% reduced")
- **Multiple `English` entries**: Different display strings for the same stat depending on conditions (positive vs. negative, different thresholds)

### base_items.json Structure

Keyed by metadata path, each entry:

```json
{
  "Metadata/Items/Weapons/.../TwoHandAxe17": {
    "name": "Vaal Axe",
    "item_class": "Two Hand Axe",
    "tags": ["weapon", "two_hand_weapon", "axe", "default"],
    "domain": "item",
    "drop_level": 64,
    "implicits": ["..."],
    "inventory_height": 4,
    "inventory_width": 2,
    "inherits_from": "...",
    "release_state": "released"
  }
}
```

The `tags` array is critical for mod pool filtering: a mod's `spawn_weights` reference these tags to determine whether it can appear on a given base.

---

### The Mapping Pipeline: Affix Text to Game Data

Given an affix line from item text, here is the full pipeline to identify it in the game data:

#### Step 1: Extract Template from Text

Strip numeric values and replace with placeholders:

```
"+45(42-45)% to Cold Resistance"  -->  "{0}% to Cold Resistance"
"Adds 19(14-21) to 34(32-38) Physical Damage"  -->  "Adds {0} to {1} Physical Damage"
```

Rules:
- Replace each numeric value (including its optional range suffix) with `{N}` where N is the occurrence index
- Strip leading `+` (positive is default, not semantically meaningful)
- Preserve leading `-` (negative IS meaningful)
- Normalize whitespace

#### Step 2: Look Up stat_id via Template

Use the pre-built template-to-stat-id lookup table (derived from `stat_translations.json`):

```
"{0}% to Cold Resistance"  -->  stat_ids: ["base_cold_damage_resistance_%"]
"Adds {0} to {1} Physical Damage"  -->  stat_ids: ["local_minimum_added_physical_damage", "local_maximum_added_physical_damage"]
```

This gives the internal stat ID(s) for the modifier.

#### Step 3: Find Matching Mod in mods.json

With the stat_id and the rolled value, search `mods.json` for entries where:
1. One of the `stats` has a matching `id`
2. The rolled value falls within that stat's `[min, max]` range
3. The mod is "rollable" (see filtering below)
4. The mod can spawn on this item's base type (spawn_weights match base tags)

#### Step 4: Determine Tier

All mods sharing the same stat_id and generation_type (prefix/suffix) form a **tier table**. Sort by max value descending: the highest range is Tier 1, next is Tier 2, etc.

```
T1: 100-109  (Kaom's)
T2:  90-99   (Prime)
T3:  80-89   (Athlete's)
...
T7:  40-49   (Hale)
```

The tier number in the advanced format header (`(Tier: 7)`) corresponds to this ordering.

#### Step 5: Calculate Roll Quality

Given the value and the tier's min/max range:

```
quality = (value - min) / (max - min)     // 0.0 = worst, 1.0 = perfect
```

For fixed-value mods (min == max), quality is always 1.0.

---

### How Advanced Format Makes This Easier

With advanced format, Steps 1-4 are partially short-circuited:

| Information | Advanced format provides | Still needed |
|---|---|---|
| Mod name | `"Hale"` in header | Can cross-reference with mods.json `name` field |
| Mod type | `Prefix` / `Suffix` in header | Direct |
| Tier number | `(Tier: 7)` in header | Can verify against calculated tier |
| Tier range | `40(40-49)` inline | `min=40, max=49` directly from text |
| Tags | `Life` after em dash | Informational, useful for filtering |
| Stat value | `40` (the number before parentheses) | Direct |

With advanced format, you can:
- **Skip the template lookup entirely** for tier range (it is in the text)
- **Verify tier** by comparing the text tier to the calculated tier from the database
- **Identify hybrid mods** unambiguously (grouped under one header)
- **Distinguish prefix from suffix** without guessing

With simple format, you must:
- Do the full template lookup pipeline
- Guess prefix vs. suffix from game data
- Cannot reliably detect hybrid mods
- Cannot determine exact tier range (only approximate tier from value)

---

### Filtering to Rollable Mods

Not all entries in `mods.json` are relevant for tier calculation. The filtering criteria (proven in the poe-inspect v1 codebase):

1. **generation_type** must be `"prefix"` or `"suffix"` (excludes `"unique"`, `"corrupted"`, `"enchantment"`, etc.)
2. **domain** must be `"item"` (excludes `"flask"`, `"abyss_jewel"`, `"crafted"`, `"unveiled"`, etc.)
3. **spawn_weights** must have at least one entry with `weight > 0`
4. **is_essence_only** must be `false`
5. **Name-based exclusions**: Exclude mods with names containing "fossil", "harvest", "veiled", "delve", "synthesis", "betrayal", "legion", "fractured"
6. **Base tag filtering**: When calculating tiers for a specific item, only include mods whose spawn_weights match at least one of the base item's tags

Note: Benchcrafted mods have `domain: "crafted"` and are excluded from the rollable tier table. The advanced format marks them with `Master Crafted` in the header and `(crafted)` on the stat line, and they have `(Rank: N)` instead of `(Tier: N)`.

### Mod Groups

The `groups` field in mods.json defines **mod groups**. An item cannot have two mods from the same group. This is important for:
- Tier calculation accuracy (group-aware tier tables)
- Crafting potential analysis (which mods can still be added)
- Identifying which specific mod variant rolled (when multiple mod groups share a stat)

Example: "IncreasedAttackSpeed" group vs. "IncreasedAttackSpeedTwoHandedAxe" group -- both affect attack speed but are different mod groups with different tier tables.

### Multi-Stat Mods

Some mods have multiple stats (hybrid mods). In `mods.json`, these have multiple entries in the `stats` array. In `stat_translations.json`, they map to multiple `ids`. In the item text:

- **Advanced format**: Multiple stat lines under one `{ }` header
- **Simple format**: Multiple separate lines (without grouping) or combined display text

For tier determination of multi-stat mods, typically the **primary stat** (first in the stats array) determines the tier. The secondary stats are fixed to the same tier entry.

---

### Existing Tools and Libraries

#### RePoE / RePoE Fork
- **Repository**: `https://github.com/brather1ng/RePoE` (original), `https://github.com/repoe-fork/repoe-fork.github.io` (fork with GitHub Pages hosting)
- **Data URL pattern**: `https://repoe-fork.github.io/{filename}.json`
- Provides the raw game data extracted from PoE's content files
- Updated with each game patch

#### poe-item-filter (Prior Art)
- Contains a data pipeline that fetches from repoe-fork, processes, and caches
- Rust backend with economy data integration

#### poe-inspect v1 (Prior Art)
- **poe-parser crate** (`packages/poe-parser/`): Full parser implementation in Rust
  - Line-by-line state machine parser
  - Regex patterns for all format elements
  - Template extraction and stat_id lookup via pre-processed lookup table
  - ModDatabase for tier calculation with base-tag-aware filtering
  - BaseItemDatabase for base type tag resolution
  - Roll quality calculation and divine target analysis
- **poe-data-processor crate** (`packages/poe-data-processor/`): Transforms raw RePoE stat_translations.json into an optimized template-to-stat-id lookup table
- **Pre-processed data**: `data/processed/poe1/template-stat-lookup.json` (template string -> stat_ids + value_count)
- **Raw game data**: `data/game-data/poe1/` contains `mods.json`, `stat_translations.json`, `base_items.json`, `stats.json`

#### Known Rust Crates for PoE Data
No widely-adopted standalone Rust crates exist specifically for PoE item parsing or RePoE data consumption. The poe-inspect v1 parser is custom. The Rust ecosystem for PoE tooling is thin compared to TypeScript/Python.

#### Community Tools That Do Mod Mapping
- **Awakened PoE Trade**: Uses a combination of trade API stat IDs and local mod data for price checking. Written in TypeScript/Electron.
- **poe-trade-companion tools**: Various community tools parse item text for trade searches
- **poedb.tw**: Comprehensive web-based mod browser with tier tables, but no public API
- **Craft of Exile**: Crafting simulator that uses RePoE data internally
- **Path of Building**: Imports items and resolves mods, but uses its own Lua-based data format

---

### Key Architectural Decisions for poe-inspect-2

Based on this research, the recommended pipeline is:

1. **Parse item text** (line-by-line state machine, as proven in v1)
2. **Extract templates** from modifier text (strip values, replace with placeholders)
3. **Look up stat_ids** via pre-built template lookup table (from stat_translations.json)
4. **Load base item tags** for the parsed base type (from base_items.json)
5. **Calculate tiers** using mods.json filtered by: rollable mods, matching stat_id, matching base tags, matching mod group
6. **Calculate roll quality** from value position within tier range
7. **Verify against advanced format data** when available (tier number, mod name, range)

The advanced format provides enough inline data (tier, range, mod name) to skip most of the database lookup for display purposes. The database is needed for:
- Enriching simple format items
- Finding ALL tiers for a stat (not just the current one)
- Determining how many open prefix/suffix slots remain
- Crafting potential analysis
- Cross-referencing with trade API stat IDs
