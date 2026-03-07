# PoE Ctrl+Alt+C Item Format: Deep Analysis & Parsing Strategy

This document is based on analysis of 45+ real item fixtures from PoE1 and PoE2, plus a complete review of the v1 parser codebase (`poe-inspect/packages/poe-parser/`).

---

## A. Format Catalog

The advanced item text format (Ctrl+Alt+C) uses `--------` separator lines to divide the item into **sections**. Sections are positional but many are optional. The format differs subtly between PoE1 and PoE2.

### A.1 Header Section (always present)

The header is always the first section, before the first separator.

**Patterns observed:**

```
Item Class: <class>
Rarity: <Normal|Magic|Rare|Unique>
<Name>              -- absent for Normal rarity
<Base Type>         -- present for all rarities
```

| Rarity | Lines after Rarity | Example |
|--------|-------------------|---------|
| Normal | 1 line (base type only) | `Imperial Staff` |
| Magic  | 1 line (name = full affixed name, NO separate base type line) | `Flagellant's Sapphire Flask of Craft` / `Despot Axe of Infamy` / `Compound Bow of Crafting` |
| Rare   | 2 lines (name, then base type) | `Loath Spur` / `Murder Boots` |
| Unique | 2 lines (name, then base type) | `Soul Strike` / `Spike-Point Arrow Quiver` |

**Critical observation for Magic items:** The fixture data shows Magic items have only ONE line after rarity, which is the full affixed name (e.g., `Despot Axe of Infamy`). There is no separate base type line. The base type must be extracted from within that name string -- which requires knowing the base type list to find where the prefix ends and the base begins.

**Item Class values observed:**
- Equipment: `Boots`, `Gloves`, `Helmets`, `Body Armours`, `Belts`, `Rings`, `Amulets`
- Weapons: `Two Hand Axes`, `Thrusting One Hand Swords`, `Wands`, `Staves`, `Bows`
- Ranged: `Quivers`
- Maps: `Maps`
- Flasks: `Utility Flasks`
- Jewels: `Jewels`, `Abyss Jewels`

### A.2 Properties Section (optional, follows header separator)

This section contains item stats. Its content depends entirely on the item class.

**Weapon sub-header line:**
Weapons have an extra line between the header and properties that states the weapon category:
```
Two Handed Axe     -- for "Two Hand Axes" class
One Handed Sword   -- for "Thrusting One Hand Swords" class
Wand               -- for "Wands" class
Staff              -- for "Staves" class
Bow                -- for "Bows" class
```
This line is NOT a property -- it's the `item_base` (weapon archetype). It appears as the first line of what looks like the properties section but is structurally distinct.

**Armour properties:**
```
Quality: +20% (augmented)
Armour: 1167 (augmented)
Evasion Rating: 293 (augmented)
Energy Shield: 295 (augmented)
```

**Weapon properties:**
```
Quality: +27% (augmented)
Physical Damage: 114-155 (augmented)
Elemental Damage: 3-6 (augmented), 7-108 (augmented)
Critical Strike Chance: 6.15% (augmented)
Attacks per Second: 1.77 (augmented)
Weapon Range: 13
Weapon Range: 1.4 metres          -- PoE1 newer format with unit
Weapon Range: 1.3 metres          -- PoE1 newer format with unit
```

**Flask properties:**
```
Quality: +20% (augmented)
Lasts 9,60 (augmented) Seconds
Consumes 20 of 50 Charges on use
Currently has 50 Charges
+5% to maximum Cold Resistance
(Maximum Resistances cannot be raised above 90%)
+40% to Cold Resistance
```
Flask properties are special: they intermix numeric stats with what look like modifier lines (e.g., `+5% to maximum Cold Resistance`). These are base type intrinsic properties, not rollable modifiers.

**Jewel properties:**
```
Limited to: 1 Historic
Radius: Large
```
Jewels have unique property formats not seen elsewhere.

**Abyss Jewel sub-header:**
```
Abyss
```
Similar to weapon `item_base`, Abyss Jewels have a one-word sub-header `Abyss` before any other properties.

**Map properties:**
```
Map Tier: 16
Atlas Region: Valdo's Rest
Item Quantity: +72% (augmented)
Item Rarity: +37% (augmented)
Monster Pack Size: +24% (augmented)
Quality: +8% (augmented)
Reward: Foil Lifesprig
More Currency: +70% (augmented)
More Maps: +50% (augmented)
More Scarabs: +103% (augmented)
```

**Quality variant (PoE1 new / PoE2):**
```
Quality (Defence Modifiers): +20% (augmented)
```
Quality can have a qualifier in parentheses before the value.

**Memory Strands (PoE1 specific to certain items):**
```
Memory Strands: 2
Memory Strands: 38
```

**`(augmented)` marker:** Present when the displayed value has been modified by quality, modifiers, or other effects. Absent when the value is the base/unmodified value. The `(unmet)` marker can appear on requirements (e.g., `Dex: 134 (unmet)`).

**Property section absent:** Some items (rings, amulets, belts, quivers, jewels) may have no defense/damage properties, going directly from header to requirements.

### A.3 Requirements Section (optional)

**PoE1 format (multi-line):**
```
Requirements:
Level: 70
Str: 155
Dex: 98
Int: 155
```
Each requirement on its own line. Requirements can include `(augmented)` or `(unmet)`.

**PoE2 format (single-line):**
```
Requires: Level 65, 60 (augmented) Str, 60 (augmented) Dex
```
All on one line, comma-separated, with `(augmented)` inline per stat.

**Absent:** Maps, currency, and some jewels have no requirements section.

### A.4 Sockets Section (optional)

**PoE1:**
```
Sockets: R-G-R-B       -- linked colors
Sockets: R R-R-R-R     -- mix of linked and unlinked
Sockets: G B            -- unlinked
Sockets: A              -- Abyssal socket (Stygian Vise)
```
Colors: R (red/str), G (green/dex), B (blue/int), W (white), A (abyssal). `-` means linked, space means unlinked.

**PoE2:**
```
Sockets: S S            -- generic sockets (no colors)
```
PoE2 uses `S` for generic sockets (skill gem sockets).

### A.5 Item Level Section (always present for equippable items)

```
Item Level: 85
```
Always a single line in its own separator-delimited section.

### A.6 Monster Level Section (maps only)

```
Monster Level: 83
```
Appears in its own section AFTER Item Level for maps. Not present on Normal maps with no mods.

### A.7 Talisman Tier Section (talismans only)

```
Talisman Tier: 3
```
Appears in its own separator-delimited section between Item Level and implicits/enchants.

### A.8 Enchant Section (optional, between talisman/item-level and implicits)

Enchants appear WITHOUT `{ }` modifier headers in the advanced format. They are identified by the `(enchant)` suffix.

**Lab enchants (PoE1):**
```
8% increased Movement Speed if you haven't been Hit Recently
(Recently refers to the past 4 seconds) (enchant)
```
Note: the enchant can be multi-line, with the `(enchant)` marker only on the last line. Prior lines look like regular text.

**Body armour enchants (PoE1):**
```
Quality does not increase Defences
(Defences are Armour, Evasion Rating and Energy Shield) (enchant)
Grants +1 to Maximum Life per 2% Quality (enchant)
```
Multiple enchants can appear, each ending with `(enchant)`.

**Passive allocation enchants:**
```
Allocates Entropy (enchant)
```

**Delirium enchants (maps):**
```
Delirium Reward Type: Armour (enchant)
Players in Area are 20% Delirious (enchant)
```

**Crucible/harvest/other enchants (PoE1):**
```
10% increased Explicit Life Modifier magnitudes (enchant)
All Sockets are Green (enchant)
```

**Flask enchants:**
```
Used when Charges reach full (enchant)
```

**Rune mods (PoE2 only):**
```
+14% to Fire Resistance (rune)
+10% to Lightning Resistance (rune)
```
These appear in the same position as enchants but use `(rune)` suffix.

### A.9 Implicit Modifier Section (optional)

In the advanced format, implicits are wrapped in `{ }` modifier headers:

```
{ Implicit Modifier — Aura }
Discipline has 19(15-20)% increased Aura Effect (implicit)
```

```
{ Implicit Modifier }
15(6-15)% increased Rarity of Items found (implicit)
```

Note: the `(implicit)` suffix appears on modifier VALUE lines, not on headers. The header `{ Implicit Modifier }` may or may not have tags after `—`.

**Influence implicits (Eldritch):**
```
{ Searing Exarch Implicit Modifier (Greater) }
19(18-20)% chance to Avoid being Stunned (implicit)
{ Eater of Worlds Implicit Modifier (Greater) — Life }
While a Unique Enemy is in your Presence, Regenerate 0.3% of Life per second per Endurance Charge (implicit)
```

Influence implicit headers include the influence source and a tier in parentheses: `(Lesser)`, `(Greater)`, `(Grand)`, `(Exceptional)`.

**Synthesised implicits:**
```
{ Implicit Modifier — Aura }
Discipline has 19(15-20)% increased Aura Effect (implicit)
```
Synthesised implicits use the same header format as regular implicits.

**Map implicits:**
```
{ Implicit Modifier }
Area is Influenced by The Shaper — Unscalable Value (implicit)
```

```
{ Implicit Modifier }
Area is influenced by the Originator's Memories — Unscalable Value (implicit)
```

### A.10 Explicit Modifier Section (the main mods)

**Prefix/Suffix with Tier:**
```
{ Prefix Modifier "Blue" (Tier: 2) — Mana }
+68(65-68) to maximum Mana
{ Suffix Modifier "of the Lynx" (Tier: 8) — Attribute }
+14(13-17) to Dexterity
```

**Prefix/Suffix without Tier (influenced/essence mods):**
```
{ Suffix Modifier "of the Essence" — Attack, Speed }
18(17-18)% increased Attack Speed
```
Some mods (essence, specific influenced mods) have no `(Tier: N)` in the header.

**Crafted modifiers:**
```
{ Master Crafted Suffix Modifier "of Craft" (Rank: 2) — Elemental, Lightning, Resistance }
+22(21-28)% to Lightning Resistance (crafted)
```
Note: crafted mods use `(Rank: N)` instead of `(Tier: N)`. The value line has `(crafted)` suffix.

**Master Crafted without Rank:**
```
{ Master Crafted Suffix Modifier "of Crafting" }
Can have up to 3 Crafted Modifiers — Unscalable Value (crafted)
```

**Master Crafted Prefix:**
```
{ Master Crafted Prefix Modifier "Upgraded" — Speed }
20(18-20)% increased Movement Speed (crafted)
10(8-12)% chance to gain Onslaught for 4 seconds on Kill (crafted)
(Onslaught grants 20% increased Attack, Cast, and Movement Speed) (crafted)
```

**Unique modifiers:**
```
{ Unique Modifier — Defences }
150(80)% faster start of Energy Shield Recharge
{ Unique Modifier }
40% reduced Energy Shield Recharge Rate
```
Unique modifier headers have no name, no tier. Tags are optional. Value lines have NO `(implicit)` or `(crafted)` suffix.

**Fractured modifiers:**
```
{ Prefix Modifier "Gleaming" (Tier: 5) — Damage, Physical, Attack }
Adds 19(14-21) to 34(32-38) Physical Damage (fractured)
```
The `(fractured)` suffix appears on the value line. The header is a normal prefix/suffix header.

**Multi-line (hybrid) modifiers:**
Many mods produce multiple stat lines under a single header:
```
{ Prefix Modifier "Mammoth's" (Tier: 1) — Defences, Armour, Evasion }
42(39-42)% increased Armour and Evasion
17(16-17)% increased Stun and Block Recovery
```

```
{ Prefix Modifier "Prior's" (Tier: 1) — Life, Defences, Energy Shield }
+11(11-15) to maximum Energy Shield
+25(24-28) to maximum Life
```

The parser must collect ALL lines between one `{ }` header and the next header/separator as belonging to the same modifier.

**Explanation lines (parenthetical):**
```
(50% of Damage from Suppressed Hits and Ailments they inflict is prevented)
(Leeched Mana is recovered over time. Multiple Leeches can occur simultaneously, up to a maximum rate)
(Bleeding deals Physical Damage over time, based on the base Physical Damage of the Skill. Damage is higher while moving)
```
These are reminder text. They appear after a mod value line. They can also have an `(implicit)` suffix:
```
(50% of Damage from Suppressed Hits and Ailments they inflict is prevented) (implicit)
(Maximum Resistances cannot be raised above 90%) (implicit)
```

**`-- Unscalable Value` annotation:**
```
Rare Monsters each have a Nemesis Mod — Unscalable Value
Can have up to 3 Crafted Modifiers — Unscalable Value (crafted)
Hits can't be Evaded — Unscalable Value (crafted)
```
The `— Unscalable Value` annotation appears on mod value lines to indicate the value cannot be scaled/modified.

**Value format in advanced mode:**
```
+68(65-68)       -- current value with (min-max) range
19(18-20)%       -- no + prefix, percentage
0.34(0.2-0.4)%   -- decimal values
1(10--10)%       -- negative range (Ventor's Gamble: min=10, max=-10 for "reduced")
-9(-12--9)%      -- negative current AND negative range
```

The `(min-max)` range is the tier roll range. It appears directly after the numeric value with no space.

### A.11 Influence Markers Section (inline, not separated)

Influence markers appear AFTER explicit mods, within the same section (no separator before them):
```
Searing Exarch Item
Eater of Worlds Item
Elder Item
Shaper Item
Crusader Item
Redeemer Item
```
These are NOT in their own separator-delimited section. They appear as plain lines after the last explicit modifier.

### A.12 Flavor Text Section (unique items, maps)

Flavor text appears in its own separator-delimited section:
```
--------
In this chaotic world
The rewards of the Soul
Outlast the rewards of the Flesh.
--------
```

Maps have usage instructions as "flavor text":
```
Travel to this Map by using it in a personal Map Device. Maps can only be used once.
```

T17 reward maps have extended instructions:
```
Travel to this Map by using it in a personal Map Device. Maps can only be used once. Defeat 90% of all monsters in this Map, including all Rare and Unique enemies to obtain the Reward. The area created is not affected by your Atlas Passive Tree, and cannot be augmented via the Map Device.
```

Jewels have placement instructions:
```
Place into an allocated Jewel Socket on the Passive Skill Tree. Right click to remove from the Socket.
```

### A.13 Footer / Special Properties Section

Special properties appear at the end, each potentially in its own separator-delimited section:

```
--------
Corrupted
```

```
--------
Fractured Item
```

```
--------
Synthesised Item
```

```
--------
Split
```

```
--------
Relic Unique
```

```
--------
Unmodifiable
```

```
--------
Modifiable only with Chaos Orbs, Vaal Orbs, Delirium Orbs and Chisels
```

```
--------
Foil (Celestial Amethyst)
```

Multiple special properties can stack:
```
--------
Corrupted
--------
Relic Unique
```

### A.14 Map Conversions Section (T17 maps only)

Appears as a sub-section within properties:
```
Chance for dropped Maps to convert to:
Shaper Map: 9% (augmented)
Elder Map: 12% (augmented)
Conqueror Map: 21% (augmented)
Unique Map: 5% (augmented)
Scarab: 20% (augmented)
```

---

## B. Ambiguity Map

### B.1 Header Name vs Base Type (CRITICAL - affects all Magic items)

**Ambiguity:** For Magic rarity items, the header has only ONE line after Rarity. This line contains the full affixed item name (e.g., `Despot Axe of Infamy`). There is no separate base type line. To extract the base type, you must find which substring of the name matches a known base type.

**Examples from fixtures:**
- `Flagellant's Sapphire Flask of Craft` -- base = `Sapphire Flask`
- `Despot Axe of Infamy` -- base = `Despot Axe`
- `Compound Bow of Crafting` -- base = `Compound Bow`

**Resolution:** Requires lookup in `base_items.json`. Must find the longest matching base type substring within the name.

**Severity:** Always ambiguous for Magic items. 100% of Magic items require this lookup.

### B.2 Header Line Count by Rarity

**Ambiguity:** After `Rarity:`, you don't know if the next line is a name or a base type without knowing the rarity. Normal items have 1 line (base type), Magic items have 1 line (affixed name), Rare/Unique have 2 lines (name + base type).

**Resolution:** The Rarity line is always present and always the second line. Parse it first, then use the rarity to determine how many header lines follow.

**Severity:** Low -- rarity is always present and unambiguous.

### B.3 Properties Section: Item Base vs First Property

**Ambiguity:** Weapons have a `item_base` line (e.g., `Two Handed Axe`, `Staff`, `Wand`) as the first line of what appears to be the properties section. The v1 parser detects this by checking if the line matches none of the known property patterns.

**Examples:**
- `Two Handed Axe` (item_base for "Two Hand Axes" class)
- `One Handed Sword` (item_base for "Thrusting One Hand Swords" class)
- `Bow` (item_base for "Bows" class)
- `Abyss` (sub-header for Abyss Jewels)

**Resolution:** Either maintain a list of known item_base strings, or use a negative check (if the line doesn't match any property pattern, it's an item_base). The item class can also guide this: if class is a weapon type, expect an item_base.

**Severity:** Medium. The v1 parser uses a long negative check with 17+ `!line.starts_with(...)` conditions. Fragile.

### B.4 Properties Section: Quality Variant

**Ambiguity:** Quality can appear as `Quality: +20% (augmented)` OR `Quality (Defence Modifiers): +20% (augmented)`. The v1 `QUALITY_PATTERN` regex doesn't handle the parenthetical qualifier.

**Resolution:** Update the regex to handle the optional qualifier: `Quality(?: \(.+?\))?: \+(\d+)% ...`

**Severity:** Medium -- affects PoE1 items with catalysts applied and PoE2 items.

### B.5 Properties Section: Weapon Range Format

**Ambiguity:** Weapon range appears as either `Weapon Range: 13` (old PoE1) or `Weapon Range: 1.4 metres` (newer PoE1). The v1 regex `^Weapon Range: (\d+)$` only handles the integer form.

**Resolution:** Update the regex to handle both: `^Weapon Range: ([\d.]+)(?: metres)?$`

**Severity:** Low-medium. Newer items all use the `metres` form.

### B.6 Elemental Damage Multi-Type Line

**Ambiguity:** The `Elemental Damage:` line can contain MULTIPLE damage ranges comma-separated:
```
Elemental Damage: 3-6 (augmented), 7-108 (augmented)
```
Each comma-separated segment is a different element. But the element type is NOT specified in the text -- you'd need to cross-reference with the item's mods to know which is Cold vs Lightning.

**Resolution:** In the advanced format, the mod headers explicitly state the element. The property line alone cannot determine which range is which element. Parse as a list of unnamed ranges.

**Severity:** Low for advanced format (mods tell you), high for simple format.

### B.7 Flask Base Properties vs Modifiers

**Ambiguity:** Flask properties include lines like `+5% to maximum Cold Resistance` and `+40% to Cold Resistance` which look identical to modifier lines. They appear in the properties section (before Requirements), not in the modifiers section.

**Resolution:** Position-based: anything before the separator after Requirements/Item Level is a property, not a modifier. Flask base effects are always in the properties section.

**Severity:** High if not handled positionally. These lines are indistinguishable from modifier text without positional context.

### B.8 Enchant Lines Without Headers

**Ambiguity:** Enchants do NOT have `{ }` modifier headers in the advanced format. They are bare lines with `(enchant)` suffix. However, multi-line enchants only have the `(enchant)` marker on specific lines:

```
8% increased Movement Speed if you haven't been Hit Recently
(Recently refers to the past 4 seconds) (enchant)
```

The first line here (`8% increased Movement Speed...`) has NO marker. You only know it's part of an enchant when you see the `(enchant)` on the next line. And the parenthetical reminder `(Recently refers to the past 4 seconds)` is itself part of the enchant text.

**Resolution:** Two approaches: (a) look-ahead to see if subsequent lines have `(enchant)`, or (b) group the enchant section positionally (between Item Level/Talisman Tier and the first `{ }` header).

**Severity:** High. Requires either look-ahead parsing or careful section boundary detection.

### B.9 Modifier Reminder Text vs Hybrid Lines

**Ambiguity:** Under a single `{ }` header, you can see:
```
{ Suffix Modifier "of Haemophilia" (Tier: 2) — Damage, Physical, Attack, Ailment }
Attacks have 25% chance to cause Bleeding
(Bleeding deals Physical Damage over time, based on the base Physical Damage of the Skill. Damage is higher while moving)
38(31-40)% increased Damage with Bleeding
```

Here there are 3 lines: a stat line, a reminder/explanation in parentheses, and another stat line. The parenthetical is NOT a separate stat -- it's reminder text for the first stat. But `38% increased Damage with Bleeding` IS a second stat of the same hybrid mod.

**Resolution:** Lines starting with `(` and ending with `)` are reminder text and should be associated with the preceding stat line, not treated as independent stats. Lines that match the numeric stat pattern are additional hybrid stats.

**Severity:** High. Misclassifying reminder text as stats produces incorrect modifier values.

### B.10 `(implicit)` Suffix on Reminder Text

**Ambiguity:** Reminder text can itself carry the `(implicit)` marker:
```
(50% of Damage from Suppressed Hits and Ailments they inflict is prevented) (implicit)
(Maximum Resistances cannot be raised above 90%) (implicit)
```

This is different from a stat line with `(implicit)`. The parenthetical content is reminder text, and the `(implicit)` signals it belongs to an implicit modifier.

**Resolution:** Parse the `(implicit)` suffix off ANY line, including reminder text. The `(implicit)` just confirms the section -- the parenthetical is still reminder text.

**Severity:** Medium. Can cause misparses if reminder text with `(implicit)` is treated as a stat.

### B.11 Negative Ranges (Ventor's Gamble edge case)

**Ambiguity:** The value format can have inverted or negative ranges:
```
1(10--10)% reduced Quantity of Items found
-9(-12--9)% maximum Player Resistances
```

In `10--10`, the range is `min=10, max=-10` (or possibly `min=-10, max=10`). The double negative `--` is ambiguous: is it `(-12) to (-9)` or `(-12-) to (9)`?

**Resolution:** The pattern is `(min-max)` where both min and max can be negative. `--` means "negative number followed by range separator followed by negative number". Parse as: split on `-` but account for leading `-` as sign. More robust: use a regex like `(-?\d+(?:\.\d+)?)-(-?\d+(?:\.\d+)?)`.

**Severity:** Low frequency (few items have negative ranges), but critical for correctness.

### B.12 Where Do Influence Markers Go?

**Ambiguity:** `Searing Exarch Item` / `Eater of Worlds Item` / etc. appear AFTER the explicit modifiers but BEFORE the separator. They are NOT in a `{ }` header. They look like standalone special properties but are co-located with modifiers.

```
{ Master Crafted Suffix Modifier ... }
+22(21-28)% to Lightning Resistance (crafted)
Searing Exarch Item              <-- here, no separator before it
Eater of Worlds Item             <-- here
--------                         <-- separator after
Fractured Item                   <-- this IS in its own section
```

**Resolution:** These lines match the pattern `^(Influence) Item$`. They should be parsed as special properties wherever they appear. The v1 parser handles this by checking `is_special_property()` during modifier parsing and breaking out of the modifier loop.

**Severity:** Medium. The co-location with modifiers means a pure section-based parser would misclassify them.

### B.13 Section Order Variation

**Ambiguity:** The section order is not fixed. Observed variations:

Standard equipment: Header | Properties | Requirements | Sockets | Item Level | (Enchants) | (Implicits) | Explicits | (Influence markers) | (Footer)

Maps: Header | Properties | Item Level | Monster Level | (Enchants) | (Implicits) | Explicits | Flavor | (Footer)

Flasks: Header | Properties | Requirements | Item Level | (Enchants) | Explicits | Flavor

Jewels: Header | Properties | (Requirements) | Item Level | Explicits | Flavor

Talismans: Header | (Properties) | Requirements | Item Level | Talisman Tier | (Enchants) | Implicits | Explicits | Flavor | Footer

**Resolution:** The state machine must be driven by content recognition (look-ahead), not fixed ordering. The v1 parser's `next_state()` does this by peeking at the next non-separator line and recognizing patterns.

**Severity:** High. A fixed-order parser WILL break on some item types.

### B.14 Map "Unique Modifier" Headers vs Equipment Unique Modifier Headers

**Ambiguity:** On unique maps and T17 reward maps, the modifiers use `{ Unique Modifier }` headers with no `(Tier:)`, same as unique equipment. But the modifier text is completely different (map mods vs item stats).

**Resolution:** Context from `item_class: Maps` determines interpretation. The mod text itself differs but the structure is identical.

**Severity:** Low -- same parsing, different semantics.

### B.15 "Modifiable only with..." Line

**Ambiguity:** T17 maps have a line like:
```
Modifiable only with Chaos Orbs, Vaal Orbs, Delirium Orbs and Chisels
```
This is neither a standard special property nor a modifier. The v1 parser does not have a pattern for this.

**Resolution:** Add a pattern or treat it as a special property variant.

**Severity:** Low -- only affects T17 maps.

### B.16 Locale-Specific Number Formatting

**Ambiguity:** The PoE2 fixture shows:
```
Lasts 9,60 (augmented) Seconds
```
This is `9.60` seconds with a comma decimal separator -- a European locale formatting. PoE1 uses period separators.

**Resolution:** Handle both `,` and `.` as decimal separators in numeric patterns. Or normalize the input.

**Severity:** Medium -- locale-dependent. Affects float parsing throughout.

---

## C. V1 Parser Critique

### C.1 What It Hardcodes That Shouldn't Be

1. **The item_base detection negative check** (`parse_properties`, lines 752-773): The parser checks if a line is NOT one of 17+ known property prefixes to determine it's an `item_base`. This is fragile -- any new property added by GGG breaks it. It should be a positive match against known item_base values from `base_items.json`.

2. **Weapon Range format** (`WEAPON_RANGE_PATTERN`): Only handles integer `Weapon Range: 13`, not the newer `Weapon Range: 1.4 metres`. This is a regex that was written for a specific era of PoE data.

3. **Quality format** (`QUALITY_PATTERN`): Doesn't handle `Quality (Defence Modifiers): +20%`. Hardcodes the assumption that Quality has no qualifier.

4. **Influence types** are hardcoded as a Rust enum with exactly 8 values. If GGG adds a new influence type, the parser must be recompiled.

5. **Rarity enum** is hardcoded to exactly 4 values. PoE2 or future expansions could add new rarities (e.g., "Currency" rarity for PoE2 gold).

6. **Flask property lines** are not explicitly handled -- the parser would try to match them as modifiers if they appeared in the wrong section.

7. **`Sockets: S S`** (PoE2 generic sockets): The `SOCKETS_PATTERN` regex `[RGBWS\-\s]+` accepts `S` but doesn't distinguish between PoE1 colored sockets and PoE2 generic sockets.

### C.2 Where It Uses Code Paths That Should Be Data-Driven

1. **State machine transitions** (`next_state`): Uses a cascade of `if` checks against regex patterns to determine the next state. This is essentially a hardcoded grammar. Adding a new section type requires modifying Rust code.

2. **Modifier header parsing** (`parse_modifier_header`): Uses 6 sequential regex matches against known header formats (Prefix, Suffix, Implicit, Unique, Crafted, Influence). New header formats require new regexes and new enum variants.

3. **Special property detection** (`is_special_property`): Hardcoded list of 6 patterns. New special properties (like the T17 `Modifiable only with...`) require code changes.

4. **Property parsing** (`parse_properties`): A massive 200+ line if/else chain matching property patterns. Each new property type is another branch.

5. **Influence type string matching**: The string-to-enum conversion in `parse_special_property` is a manual match statement.

### C.3 What Edge Cases It Explicitly Skips or Handles Poorly

1. **Magic item base type extraction**: The v1 parser treats Magic items the same as Rare (expects 2 lines: name + base_type). But Magic items only have 1 line. The fixtures show this working because the test fixtures happen to not include edge cases where this matters -- but the `parse_header` code at line 726 assumes "Magic, Rare, Unique items have both name and base type."

2. **Flask base properties**: No special handling. Flask property lines that look like modifiers would be swallowed into properties or skipped.

3. **Multi-line enchants without headers**: The enchant detection relies on `(enchant)` suffix per line. Multi-line enchants where only the last line has `(enchant)` would lose the first line.

4. **Hybrid modifier reminder text**: The parser collects ALL lines between headers as modifier text. Parenthetical reminder text becomes part of `hybrid_parts`. This is somewhat correct but loses the distinction between stat lines and reminder text.

5. **Map conversions `Unique Map:` and `Scarab:` lines**: Explicitly skipped with a comment "Skip unknown conversion types."

6. **PoE2 data**: The parser loads PoE2 game data files but the fixture coverage is exactly one item. Rune handling is present but minimally tested.

7. **`Relic Unique` footer**: Not handled -- no pattern for it.

8. **`Split` footer**: Not explicitly handled (but might be caught as an unrecognized line).

9. **Locale-specific decimals** (`9,60` seconds): Not handled.

### C.4 What It Gets RIGHT

1. **Two-pass architecture (parse then enrich)**: The parser first structurally parses the text into a `ParsedItem`, then runs `enrich_all_modifiers()` to look up stat IDs, calculate tiers, and compute roll quality. This separation is correct and should be preserved.

2. **Modifier header parsing**: The regex-based header parsing for `{ Prefix Modifier "name" (Tier: N) -- tags }` is thorough and handles all observed variants.

3. **Template extraction** (`extract_template`): Converting modifier text like `+45(42-45)% to Cold Resistance` into `{0}% to Cold Resistance` for database lookup is elegant and effective.

4. **Tier verification**: Cross-checking text-stated tiers against database-calculated tiers catches data errors.

5. **Base tag filtering**: Using base item tags to filter which mods can spawn on which bases makes tier calculation accurate.

6. **Mod group detection**: Detecting which mod group a stat value belongs to for accurate tier tables within that group.

7. **The overall structure** of `ParsedItem` with `header`, `properties`, `requirements`, `modifiers`, `special_properties` is a good output shape.

---

## D. Parsing Strategy Proposal

### D.1 The Core Problem

The PoE item text format is **semi-structured**: it has predictable delimiters (separators), predictable labeled lines (`Item Class:`, `Rarity:`, etc.), and predictable modifier headers (`{ ... }`). BUT it also has:

1. **Context-dependent interpretation** (Magic item names require base type lookup)
2. **Variable section ordering** (maps vs equipment vs flasks vs jewels)
3. **Lines whose meaning depends on position** (flask properties vs modifiers)
4. **Mixed-format sections** (influence markers inside modifier sections)

A pure PEG grammar cannot handle #1 (data-dependent disambiguation). A pure regex line-by-line parser (like v1) works but becomes a maintenance nightmare as the format grows.

### D.2 Approach Evaluation

#### Approach 1: Two-Pass (Grammar -> Ambiguous AST -> Resolution)

**How it works:**
1. Pass 1: A structural parser splits the text into sections by separators. Each section is classified by its first line(s) into a section type. Lines within sections are tagged but NOT fully interpreted. This produces a "raw AST" with ambiguous nodes.
2. Pass 2: A resolution layer uses game data to disambiguate. Magic item names are resolved. Modifier texts are matched to stat IDs. Tiers are verified.

**Advantages:**
- Clean separation of concerns
- The structural grammar is simple and stable (separator-delimited sections)
- Resolution can be updated independently (new base types, new mods) without changing the parser
- Testable: can test structural parsing without game data, and resolution with game data

**Disadvantages:**
- The ambiguous AST needs types that represent "this might be a name or a base type"
- Two passes means two error surfaces
- Some ambiguities (multi-line enchants) are hard to represent in a "still ambiguous" AST

#### Approach 2: Grammar with Callbacks (PEG with Semantic Actions)

**How it works:** A PEG grammar defines the structure, but at certain decision points, the parser calls out to a "resolver" that can query game data. For example, the Magic item header rule would call a function that checks whether a string is a base type.

**Advantages:**
- Single pass
- Grammar is self-documenting
- Decision points are explicit in the grammar

**Disadvantages:**
- PEG libraries (pest, nom) don't natively support semantic action callbacks that affect parse decisions
- Coupling between grammar and data makes the grammar less reusable
- Harder to test in isolation
- Error messages from PEG failures are notoriously poor

#### Approach 3: Pattern Table (Ordered Rules)

**How it works:** An ordered list of `(regex, context_predicate, handler)` rules. For each line, the parser tries each rule in order. The first match wins. Context predicates can check current state, previously parsed data, etc.

**Advantages:**
- Data-driven: new patterns can be added without code changes (if rules are loaded from config)
- Explicit priority ordering
- Context predicates handle the data-dependency naturally

**Disadvantages:**
- O(n*m) matching (n patterns per line) -- mitigated by context predicates pruning
- Rule ordering bugs are subtle and hard to debug
- No structural guarantee that the parse is complete or correct

#### Approach 4: Section-First Parser with Typed Handlers (Recommended)

**How it works:** A hybrid approach:

1. **Tokenizer**: Split input on separator lines into a `Vec<Section>` where each section is a `Vec<&str>` of non-separator lines.

2. **Section Classifier**: Each section is classified by examining its first line(s). Classification uses a combination of:
   - Pattern matching (regex for labeled lines like `Item Class:`, `Requirements:`, `Sockets:`)
   - Positional heuristics (section 0 is always Header, section 1 is always Properties-or-Requirements)
   - Content-based fallback (if lines contain `{ ... }` headers, it's a Modifier section)

3. **Typed Section Parsers**: Each section type has its own parser function that knows the exact format for that section. No state machine -- each section parser is independent.

4. **Data-Assisted Resolution**: A post-parse resolution pass uses game data for:
   - Magic item base type extraction from the affixed name
   - Modifier stat ID lookup via template matching
   - Tier verification and roll quality calculation
   - Influence implicit tier level interpretation

**Architecture:**

```
Input Text
    |
    v
[Tokenizer] -- splits on "--------" lines
    |
    v
Vec<RawSection>  -- each is Vec<&str>
    |
    v
[Section Classifier] -- examines first line(s) of each section
    |
    v
Vec<ClassifiedSection>  -- enum { Header(lines), Properties(lines), ... }
    |
    v
[Section Parsers] -- each section type has its own parser
    |
    v
RawItem  -- structured but may have unresolved fields
    |
    v
[Resolver] -- uses GameData (base_items, mods, stat_translations)
    |
    v
ResolvedItem  -- fully parsed and enriched
```

**Key types:**

```
enum SectionKind {
    Header,
    Properties,
    Requirements,
    Sockets,
    ItemLevel,
    MonsterLevel,
    TalismanTier,
    MapConversions,
    Enchants,
    Implicits,
    Explicits,
    InfluenceMarkers,
    FlavorText,
    SpecialProperties,
    Unknown(Vec<String>),  // preserve unrecognized sections
}

struct RawItem {
    header: RawHeader,        // may have unresolved base type for Magic
    sections: Vec<(SectionKind, Vec<String>)>,
}

struct RawHeader {
    item_class: String,
    rarity: Rarity,
    line_1: String,           // name or base type depending on rarity
    line_2: Option<String>,   // base type for Rare/Unique, None for Normal/Magic
}

struct ResolvedItem {
    header: ItemHeader,       // name and base_type fully resolved
    properties: ItemProperties,
    requirements: Requirements,
    sockets: Option<String>,
    item_level: u32,
    modifiers: ModifierList,
    special_properties: Vec<SpecialProperty>,
    flavor_text: Option<String>,
}
```

### D.3 Section Classification Algorithm

The section classifier runs through sections in order. It maintains a cursor of "what section am I likely looking at?" but unlike the v1 state machine, it classifies all sections in one sweep before parsing any:

1. Section 0: Always Header (starts with `Item Class:`)
2. Section 1: Examine first line:
   - If it matches a property pattern OR is a known item_base string -> Properties
   - If it starts with `Requirements:` or `Requires:` -> Requirements
   - If it starts with `Sockets:` -> Sockets
3. Subsequent sections: examine first line:
   - `Requirements:` / `Requires:` -> Requirements
   - `Sockets:` -> Sockets
   - `Item Level:` -> ItemLevel
   - `Monster Level:` -> MonsterLevel
   - `Talisman Tier:` -> TalismanTier
   - `Chance for dropped Maps to convert to:` -> MapConversions
   - Lines ending with `(enchant)` or `(rune)` -> Enchants
   - Lines starting with `{` -> starts a Modifiers section (may contain Implicits, Explicits, or a mix)
   - Known special property markers -> SpecialProperties
   - None of the above -> FlavorText or Unknown

This is more robust than the v1 approach because:
- Each section is classified independently
- Unknown sections are preserved, not silently dropped
- No mutable state machine that can get stuck in the wrong state

### D.4 Handling the Key Ambiguities

**Magic item base type (B.1):** The `RawHeader` preserves `line_1` without interpretation. The Resolver pass has a `BaseTypeIndex` (a trie or sorted list of all base type names from `base_items.json`). It finds the longest matching suffix/substring. This is explicitly a data-assisted step.

**Item_base detection (B.3):** Maintain a `Set<String>` of known item_base values (loaded from game data or hardcoded: `"Two Handed Axe"`, `"One Handed Sword"`, `"Wand"`, `"Staff"`, `"Bow"`, `"Abyss"`, etc.). Check the first line of the Properties section against this set. Positive match, not negative exclusion.

**Multi-line enchants (B.8):** Enchant sections are classified by the presence of `(enchant)` or `(rune)` on ANY line in the section. Then ALL lines in that section are treated as enchant text, grouped by `(enchant)` markers.

**Modifier reminder text (B.9):** Within a modifier section, after extracting `{ }` headers and grouping lines by header, classify each line as:
- Stat line: matches the numeric value pattern
- Reminder text: starts with `(` and ends with `)`
- Continuation: everything else

### D.5 Why This Approach Wins

1. **Separation of structural parsing from semantic interpretation**: The tokenizer and classifier deal only with format structure. The resolver deals with PoE game knowledge. They change for different reasons.

2. **Testable at every layer**: Tokenizer tests don't need game data. Classifier tests use small fixture sections. Resolver tests use game data fixtures.

3. **Extensible without code changes** (mostly): New property types, influence types, and special properties can often be added by updating data tables and classifier rules, not by writing new Rust functions.

4. **Handles unknowns gracefully**: Unknown sections are preserved, not dropped. This means new GGG format additions don't cause parse failures -- they just produce `Unknown` sections that can be inspected.

5. **Single responsibility per component**: The classifier's job is simple: "what kind of section is this?" It doesn't parse values. The section parser's job is simple: "extract structured data from this known-type section." The resolver's job is simple: "fill in the blanks using game data."

---

## E. PoE2 Differences

Based on the single PoE2 fixture available (`data/poe2/advanced.txt` -- a Dastard Armour body armour), the following differences are confirmed:

### E.1 Requirements Format

**PoE1:** Multi-line with header
```
Requirements:
Level: 70
Str: 155
```

**PoE2:** Single-line
```
Requires: Level 65, 60 (augmented) Str, 60 (augmented) Dex
```

The v1 parser handles both via separate regex paths (`REQUIREMENTS_HEADER_PATTERN` and `POE2_REQUIREMENTS_PATTERN`). The proposed architecture handles this in the Requirements section parser, which checks which format is used.

### E.2 Socket Format

**PoE1:** Colored sockets with links
```
Sockets: R-G-R-B
Sockets: R R-R-R-R
```

**PoE2:** Generic sockets
```
Sockets: S S
```

PoE2 sockets are generic (skill gem sockets, not color-coded). The socket line format is the same (`Sockets: ...`) but the content semantics differ. The parser can handle both by accepting `[RGBWAS\-\s]+` -- the downstream consumer decides how to interpret.

### E.3 Rune Modifiers

**PoE2 only.** Runes appear in the same position as enchants, with `(rune)` suffix:
```
+14% to Fire Resistance (rune)
+10% to Lightning Resistance (rune)
```

These are functionally similar to enchants -- applied by the player, not rolled. The proposed architecture classifies them into the Enchants section (shared with `(enchant)` lines).

### E.4 Implicit Modifier Section

**PoE2** has implicits in `{ }` headers just like PoE1:
```
{ Implicit Modifier — Life }
+64(60-80) to maximum Life (implicit)
```

No structural difference from PoE1 here.

### E.5 Stat Values in Simple Mode

**PoE1 simple mode:** No ranges, no headers
```
80% increased Armour and Energy Shield (fractured)
+42 to Strength
```

**PoE2 simple mode:** Same format
```
+64 to maximum Life (implicit)
+100 to maximum Life
```

Simple mode (Ctrl+C, not Ctrl+Alt+C) is identical between PoE1 and PoE2 in structure.

### E.6 Property: Memory Strands

Observed in PoE1 items but possibly relevant to PoE2:
```
Memory Strands: 38
```
This is a PoE1-specific property. PoE2 may have its own unique properties.

### E.7 Handling Strategy for Both Games

The proposed section-first architecture handles PoE1/PoE2 differences cleanly:

1. **GameVersion** is an input parameter, but most parsing is version-agnostic (same section structure, same separator format)
2. **Requirements section parser** checks both formats regardless of version (defensive)
3. **Socket parser** accepts both color-coded and generic formats
4. **Enchant/Rune section parser** accepts both `(enchant)` and `(rune)` suffixes
5. **Resolver** loads version-appropriate game data (`base_items.json` for the correct game)

The main risk is that PoE2's format may evolve differently from PoE1 as it matures. With only one PoE2 fixture, the analysis is necessarily limited. The architecture should be designed to handle unknown sections gracefully, so new PoE2-specific sections don't cause hard failures.

---

## Appendix: Section Order Summary Across All Fixtures

| Item Type | Sections in Order |
|-----------|-------------------|
| Rare armor (boots/gloves/helmet/body) | Header, Properties, Requirements, Sockets, ItemLevel, (Enchants), (Implicits), Explicits, (InfluenceMarkers), (Footer) |
| Rare weapon (axe/sword/wand) | Header, Properties*, Requirements, Sockets, ItemLevel, (Implicits), Explicits, (InfluenceMarkers), (Footer) |
| Normal weapon (staff) | Header, Properties*, Requirements, Sockets, ItemLevel, Implicits, (InfluenceMarkers) |
| Rare ring/amulet | Header, (Properties**), Requirements, (Sockets***), ItemLevel, (TalismanTier), (Enchants), Implicits, Explicits, (FlavorText), (Footer) |
| Unique ring/quiver | Header, (Properties**), Requirements, ItemLevel, Implicits, Explicits, FlavorText, (Footer) |
| Magic flask | Header, Properties****, Requirements, ItemLevel, (Enchants), Explicits, FlavorText |
| Magic weapon | Header, Properties*, Requirements, Sockets, ItemLevel, Explicits |
| Rare/unique map | Header, Properties*****, ItemLevel, MonsterLevel, (Enchants), (Implicits), Explicits, FlavorText, (Footer) |
| Normal map | Header, Properties*****, ItemLevel, MonsterLevel, FlavorText |
| Unique jewel | Header, Properties******, ItemLevel, Explicits, FlavorText |
| Abyss jewel | Header, Properties*******, Requirements, ItemLevel, Explicits, FlavorText |
| Rare belt | Header, Properties, Requirements, Sockets, ItemLevel, Implicits, Explicits, (InfluenceMarkers) |
| T17 map | Header, Properties*****, MapConversions, ItemLevel, MonsterLevel, Explicits, FlavorText, (Footer) |
| PoE2 body armour | Header, Properties, Requirements, Sockets, ItemLevel, (Runes), Implicits, Explicits |

*Properties includes item_base sub-header line for weapons.
**No defense/damage properties for rings/amulets (but may have Quality (Catalyst)).
***Stygian Vise has `Sockets: A` (abyssal).
****Flask properties include base effects that look like modifier lines.
*****Map properties include Map Tier, IIQ, IIR, Pack Size, Quality, Reward, More X.
******Jewel properties include `Limited to:` and `Radius:`.
*******Abyss Jewel has `Abyss` sub-header.

## Appendix: Complete Header Format Patterns Observed in `{ }` Headers

| Pattern | Example | Regex |
|---------|---------|-------|
| Prefix with Tier and Tags | `{ Prefix Modifier "Blue" (Tier: 2) -- Mana }` | `Prefix Modifier "(.+?)" \(Tier: (\d+)\)(?: -- (.+?))?` |
| Suffix with Tier and Tags | `{ Suffix Modifier "of the Lynx" (Tier: 8) -- Attribute }` | `Suffix Modifier "(.+?)" \(Tier: (\d+)\)(?: -- (.+?))?` |
| Prefix/Suffix without Tier | `{ Suffix Modifier "of the Essence" -- Attack, Speed }` | `(?:Prefix\|Suffix) Modifier "(.+?)"(?: -- (.+?))?` |
| Implicit with Tags | `{ Implicit Modifier -- Aura }` | `Implicit Modifier(?: -- (.+?))?` |
| Implicit without Tags | `{ Implicit Modifier }` | same |
| Unique with Tags | `{ Unique Modifier -- Defences }` | `Unique Modifier(?: -- (.+?))?` |
| Unique without Tags | `{ Unique Modifier }` | same |
| Master Crafted Prefix with Rank | `{ Master Crafted Prefix Modifier "Upgraded" (Rank: 3) -- Defences }` | `Master Crafted (?:Prefix\|Suffix)? ?Modifier "(.+?)"(?: \(Rank: (\d+)\))?(?: -- (.+?))?` |
| Master Crafted Suffix with Rank | `{ Master Crafted Suffix Modifier "of Craft" (Rank: 2) -- Elemental }` | same |
| Master Crafted without Rank | `{ Master Crafted Suffix Modifier "of Crafting" }` | same |
| Master Crafted Prefix no slot | `{ Master Crafted Prefix Modifier "Upgraded" -- Speed }` | same |
| Searing Exarch Implicit | `{ Searing Exarch Implicit Modifier (Greater) -- tags }` | `(Searing Exarch\|Eater of Worlds) Implicit Modifier \((.+?)\)(?: -- (.+?))?` |
| Eater of Worlds Implicit | `{ Eater of Worlds Implicit Modifier (Lesser) }` | same |

Note the v1 parser's `PREFIX_MODIFIER_PATTERN` requires `(Tier: N)` -- it will MISS prefix/suffix mods without tiers (like Essence mods `"of the Essence"` which have no Tier). The v1 code would fall through all the if/else branches and return `None`, causing that modifier's lines to be silently dropped. This is a confirmed bug in v1.

## Appendix: Value Line Suffix Markers

| Marker | Meaning | Example |
|--------|---------|---------|
| `(augmented)` | Value modified by quality/mods | `Armour: 1167 (augmented)` |
| `(implicit)` | Line is an implicit modifier | `+25% to Global Critical Strike Multiplier (implicit)` |
| `(crafted)` | Line is a crafted modifier | `+22(21-28)% to Lightning Resistance (crafted)` |
| `(fractured)` | Line is a fractured modifier | `Adds 19(14-21) to 34(32-38) Physical Damage (fractured)` |
| `(enchant)` | Line is an enchantment | `8% increased Movement Speed... (enchant)` |
| `(rune)` | Line is a rune modifier (PoE2) | `+14% to Fire Resistance (rune)` |
| `(unmet)` | Requirement not met by character | `Dex: 134 (unmet)` |
| `-- Unscalable Value` | Value cannot be scaled | `Hits can't be Evaded -- Unscalable Value` |

Note: the em-dash `--` in `-- Unscalable Value` is Unicode U+2014, not ASCII hyphens.
