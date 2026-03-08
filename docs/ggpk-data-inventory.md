# GGPK Data Inventory

Validated against PoE 3.28 Mirage (2026-03-07).

## Overview

The GGPK contains **911 datc64 tables** (English) mirrored across **9 languages**, plus **41 stat description text files**.

Top-level directories in the bundle index: `art/`, `audio/`, `cachedhlslshaders/`, `data/`, `metadata/`, `minimap/`, `shaders/`.

Total indexed paths: 1,150,611.

## Tables We Need (~15 of 911)

These are the tables required for item parsing and evaluation. Listed in dependency order.

### Core Item Data

| Table | Rows | Size | Purpose |
|-------|------|------|---------|
| `BaseItemTypes` | 5,334 | 2.4MB | Item base identification (Id, Name, ItemClass, DropLevel, ImplicitMods, Tags) |
| `ItemClasses` | — | 21KB | Item class hierarchy (BodyArmour, Ring, etc.) |
| `ItemClassCategories` | — | 4.5KB | Category grouping (Armour, Weapon, etc.) ✅ extracted |
| `Rarity` | ~4 | 358B | Max prefix/suffix counts per rarity ✅ extracted |
| `WeaponTypes` | — | 15KB | Base stats for weapons (damage, speed, crit) |
| `ArmourTypes` | — | 29KB | Base stats for armour (armour, evasion, ES) |
| `ShieldTypes` | — | 2KB | Base stats for shields |

### Mod System

| Table | Rows | Size | Purpose |
|-------|------|------|---------|
| `Mods` | 39,291 | 32MB | Mod definitions: stat IDs, ranges (Min/Max), spawn weights, tags, domain, generation type |
| `ModFamily` | — | 542KB | Mod groups (e.g., all "IncreasedLife" mods) |
| `ModType` | — | 2MB | Mod type classification |
| `Stats` | 22,749 | 4.4MB | Stat definitions (Id, IsLocal, Semantics) |
| `Tags` | — | 91KB | Tag system (for mod spawn weights) |

### Crafting

| Table | Rows | Size | Purpose |
|-------|------|------|---------|
| `CraftingBenchOptions` | — | 402KB | Bench craft recipes (costs, mod applied, item class restrictions) |
| `Essences` | — | 112KB | Essence → forced mod mappings |

### Stat Descriptions (Text Files, Not datc64)

| File | Size | Purpose |
|------|------|---------|
| `stat_descriptions.txt` | 30MB | **Master file** — maps stat IDs to display text. This is how we reverse-lookup item text → stat IDs |
| `advanced_mod_stat_descriptions.txt` | 217KB | Advanced mod display (tier info overlay) |

The stat description files live at `metadata/statdescriptions/*.txt` and use a custom format (see `docs/research/stat-description-format.md`).

### Supporting / Maybe

| Table | Size | Purpose |
|-------|------|---------|
| `CurrencyItems` | 610KB | Currency item data |
| `SkillGems` | 147KB | Gem data (if we evaluate gem items) |
| `Flasks` | 6.5KB | Flask base data |
| `Rarity` | 358B | ~~Moved to Core Item Data~~ (max prefix/suffix counts extracted) |
| `Words` | 559KB | Word lists (used in magic item name generation) |

## Localization

Every datc64 table has a corresponding datcl64 (localized) version. Languages available:

- French, German, Japanese, Korean, Portuguese, Russian, Spanish, Thai, Traditional Chinese

Localized tables live at `data/{language}/{table}.datc64` (e.g., `data/french/mods.datc64`).

poe-query's `get_filepath()` already handles this — pass `language: "French"` to `DatReader::from_install()`.

## All 41 Stat Description Files

```
active_skill_gem_stat_descriptions.txt      (934KB)
advanced_mod_stat_descriptions.txt          (217KB)
atlas_relic_stat_descriptions.txt           (10KB)
atlas_stat_descriptions.txt                 (2.8MB)
aura_skill_stat_descriptions.txt            (248KB)
banner_aura_skill_stat_descriptions.txt     (55KB)
beam_skill_stat_descriptions.txt            (11KB)
brand_skill_stat_descriptions.txt           (34KB)
buff_skill_stat_descriptions.txt            (156KB)
chest_stat_descriptions.txt                 (197KB)
curse_skill_stat_descriptions.txt           (221KB)
debuff_skill_stat_descriptions.txt          (67KB)
expedition_relic_stat_descriptions.txt      (446KB)
gem_stat_descriptions.txt                   (2.8MB)
graft_stat_descriptions.txt                 (363KB)
heist_equipment_stat_descriptions.txt       (329KB)
leaguestone_stat_descriptions.txt           (506KB)
map_stat_descriptions.txt                   (468KB)
mercenary_support_stat_descriptions.txt     (160KB)
minion_attack_skill_stat_descriptions.txt   (647KB)
minion_skill_stat_descriptions.txt          (1.1MB)
minion_spell_damage_skill_stat_descriptions.txt (359KB)
minion_spell_skill_stat_descriptions.txt    (29KB)
mirage_stat_descriptions.txt                (134KB)
monster_stat_descriptions.txt               (44KB)
necropolis_stat_descriptions.txt            (458KB)
offering_skill_stat_descriptions.txt        (41KB)
passive_skill_aura_stat_descriptions.txt    (75KB)
passive_skill_stat_descriptions.txt         (355KB)
primordial_altar_stat_descriptions.txt      (428KB)
sanctum_relic_stat_descriptions.txt         (128KB)
secondary_debuff_skill_stat_descriptions.txt (5KB)
sentinel_stat_descriptions.txt              (393KB)
single_minion_spell_skill_stat_descriptions.txt (143KB)
skill_stat_descriptions.txt                 (5.5MB)
skillpopup_stat_filters.txt                 (474KB)
stat_descriptions.txt                       (30MB)
tincture_stat_descriptions.txt              (307KB)
vaal_side_area_stat_descriptions.txt        (306B)
variable_duration_skill_stat_descriptions.txt (14KB)
village_stat_descriptions.txt               (156KB)
```

## Schema Maintenance

### The Problem

GGG never publishes table schemas. The community (`poe-tool-dev/dat-schema`) reverse-engineers them by comparing row sizes between patches and probing new bytes. When a new league launches, schemas break — new fields are added, row sizes change.

### Our Approach

1. **Community schema is a starting point, not a dependency.** We use `poe-tool-dev/dat-schema` as our baseline but must be prepared to fix schemas ourselves.

2. **New fields are almost always appended.** The fields we care about (Id, Name, stats, ranges, tags) are early in the row layout and haven't shifted in years. Schema mismatches typically only affect the last N bytes.

3. **Row size comparison is the primary signal.** When PoE patches:
   - Run `check_index` / `list_dat` to get new file sizes
   - Compare row size: `(file_size - 8) / row_count` vs schema expected size
   - Difference tells you how many bytes were added/removed

4. **Patch notes guide field identification.** If a patch adds "Mirage tags" to mods, and Mods.datc64 gains 8 bytes/row, that's likely a new foreign key to a MirageTags table.

5. **Probe unknown bytes.** Read the new bytes as different types (u32, u64, bool, list) and cross-reference with known data to identify what they represent.

6. **Tolerance over precision.** For the fields we actually use, we can read them even when the schema is slightly wrong — we just need correct offsets for OUR fields, not every field.

### Current Schema Status (3.28 Mirage)

| Table | Schema Match |
|-------|-------------|
| Stats | 8 bytes short (schema needs 1 new field) |
| BaseItemTypes | 8 bytes short (schema needs 1 new field) |
| Mods | 21 bytes over (schema has stale field sizes or extra fields) |

These mismatches don't prevent reading the core fields we need.

## GGPK Format Notes

- **File format**: `.datc64` (data), `.datcl64` (localized data)
- **Path format**: All lowercase in index (e.g., `data/mods.datc64`)
- **Hash algorithm**: MurmurHash64A with seed `0x1337b33f` (changed in 3.21.2)
- **Compression**: Oodle (C++ FFI via libooz)
- **Index**: `Bundles2/_.index.bin` (or embedded in `Content.ggpk` for standalone installs)
- **Stat descriptions**: Plain text at `metadata/statdescriptions/*.txt`, custom format with `include` directives
