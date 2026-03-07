# Stat Description File Format

Extracted and analyzed from PoE 3.28 Mirage (2026-03-07).

## Overview

The stat description files are **the translation layer** between internal stat IDs and human-readable item text. They contain ALL languages inline — English plus 10 localizations per entry. There is no separate translation file; this IS the translation file.

The master file is `metadata/statdescriptions/stat_descriptions.txt` (30MB, 429k lines, UTF-16LE encoded in GGPK). Other stat description files include this master via `include` directives and add domain-specific overrides.

**This file is the key to reverse-mapping item text back to stat IDs.** When a player sees `+92 to maximum Life`, this file tells us the stat is `base_maximum_life` with value `92`, matched via the template `{0:+d} to maximum Life`.

## File Hierarchy

```
stat_descriptions.txt (master, 30MB)
  └── included by: gem_stat_descriptions.txt
        └── included by: active_skill_gem_stat_descriptions.txt
  └── included by: skill_stat_descriptions.txt
  └── included by: passive_skill_stat_descriptions.txt
  └── included by: map_stat_descriptions.txt
  └── included by: (many others)
```

Files higher in the chain override descriptions from included files. If `skill_stat_descriptions.txt` redefines a stat, its version takes precedence over the one in `stat_descriptions.txt`.

## Grammar Structure

The file consists of three top-level constructs:

### 1. `description` block

```
description
    <stat_count> <stat_id_1> [stat_id_2 ...]
    <variant_count>
        <range_1> [<range_2> ...] "format string" [transforms...] [reminderstring <id>]
        <range_1> [<range_2> ...] "format string" [transforms...]
    lang "LanguageName"
    <variant_count>
        <range_1> [<range_2> ...] "format string" [transforms...]
        ...
    [lang "AnotherLanguage"
    ...]
```

### 2. `no_description` directive

```
no_description <stat_id>
```

Marks a stat as having no visible text (internal-only stats like `level`, `item_drop_slots`).

### 3. `include` directive

```
include "Metadata/StatDescriptions/other_file.txt"
```

Includes another stat description file. Descriptions in the current file override those from included files.

### 4. `no_identifiers` directive

```
no_identifiers
```

Appears after includes, purpose unclear (possibly marks that subsequent entries don't have row-ID references).

## Range Syntax

Each variant line specifies value ranges that determine when this text template applies:

| Range | Meaning |
|-------|---------|
| `#` | Any value (wildcard) |
| `5` | Exactly 5 |
| `1\|#` | >= 1 (1 to infinity) |
| `#\|-1` | <= -1 (negative infinity to -1) |
| `-1\|1` | -1 to 1 (inclusive) |
| `0` | Exactly 0 (often used to suppress display) |

For multi-stat descriptions, ranges are space-separated, one per stat:
```
5 # 1 #    — first stat == 5, second any, third >= 1, fourth any
```

## Format Specifiers

| Specifier | Meaning |
|-----------|---------|
| `{0}` | First stat value |
| `{1}` | Second stat value |
| `{0:+d}` | First stat, signed integer with explicit `+` sign |
| `{0:d}` | First stat, integer |

## Value Transforms

Applied after the format string, these modify how the raw stat value is displayed:

### Unit conversions
| Transform | Effect |
|-----------|--------|
| `milliseconds_to_seconds N` | Divide stat N by 1000, display as seconds |
| `deciseconds_to_seconds N` | Divide stat N by 10, display as seconds |
| `per_minute_to_per_second N` | Divide stat N by 60 |

### Arithmetic
| Transform | Effect |
|-----------|--------|
| `negate N` | Flip sign of stat N (turns -15 into 15 for "reduced" text) |
| `double N` | Multiply stat N by 2 |
| `negate_and_double N` | Negate then double |
| `divide_by_three N` | Divide stat N by 3 |
| `divide_by_four N` | Divide stat N by 4 |
| `divide_by_five N` | Divide stat N by 5 |
| `divide_by_six N` | Divide stat N by 6 |
| `divide_by_twelve N` | Divide stat N by 12 |
| `divide_by_twenty N` | Divide stat N by 20 |
| `divide_by_one_hundred N` | Divide stat N by 100 |
| `divide_by_one_hundred_and_negate N` | Divide by 100 and negate |
| `divide_by_one_thousand N` | Divide stat N by 1000 |
| `times_twenty N` | Multiply stat N by 20 |
| `times_one_point_five N` | Multiply stat N by 1.5 |
| `plus_two_hundred N` | Add 200 to stat N |

### Display precision
| Transform | Effect |
|-----------|--------|
| `divide_by_one_hundred_2dp_if_required N` | Divide by 100, show 2 decimal places if needed |

### Special lookups
| Transform | Effect |
|-----------|--------|
| `old_leech_percent N` | Legacy leech calculation |
| `old_leech_permyriad N` | Legacy leech (per 10,000) |
| `multiplicative_damage_modifier N` | Special damage calculation |
| `mod_value_to_item_class N` | Map value to item class name |
| `display_indexable_support N` | Map index to support gem name |
| `display_indexable_skill N` | Map index to skill name |
| `passive_hash N` | Map to passive skill name |
| `affliction_reward_type N` | Map to affliction reward name |
| `locations_to_metres N` | Convert game units to metres |
| `tree_expansion_jewel_passive N` | Timeless jewel passive lookup |
| `weapon_tree_unique_base_type_name N` | Weapon passive tree base type |

### Metadata
| Keyword | Effect |
|---------|--------|
| `canonical_line` | Marks this variant as the "primary" display form |
| `reminderstring <ReminderTextId>` | Appends reminder text (e.g., "Recently refers to past 4 seconds") |

## Complete Example

```
description
    1 base_maximum_life
    1
        # "{0:+d} to maximum Life"
    lang "Portuguese"
    1
        # "{0:+d} de Vida máxima"
    lang "Traditional Chinese"
    1
        # "{0:+d} 最大生命"
    lang "French"
    1
        # "{0:+d} de Vie maximale"
    ...
```

- 1 stat: `base_maximum_life`
- 1 variant (English): any value (`#`), display as `+92 to maximum Life`
- Same pattern repeated for each language

## Increased/Reduced Pattern

Many stats have two variants — one for positive, one for negative:

```
description
    1 maximum_life_mana_and_energy_shield_+%
    2
        1|# "{0}% increased maximum Life, Mana and Global Energy Shield"
        #|-1 "{0}% reduced maximum Life, Mana and Global Energy Shield" negate 1
```

- Variant 1: value >= 1 → "increased"
- Variant 2: value <= -1 → "reduced", with `negate 1` to display the absolute value

## Multi-Stat Example

```
description
    2 local_display_socketed_gems_minimum_added_fire_damage local_display_socketed_gems_maximum_added_fire_damage
    1
        # # "Socketed Gems deal {0} to {1} Added Fire Damage"
```

- 2 stats, both with wildcard range (`#`)
- `{0}` = minimum, `{1}` = maximum

## Encoding

- Stored as **UTF-16LE** in the GGPK bundles
- Must be converted to UTF-8 for processing
- Contains characters from all supported languages inline

## Parser Requirements

This format MUST be parsed with a formal grammar (PEG/PEST), not ad-hoc string matching. Reasons:
1. Edge cases accumulate over time — GGG adds new transforms each league
2. The range syntax has multiple forms that interact with multi-stat entries
3. Language blocks repeat the variant structure but may have different variant counts
4. Transforms chain and apply to specific stat indices
5. Hand-coded parsers become unmaintainable as patterns multiply
