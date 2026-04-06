# poe-item: Grammar vs Resolver Review

## Design Intent

The crate uses a two-pass architecture:
- **Pass 1 (PEST grammar):** Structural parsing — recognizes sections, separators, typed line formats. Produces a parse tree.
- **Pass 2 (Rust resolver):** Data-dependent disambiguation — uses GameData to resolve things the grammar can't know (base type lookups, stat ID resolution, rarity-dependent classification).

The boundary should be: **if it can be recognized by pattern alone, it's grammar. If it needs game data, it's resolver.**

## The `Section::Generic` Problem

The grammar has 11 typed section rules (requirements, sockets, item level, monster level, talisman tier, experience, modifiers, influence, status, note) and one catch-all: `generic_section`. Everything that doesn't match a typed rule becomes `Section::Generic(Vec<String>)`.

The resolver then re-parses these generic sections using hardcoded string matching to figure out what they are. This is structural parsing happening in Pass 2 that should be in Pass 1.

### What flows through `Section::Generic` today

For most items, **30-80% of content** goes through Generic:

- Equipment properties (Armour, Evasion, Energy Shield, Block, Quality, etc.)
- Weapon sub-header + properties
- Map properties (Map Tier, Item Quantity, etc.)
- Heist contract/blueprint properties
- Enchant lines
- Flask properties
- Gem tags, properties, description, stats, quality effects
- Vaal variant sections
- Usage instructions
- Flavor text
- Currency/scarab effect descriptions
- "Additional Effects From Quality" marker

## Inventory of Hardcoded Patterns

### Constants and regexes (resolver.rs)

| Pattern | Line | What it matches |
|---------|------|----------------|
| `VALUE_RANGE_RE` | 21-23 | Value annotations: `32(25-40)`, `-9(-25-50)` |
| `SUFFIX_RE` | 25-27 | Type suffixes: `(implicit)`, `(crafted)`, `(enchant)`, `(fractured)` |
| `UNSCALABLE_SUFFIX` | 30 | `" — Unscalable Value"` em-dash marker |
| `USAGE_PREFIXES` | 486-495 | 8 hardcoded strings: "Right click", "Place into", "Travel to", "Can be used", "This is a Support Gem", "Shift click to unstack", "Use Intelligence", "Give this" |

### Section classification (classify_single_section, resolver.rs:559-646)

| Check | Line | Detection method |
|-------|------|-----------------|
| Enchant section | 570 | `all(line.ends_with("(enchant)"))` |
| Usage instructions | 575 | `starts_with()` against 8 prefixes |
| Property section | 580 | `line.contains(": ")` — colon detection |
| Heist skill requirements | 584 | `starts_with("Requires ") && contains("(Level ")` |
| Weapon sub-header | 596-604 | First line has no `": "` + `is_weapon_class()` check |
| Currency mixed sections | 609 | `rarity == Currency` heuristic |
| Gem/Currency descriptions | 619 | `rarity == Currency | Gem` heuristic |
| Unique/DivCard flavor | 625 | `rarity == Unique | DivinationCard` heuristic |
| Quoted flavor text | 630 | `starts_with('"')` |
| Normal item heuristic | 637-643 | `rarity == Normal && (len <= 2 || text.len() < 80)` — fragile |

### Property parsing (parse_property_lines, resolver.rs:648-676)

| Check | Line | Detection method |
|-------|------|-----------------|
| Heist requirement format | 656 | `parse_heist_requirement()` — `starts_with("Requires ")` + `find(" (Level ")` |
| Augmented marker | 661-664 | `contains("(augmented)")` then string replace |

### Gem section handling (resolver.rs:700-813)

| Check | Line | Detection method |
|-------|------|-----------------|
| Gem section ordering | 705-717 | Assumes generic sections appear in fixed order: [1] tags+props, [2] description, [3] stats+quality |
| Tag parsing | 738-743 | First line split by `", "` = tags, rest = properties |
| Quality effects marker | 757 | `starts_with("Additional Effects From Quality")` |
| Vaal detection | 785-789 | Single-line section + `starts_with("Vaal ")` |
| Vaal section ordering | 792-804 | Assumes 4 sections in order: props, description, stats, quality |

### Stat line processing (resolve_stat_line, resolver.rs:866-899)

| Check | Line | Detection method |
|-------|------|-----------------|
| Reminder text | 867 | `starts_with('(') && ends_with(')')` |
| Unscalable marker | 868 | `ends_with(UNSCALABLE_SUFFIX)` |

### Mod processing (resolve_mod, resolver.rs:311-364)

| Check | Line | Detection method |
|-------|------|-----------------|
| Fractured body suffix | 355 | `line.ends_with("(fractured)")` |

### Display text construction (build_display_text, resolver.rs:924-934)

| Check | Line | Detection method |
|-------|------|-----------------|
| Range stripping | 926 | `VALUE_RANGE_RE.replace_all()` regex |
| Suffix stripping | 928 | `SUFFIX_RE.replace()` regex |
| Unscalable stripping | 930 | `strip_suffix(UNSCALABLE_SUFFIX)` |

## What's correctly in the resolver (needs GameData)

These are data-dependent and genuinely belong in Pass 2:

- **Magic base type extraction** — substring lookup against BaseItemTypes table
- **Stat ID resolution** — reverse index lookup for display text -> stat_id
- **Confirmed stat ID application** — cross-reference mod table for exact IDs
- **Multi-line stat joining** — join consecutive lines and re-lookup
- **Local/non-local stat pair detection** — GameData stat table comparison
- **Pseudo stat computation** — sum across mods with multipliers from poe-data definitions
- **DPS computation** — weapon property parsing for trade pseudo values
- **Rarity-dependent routing** — currency effects vs flavor text vs description (needs Rarity context)
- **Socket color parsing** — R/G/B/W counting from socket string format

## Summary

There are **~25 hardcoded string patterns** in the resolver doing structural parsing. The grammar recognizes 11 section types but leaves everything else to a catch-all. The resolver compensates with string matching, regex, section ordering assumptions, and heuristics.

The rarity-dependent classification (lines 609-643) is a gray area — it needs rarity context that the grammar doesn't have, but the pattern matching it uses (colon detection, quote detection) is structural. The gem section handling is the most fragile: it assumes a fixed section order with no validation.

## Findings (2026-04-06 cleanup)

### Item-type-specific grammar paths work

The grammar CAN match `"Rarity: Gem"` literally in the header and use a gem-specific section ordering rule (`gem_item`). This was implemented and proven across 12 gem fixtures covering all variants (skill, support, vaal, transfigured, imbued, exceptional/awakened, corrupted).

**Result:** The resolver's gem extraction code (`extract_gem_data`, `split_gem_tags_and_props`, `split_stats_and_quality`, `extract_vaal_data`) was deleted entirely. The grammar identifies every gem section by type, the tree walker builds `GemData` directly, and the resolver handles it in one line.

### Recommendation: rarity-specific grammar paths for non-equipment items

The same approach should work for **Currency** (`"Rarity: Currency"`) and **Divination Card** (`"Rarity: Divination Card"`). These have predictable section structures that the grammar can match, eliminating rarity-dependent heuristics from the resolver.

Equipment items (Normal, Magic, Rare, Unique) share the same section structure and should continue using the generic `standard_item` path with `section*` matching.

### What was moved to grammar

| Pattern | Was | Now |
|---------|-----|-----|
| Property sections (`Key: Value`) | Resolver `contains(": ")` | Grammar `property_section` |
| Enchant sections (`(enchant)` suffix) | Resolver `ends_with("(enchant)")` | Grammar `enchant_section` |
| Gem tags | Resolver `split(", ")` on first generic line | Grammar `gem_tags_line` |
| Gem description | Resolver positional (2nd generic section) | Grammar `gem_description_section` |
| Gem stats + quality | Resolver `starts_with("Additional Effects From Quality")` | Grammar `gem_stats_section` with `gem_quality_block` |
| Gem usage instructions | Resolver `USAGE_PREFIXES` check | Grammar `gem_usage_section` (dropped) |
| Vaal variant | Resolver `starts_with("Vaal ")` | Grammar `vaal_section` with `"Vaal "` prefix |
| Weapon sub-header | Resolver `is_weapon_class()` + no-colon check | Grammar `property_subheader` |

### What remains in the resolver (correctly)

- Rarity-based text classification for equipment (flavor vs description) — data-driven
- Usage instruction detection via `poe_data::domain` — domain knowledge
- Mixed heist sections — grammar can't handle mixed property/non-property lines
- Stat ID resolution, base type extraction, pseudo computation — genuinely needs GameData

### Next: rarity-specific grammar paths for all item types

The gem work proved that the grammar can use the Rarity header to route into item-type-specific section ordering. This should be extended:

| Rarity | Grammar path | Status | Notes |
|--------|-------------|--------|-------|
| Gem | `gem_item` | **Done** | 12 fixtures, all gem extraction removed from resolver |
| Currency | `currency_item` | Planned | Stack Size props, description, usage — very predictable |
| Divination Card | `divination_card_item` | Planned | Stack Size, reward text, flavor — simplest |
| Unique | `unique_item` | Planned | Like equipment but flavor text becomes grammar-identified |
| Normal/Magic/Rare | `standard_item` | Current | Shared structure (game upgrade path), use generic `section*` |

**Key insight:** Normal → Magic → Rare is an upgrade path in game logic and they share section structure. Unique is as structurally different from Normal as Gem is. Currency and Divination Card have completely distinct layouts. The grammar should reflect this — one path per distinct layout, not one path for everything.
