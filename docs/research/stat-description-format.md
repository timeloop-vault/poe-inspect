# PoE Stat Description File Format

**Date**: 2026-03-07
**Status**: Research complete
**Purpose**: Understand the raw format of PoE's stat description files (`Metadata/StatDescriptions/*.txt`) to evaluate whether poe-inspect-2 should parse them directly instead of using RePoE's `stat_translations.json`.

**Methodology**: Analysis of an extracted `stat_descriptions.txt` file (patch 3.8.0.2), plus analysis of a prior ~260 line JavaScript parser (`stat_generator.js`) for the same file, combined with knowledge of PyPoE's `stat_filters.py` parser and the existing `stat_translations.json` from RePoE.

---

## 1. What the Raw Format Looks Like

### 1a. Encoding

The files are **UTF-16LE** encoded text files (not binary, not `.dat`). They live inside the GGPK/bundle archive at paths like:

- `Metadata/StatDescriptions/stat_descriptions.txt` (the main one)
- `Metadata/StatDescriptions/passive_skill_stat_descriptions.txt`
- `Metadata/StatDescriptions/active_skill_gem_stat_descriptions.txt`
- `Metadata/StatDescriptions/aura_skill_stat_descriptions.txt`
- `Metadata/StatDescriptions/monster_stat_descriptions.txt`
- And many more domain-specific files

The main `stat_descriptions.txt` is the one relevant to item modifiers. It is the largest and contains the translations used for displaying mod text on items.

### 1b. Overall File Structure

A stat description file contains three types of top-level constructs:

1. **`no_description <stat_id>`** -- declares that a stat has no display text (it is hidden/internal)
2. **`description`** -- a translation entry block (the main construct)
3. **`include "<filename>"`** -- includes another stat description file (PyPoE supports this; the main `stat_descriptions.txt` may include others)

The file is whitespace-sensitive: indentation with tabs separates hierarchical levels.

### 1c. `no_description` Entries

```
no_description level
no_description running
no_description item_drop_slots
no_description main_hand_weapon_type
no_description monster_slain_experience_+%
no_description monster_dropped_item_rarity_+%
...
```

These declare stat IDs that should never be displayed to the player. They are simple one-liners.

### 1d. `description` Blocks -- The Core Format

Each `description` block maps one or more stat IDs to display text templates with conditions and format handlers. The structure is:

```
description
	<stat_count> <stat_id_1> [<stat_id_2> ...]
	<translation_count>
		<condition_1> [<condition_2> ...] "<display_text>" [<handler> <index> ...] [reminderstring <id>]
		<condition_1> [<condition_2> ...] "<display_text>" [<handler> <index> ...] [reminderstring <id>]
		...
	lang "<language_name>"
	<translation_count>
		<condition_1> [<condition_2> ...] "<display_text>" [<handler> <index> ...] [reminderstring <id>]
		...
	lang "<language_name>"
	...
```

**Detailed breakdown:**

**Line 1 (after `description`):** The stat ID declaration line.
- First token: integer count of stat IDs
- Remaining tokens: the stat ID strings

```
	1 local_attack_speed_+%
```
means: this entry maps 1 stat (`local_attack_speed_+%`).

```
	2 local_minimum_added_fire_damage local_maximum_added_fire_damage
```
means: this entry maps 2 stats jointly (a multi-stat translation).

**Line 2:** The number of translation variants for the default (English) language.

**Lines 3+:** Translation variant lines. Each has:
1. One condition per stat (space-separated)
2. A quoted display string
3. Optional handler directives after the closing quote

---

## 2. Format Components in Detail

### 2a. Condition Syntax

Each stat referenced in the entry has one condition token per translation line. Conditions determine which translation variant applies based on the stat's value at runtime.

| Condition | Meaning |
|-----------|---------|
| `#` | Any value (wildcard -- no restriction) |
| `1` | Exactly 1 |
| `0` | Exactly 0 |
| `1\|#` | Range: 1 to any (i.e., >= 1, positive values) |
| `#\|-1` | Range: any to -1 (i.e., <= -1, negative values) |
| `2\|#` | Range: 2 to any (i.e., >= 2) |
| `-99\|-1` | Range: -99 to -1 |
| `#\|-100` | Range: any to -100 |
| `100` | Exactly 100 |

The pipe `|` separates `min|max`. `#` means "unbounded" (null). A bare number means both min and max are that value (exact match).

For multi-stat entries, conditions are listed in order matching the stat IDs. For example, with stats `local_physical_damage_+% local_weapon_no_physical_damage`:

```
	# 1|#    "No Physical Damage"
	# #|-1   "No Physical Damage"
	#|-100 # "No Physical Damage"
	1|# 0    "%1%%% increased Physical Damage"
	-99|-1 0 "%1%%% reduced Physical Damage" negate 1
```

The first condition applies to stat 1, the second to stat 2. The engine evaluates variants top-to-bottom and uses the first match.

### 2b. Display Text Syntax

Display text is enclosed in double quotes. Key formatting elements:

| Pattern | Meaning |
|---------|---------|
| `%1%` | Placeholder for stat 1's value |
| `%2%` | Placeholder for stat 2's value |
| `%1$+d` | Placeholder for stat 1's value with forced sign (printf-style: shows `+45` or `-12`) |
| `%%` | Literal percent sign (escaped) |
| `%1%%%` | Stat 1's value followed by a literal `%` sign (e.g., `45%`) |
| `\n` | Literal newline in the display text |

**RePoE's transformation:** RePoE converts these to a different placeholder format:
- `%1%` and `%1$+d` become `{0}` (0-indexed)
- `%2%` becomes `{1}`
- `%%` becomes a literal `%`
- The `$+d` suffix is captured as a format hint (`+#` vs `#`)

### 2c. Index Handlers (Value Transformations)

After the closing quote, optional handler directives specify transformations applied to stat values before display. Format: `<handler_name> <stat_index>` (1-based index).

| Handler | Effect | Example |
|---------|--------|---------|
| `negate` | Multiply value by -1 for display | Converts internal `-5` to displayed `5` for "reduced" text |
| `per_minute_to_per_second` | Divide value by 60 | Internal `600` (per minute) displays as `10` (per second) |
| `milliseconds_to_seconds` | Divide value by 1000 | Internal `4000` displays as `4` |
| `deciseconds_to_seconds` | Divide value by 10 | Internal `30` displays as `3` |
| `divide_by_one_hundred` | Divide value by 100 | Internal `150` displays as `1.5` |
| `old_leech_percent` | Divide value by 5 (legacy) | Internal `25` displays as `5` |
| `old_leech_permyriad` | Divide value by 50 | Internal `250` displays as `5` |
| `times_twenty` | Multiply value by 20 | Internal `5` displays as `100` |
| `canonical_line` | Marks the line as the "canonical" representation | Used for deduplication/precedence |
| `canonical_stat` | Marks a specific stat as canonical | Similar to canonical_line |
| `reminderstring` | Followed by reminder text ID | Not a value handler but a metadata annotation |

Multiple handlers can appear on the same line. Each applies to a specific stat index (1-based). Example:

```
# "Socketed Golem Skills have Minions Regenerate %1%%% of Life per second" per_minute_to_per_second 1
```

This means: apply `per_minute_to_per_second` to stat index 1.

```
"Commissioned %2% coins to commemorate Cadiro" times_twenty 2
```

This means: apply `times_twenty` to stat index 2 (the seed value).

### 2d. Reminder Strings

Lines can end with `reminderstring <ReminderTextId>`. This references a separate table of tooltip text that provides explanatory notes (displayed in parentheses in-game). Example:

```
# "Gain Onslaught for %1% seconds" milliseconds_to_seconds 1 reminderstring ReminderTextOnslaught
```

The reminder text is something like `(Onslaught grants 20% increased Attack, Cast, and Movement Speed)`.

### 2e. Language Blocks

After the default (English) translation lines, additional language blocks follow:

```
	lang "Russian"
	<translation_count>
		...
	lang "German"
	<translation_count>
		...
```

Languages observed in the data: English (default, no `lang` prefix), Russian, German, French, Portuguese, Korean, Thai, Traditional Chinese, Simplified Chinese, Spanish, Japanese.

Each language block repeats the same condition structure but with localized display text.

---

## 3. Concrete Examples from the Raw File

### 3a. Simple Single-Stat, No Conditions

```
description
	1 item_generation_cannot_change_prefixes
	1
		# "Prefixes Cannot Be Changed"
	lang "Portuguese"
	1
		# "Prefixos Nao Podem Ser Modificados"
	...
```

- 1 stat: `item_generation_cannot_change_prefixes`
- 1 English variant: any value matches (`#`), shows fixed text
- No handlers, no placeholders

### 3b. Increased/Reduced Pattern (negate handler)

```
description
	1 local_attack_speed_+%
	2
		1|# "%1%%% increased Attack Speed"
		#|-1 "%1%%% reduced Attack Speed" negate 1
	lang "Portuguese"
	2
		1|# "Velocidade de Ataque aumentada em %1%%%"
		#|-1 "Velocidade de Ataque reduzida em %1%%%" negate 1
	...
```

- 1 stat: `local_attack_speed_+%`
- 2 English variants:
  - When value >= 1: show `"X% increased Attack Speed"`
  - When value <= -1: show `"X% reduced Attack Speed"` with `negate 1` (flip sign so `-5` displays as `5`)
- This is the most common pattern for percentage-based stats

### 3c. Signed Value (printf-style `$+d`)

```
description
	1 base_maximum_life
	1
		# "%1$+d to maximum Life"
	lang "Portuguese"
	1
		# "%1$+d de Vida maxima"
	...
```

- 1 stat: `base_maximum_life`
- 1 English variant: any value, displays with forced sign (`+45 to maximum Life` or `-10 to maximum Life`)
- The `$+d` format specifier means "show + or - sign always"

### 3d. Multi-Stat Translation (Adds X to Y Damage)

```
description
	2 global_minimum_added_physical_damage global_maximum_added_physical_damage
	1
		# # "Adds %1% to %2% Physical Damage"
	lang "Portuguese"
	1
		# # "Adiciona %1% a %2% de Dano Fisico"
	...
```

- 2 stats: `global_minimum_added_physical_damage` and `global_maximum_added_physical_damage`
- 1 variant: both stats can be any value, displayed as `"Adds 10 to 20 Physical Damage"`
- Each `#` condition corresponds to a stat in order

### 3e. Complex Multi-Stat with Multiple Conditions

```
description
	2 local_physical_damage_+% local_weapon_no_physical_damage
	5
		# 1|# "No Physical Damage"
		# #|-1 "No Physical Damage"
		#|-100 # "No Physical Damage"
		1|# 0 "%1%%% increased Physical Damage" canonical_line
		-99|-1 0 "%1%%% reduced Physical Damage" negate 1
```

- 2 stats jointly control which text to show
- 5 variants with complex condition logic:
  - If stat 2 (no_physical_damage) is >= 1: "No Physical Damage"
  - If stat 2 is <= -1: "No Physical Damage"
  - If stat 1 is <= -100: "No Physical Damage"
  - If stat 1 >= 1 AND stat 2 == 0: "X% increased Physical Damage" (canonical)
  - If stat 1 is -99 to -1 AND stat 2 == 0: "X% reduced Physical Damage" (negated)

### 3f. Handler with Per-Minute to Per-Second Conversion

```
description
	1 local_display_socketed_golem_life_regeneration_rate_per_minute_%
	1
		# "Socketed Golem Skills have Minions Regenerate %1%%% of Life per second" per_minute_to_per_second 1
```

- The internal stat stores regeneration per minute
- The handler divides by 60 so the display shows per second
- `per_minute_to_per_second 1` means "apply this transformation to stat index 1"

### 3g. Milliseconds to Seconds Conversion

```
description
	1 local_display_socketed_golem_skill_grants_onslaught_when_summoned
	1
		# "Gain Onslaught for %1% seconds when you Cast Socketed Golem Skill" milliseconds_to_seconds 1 reminderstring ReminderTextOnslaught
```

- Internal value is in milliseconds (e.g., 4000)
- Handler converts to seconds for display (e.g., 4)
- Also includes a reminder string reference

### 3h. Multi-Stat with Ignored Stats and Multiple Conditions

```
description
	2 local_unique_hungry_loop_number_of_gems_to_consume local_unique_hungry_loop_has_consumed_gem
	5
		1 0 "Consumes Socketed Support Gems when they reach Maximum Level\nCan Consume %1% Support Gem\nHas not Consumed any Gems"
		1 # "Consumes Socketed Support Gems when they reach Maximum Level\nCan Consume %1% additional Support Gem"
		2|# 0 "Consumes Socketed Support Gems when they reach Maximum Level\nCan Consume %1% Support Gems\nHas not Consumed any Gems"
		2|# # "Consumes Socketed Support Gems when they reach Maximum Level\nCan Consume %1% additional Support Gems"
		-1 1 "Has Consumed 1 Gem"
```

- 2 stats control the text
- Stat 1 determines the number of gems to consume
- Stat 2 determines whether a gem has already been consumed
- Different pluralization and text based on combinations
- Multi-line display text using `\n`

---

## 4. Parsing Complexity Assessment

### 4a. File Scale

Based on the 3.8.0.2 extracted file:
- The file contains approximately 5,700+ `description` blocks (for all languages combined within each block)
- Approximately 34 `no_description` entries at the top
- The file is very large (tens of thousands of lines when accounting for all language variants)
- Modern patches (3.25+) likely have more entries as new stats have been added

### 4b. Parser Complexity

**Overall: Moderate.** The format is a line-oriented, indent-sensitive text format with clear structure. A parser would need to handle:

1. **UTF-16LE decoding** -- the file must be decoded before parsing
2. **Line-by-line state machine** -- track whether we're in a description block, language block, or at the top level
3. **Tab-based indentation** -- tabs separate hierarchy levels
4. **Condition parsing** -- split on `|` and handle `#` (wildcard) vs integer literals
5. **Quoted string extraction** -- extract the display text between double quotes
6. **Handler parsing** -- after the closing quote, parse `<name> <index>` pairs
7. **Multi-stat entries** -- stat count determines how many conditions per line
8. **Language blocks** -- `lang "Name"` switches the current language context
9. **`no_description` entries** -- simple to handle
10. **`include` directives** -- file inclusion (may or may not be needed depending on which description file we process)

**Edge cases observed:**
- Inconsistent whitespace (some lines use spaces, some use tabs, some mix)
- The `reminderstring` keyword appears in the handler position but is not a value transformation
- Multi-line strings using `\n` (literal backslash-n in the text, not actual newlines)
- The `canonical_line` and `canonical_stat` markers which affect precedence
- `%%` escaping for literal percent signs
- The `$+d` printf-style format specifier
- Some lines have extra trailing whitespace

### 4c. Estimated Code Size (Rust)

A Rust parser for this format would be approximately:
- **300-500 lines** for the core parser (tokenizer + state machine)
- **100-200 lines** for data types (translation entry, condition, handler enums)
- **50-100 lines** for value transformation functions (negate, divide, etc.)
- **Total: ~500-800 lines** of well-structured Rust code

This is quite manageable. The format is simpler than many configuration file formats.

### 4d. Format Stability

The stat description format has been stable since at least 2013 (when PyPoE was first written). Key observations:

- **GGG has NOT changed the fundamental syntax** across major patches. The format from 3.8 (2019) is the same as described by PyPoE's parser written around 2015.
- **New entries are added** with each patch (new stats get new description blocks), but the format itself remains unchanged.
- **New handlers** have been added over the years (e.g., `canonical_stat` was added later), but the handler syntax `<name> <index>` has been consistent.
- **Language additions** (e.g., Korean, Thai, Simplified Chinese) were added in later patches but use the same `lang` block syntax.
- **The UTF-16LE encoding** has been consistent.

**Risk of format change: Low.** GGG has no incentive to change this internal format. It is processed by their game engine and would require rewriting their own tools to change it. The format has survived 10+ years of patches unchanged.

---

## 5. Existing Parsers

### 5a. PyPoE (Python) -- The Reference Implementation

**Repository:** `github.com/OmegaK2/PyPoE`
**File:** `PyPoE/poe/file/stat_filters.py`

PyPoE's `StatFilterFile` class is the canonical parser. Key implementation details:
- Reads UTF-16 encoded text files
- Line-by-line parsing with state tracking
- Handles `no_description`, `description`, `include`, and `lang` blocks
- Parses conditions, handlers, and reminder strings
- Outputs structured Python objects that RePoE then serializes to JSON

PyPoE is unmaintained (last active ~2020) but its parser logic remains the definitive reference for the format.

### 5b. Prior JavaScript Parser -- Local Prior Art

A ~260 line JavaScript parser was previously written for this exact file. Key characteristics:
- ~260 lines of JavaScript
- Reads the UTF-16LE file and splits by newline
- State machine: tracks `newStat`, `skip` (for non-English languages)
- Only parses English translations (skips `lang` blocks)
- Parses conditions using `split('|')` for min/max ranges
- Extracts handlers via `arg_extra()` function
- Converts `%N%` and `%N$+d` placeholders to `#` for template matching
- Outputs a JSON lookup table mapping template strings to stat IDs
- Cross-references with RePoE's `mods.json` for enrichment

This parser works but is rough -- it was a prototype. However, it proves the format is parseable in ~260 lines of JavaScript without difficulty.

### 5c. Path of Building (Lua)

Path of Building parses stat descriptions in Lua as part of its data pipeline. PoB's parser handles the same format but stores results in Lua table structures. The parser is part of PoB's data export tooling, not its runtime code.

### 5d. Rust Implementations

**No known Rust parser exists** for the stat description format specifically. However:
- `poe-bundle` (Rust) can extract the raw `.txt` files from the game archive
- `poe-query` (Rust) parses `.dat` binary files but does NOT handle stat description text files
- A Rust parser would be a new contribution to the ecosystem

### 5e. TypeScript/JavaScript Implementations

Besides the local `stat_generator.js` prototype:
- No widely-distributed TypeScript/JavaScript parser for the raw format exists
- Most JS/TS tools (Awakened PoE Trade, etc.) consume RePoE's pre-parsed `stat_translations.json` rather than parsing the raw files

---

## 6. How RePoE Transforms the Raw Format

RePoE's pipeline for `stat_translations.json`:

```
Metadata/StatDescriptions/stat_descriptions.txt  (UTF-16LE text)
    |
    v
PyPoE StatFilterFile parser  (Python)
    |
    v
Structured Python objects (StatDescription, TranslationEntry, etc.)
    |
    v
RePoE export script  (Python)
    |
    v
stat_translations.json  (UTF-8 JSON, ~11MB)
```

### 6a. What RePoE Changes

| Raw Format | RePoE JSON | Change |
|------------|-----------|--------|
| `%1%`, `%1$+d` | `{0}` | 1-indexed to 0-indexed, printf syntax removed |
| `%2%` | `{1}` | Same |
| `%%` | `%` | Unescaped |
| Condition `1\|#` | `{"min": 1, "max": null}` | Pipe syntax to JSON object |
| Condition `#` | `{"min": null, "max": null}` | Wildcard to null/null |
| Condition `#\|-1` | `{"min": null, "max": -1}` | Same |
| `negate 1` | `"index_handlers": [["negate"]]` | Name preserved, index maps to array position |
| `per_minute_to_per_second 1` | `"index_handlers": [["per_minute_to_per_second"]]` | Same |
| `$+d` format | `"format": ["+#"]` | Captured as format hint |
| `%N%` format | `"format": ["#"]` | Default format |
| `reminderstring X` | `"reminder_text": "(text...)"` | Reference resolved to actual text |
| `lang "Russian"` | `"Russian": [...]` | Language name becomes JSON key |
| `canonical_line` | Not explicitly preserved | May be dropped or handled implicitly |

### 6b. What RePoE Preserves Faithfully

- All stat IDs (verbatim)
- All condition ranges
- All display text strings (with placeholder format conversion)
- All index handlers
- All language translations
- Reminder text (resolved to actual strings)
- The multi-stat grouping (multiple IDs per entry)
- The variant ordering (conditions evaluated top-to-bottom)

### 6c. What RePoE Adds

- `trade_stats`: Cross-references to the official trade API stat IDs (not in the raw file)
- `hidden`: Flag for hidden/internal stats
- Resolved reminder text (the raw file only has reference IDs like `ReminderTextOnslaught`)

### 6d. What is Lost

- `canonical_line` / `canonical_stat` markers (may not be preserved in JSON)
- Original ordering within the file (JSON doesn't guarantee array order, though RePoE preserves it in practice)
- The raw file structure (includes, no_description entries)
- Exact whitespace/formatting

---

## 7. Parse-It-Ourselves vs Use RePoE

### 7a. Arguments for Parsing Raw Files Directly

1. **Patch-day independence**: When a new patch drops, the raw files are immediately available in the game installation. RePoE may take hours or days to update.

2. **Full control**: We own the pipeline end-to-end. No dependency on a community maintainer.

3. **PoE2 support**: RePoE's PoE2 coverage for stat translations is incomplete. The raw files from the PoE2 installation use the same format.

4. **Simpler data flow**: Read from game files directly instead of fetching JSON from GitHub. No network dependency at runtime.

5. **Format is stable**: The raw format has been unchanged for 10+ years. The risk of it breaking is near zero.

6. **Parser is small**: Estimated 500-800 lines of Rust -- comparable to the amount of code we'd write to deserialize and process `stat_translations.json` anyway.

7. **Prior art exists locally**: The `stat_generator.js` proves the format is parseable in ~260 lines.

### 7b. Arguments for Using RePoE's stat_translations.json

1. **Already proven**: poe-inspect v1 used `stat_translations.json` successfully against thousands of real items.

2. **No GGPK/bundle dependency**: `stat_translations.json` is a static JSON file downloadable from GitHub. No need for `poe-bundle`, `ooz` (C++ FFI), or access to the game installation.

3. **Trade stats included**: RePoE adds `trade_stats` cross-references that the raw file doesn't have. These are valuable for trade API integration.

4. **Reminder text resolved**: RePoE resolves `reminderstring ReminderTextOnslaught` to the actual text `(Onslaught grants 20% increased Attack, Cast, and Movement Speed)`. Doing this ourselves would require additionally parsing `ClientStrings.dat` or similar.

5. **Community validated**: The JSON output has been used by dozens of tools. Edge cases have been found and fixed over years.

6. **No encoding hassle**: JSON is UTF-8. The raw file is UTF-16LE, requiring an extra decoding step.

7. **All languages pre-parsed**: The JSON has all languages in a structured format. No need to implement the `lang` block parser.

### 7c. What Raw Gives Us That JSON Doesn't

- **`canonical_line` / `canonical_stat` markers**: These may affect which translation variant takes precedence when multiple entries could match. RePoE may not preserve these.
- **File inclusion structure**: Understanding which stat descriptions are in which file (items vs passives vs gems).
- **`no_description` entries**: Knowing which stats are explicitly hidden.
- **Immediate patch-day access**: Bypasses the RePoE update cycle.

### 7d. What JSON Gives Us That Raw Doesn't

- **Trade API stat IDs**: The `trade_stats` field maps internal stat IDs to the trade site's identifiers.
- **Resolved reminder text**: Actual tooltip text instead of reference IDs.
- **A flat, structured format**: No parsing needed, just `serde_json::from_str`.
- **Extensive community testing**: Thousands of items have validated the translations.

### 7e. Risk Assessment

| Risk | Raw Parser | RePoE JSON |
|------|-----------|------------|
| Format change breaks parser | Very Low (unchanged 10+ years) | N/A (JSON is standard) |
| Missing data after patch | None (read from game files) | Medium (depends on maintainer update speed) |
| Build complexity | Medium (UTF-16 decoding, GGPK access via poe-bundle + ooz FFI) | None |
| Correctness bugs | Medium (must handle all edge cases ourselves) | Low (community-validated) |
| Maintenance burden | Low (format is stable) | Very Low (just update URL) |
| PoE2 support | Good (same format) | Uncertain (RePoE PoE2 coverage is thin) |

---

## 8. Recommendation

### For poe-inspect-2 MVP: Use RePoE's stat_translations.json

The argument is clear-cut for the MVP:
- We already know it works (v1 proved it)
- No build complexity
- Trade stats included
- Community-validated

### For Future Consideration: Build a Raw Parser as a Supplementary Path

The raw format is simple enough that building a Rust parser is realistic and valuable as a second-priority task. The key triggers that would make it worth doing:

1. **RePoE update lag on patch day** becomes a problem for our users
2. **PoE2 stat translations** are needed and RePoE doesn't cover them
3. **We want to eliminate the network dependency** entirely (read from local game files)
4. **We want `canonical_line` / `canonical_stat` data** that RePoE may not preserve

### If We Build a Raw Parser, the Approach Would Be

1. **Use `poe-bundle`** to extract `Metadata/StatDescriptions/stat_descriptions.txt` from the game installation
2. **Decode UTF-16LE** to a Rust `String`
3. **Line-by-line state machine parser** (~500-800 lines of Rust):
   - Parse `no_description` lines
   - Parse `description` blocks (stat IDs, conditions, display text, handlers)
   - Optionally parse `lang` blocks (or skip non-English if we only need English)
   - Handle `include` directives
4. **Output the same data structure** we'd get from `stat_translations.json` -- making the downstream code agnostic to the data source
5. **Total effort**: ~2-3 days of focused work, including tests

### The Key Insight

The stat description format is NOT the bottleneck or the hard part. The hard part of the data pipeline is:
- Correctly extracting templates from item clipboard text
- Handling the `negate` / `divide_by_one_hundred` / etc. transformations correctly
- Mapping stat IDs to mod entries in `mods.json`
- Calculating tiers with base-tag-aware filtering

All of this complexity exists regardless of whether the stat translations come from a raw `.txt` file or from `stat_translations.json`. The data source is interchangeable; the processing logic is the same.

---

## 9. Format Grammar (Pseudo-BNF)

For reference, here is a pseudo-grammar of the stat description format:

```
file = (no_description | description | include)*

no_description = "no_description" WS stat_id NL

include = "include" WS QUOTE filename QUOTE NL

description = "description" NL
              TAB stat_header NL
              translation_block
              (lang_block)*

stat_header = stat_count WS stat_id (WS stat_id)*

translation_block = TAB translation_count NL
                    (TAB TAB translation_line NL)+

lang_block = TAB "lang" WS QUOTE language_name QUOTE NL
             TAB translation_count NL
             (TAB TAB translation_line NL)+

translation_line = condition (WS condition)* WS QUOTE display_text QUOTE (WS handler)*

condition = "#"                     -- wildcard (any value)
          | integer                 -- exact match
          | integer "|" bound       -- range (min|max)
          | bound "|" integer       -- range (min|max)
          | bound "|" bound         -- range (min|max)

bound = "#" | integer

handler = handler_name WS stat_index
        | "reminderstring" WS reminder_id
        | "canonical_line"

handler_name = "negate" | "per_minute_to_per_second" | "milliseconds_to_seconds"
             | "deciseconds_to_seconds" | "divide_by_one_hundred"
             | "old_leech_percent" | "old_leech_permyriad"
             | "times_twenty" | "canonical_stat"
             | (other handler names)

stat_index = integer  -- 1-based index into the stat_id list

display_text = (text with %N%, %N$+d placeholders and %% escaping)*

stat_id = [a-z0-9_+%]+
stat_count = integer
translation_count = integer
```

---

## 10. Key References

### Files Analyzed
- Raw stat description file: extracted `stat_descriptions.txt` (patch 3.8.0.2)
- Prior JavaScript parser: `stat_generator.js` (~260 lines)
- RePoE output: `stat_translations.json`
- poe-inspect v1: data processor (`transformer.rs`) and template lookup (`template_lookup.rs`)

### Repositories
- **PyPoE** (reference parser): `github.com/OmegaK2/PyPoE` -- `PyPoE/poe/file/stat_filters.py`
- **RePoE** (JSON exporter): `github.com/brather1ng/RePoE`
- **repoe-fork** (maintained fork): `github.com/repoe-fork/repoe-fork.github.io`
- **poe-bundle** (Rust GGPK reader): `github.com/ex-nihil/poe-bundle`
- **poe-query** (Rust .dat query tool): `github.com/ex-nihil/poe-query`

### Related Research
- `docs/research/ggpk-extraction-ecosystem.md` -- covers the overall data extraction pipeline
- `docs/research/ggpk-tools-poe-bundle.md` -- detailed analysis of poe-bundle and poe-query
- `docs/research/item-format-and-data.md` -- how stat translations are used in the item parsing pipeline
