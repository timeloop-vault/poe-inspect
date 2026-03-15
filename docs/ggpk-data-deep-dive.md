# GGPK Data Deep Dive — Full Inventory

Investigated 2026-03-15. All 911 datc64 tables extracted to `_reference/ggpk-data-3.28/`.

## Key Discovery: ClientStrings Contains Everything

`clientstrings.datc64` (8,264 rows, 1.7MB) is GGG's master localization table.
It contains the display text for **every** item property, status, influence marker,
mod header format, and UI label. This is the authoritative source for all text
that appears in item tooltips.

### Item Property Display Names

| ClientString ID | Display Text | Trade Filter Text | Match? |
|---|---|---|---|
| `ItemDisplayArmourArmour` | `Armour` | `Armour` | Exact |
| `ItemDisplayArmourEvasionRating` | `Evasion Rating` | `Evasion` | **Mismatch** |
| `ItemDisplayArmourEnergyShield` | `Energy Shield` | `Energy Shield` | Exact |
| `ItemDisplayArmourWard` | `Ward` | `Ward` | Exact |
| `ItemDisplayShieldBlockChance` | `Chance to Block` | `Block` | **Mismatch** |
| `ItemDisplayWeaponPhysicalDamage` | `Physical Damage` | `Damage` | **Mismatch** |
| `ItemDisplayWeaponAttacksPerSecond` | `Attacks per Second` | `Attacks per Second` | Exact |
| `ItemDisplayWeaponCriticalStrikeChance` | `Critical Strike Chance` | `Critical Chance` | **Mismatch** |
| `ItemDisplayStringItemLevel` | `Item Level` | `Item Level` | Exact |
| `ItemDisplayStringRarity` | `Rarity` | (type_filter) | Exact |
| `ItemDisplayStringSockets` | `Sockets` | `Sockets` | Exact |
| `ItemDisplayStringTalismanTier` | `Talisman Tier` | `Talisman Tier` | Exact |
| `ItemDisplayMapTier` | `Map Tier` | `Map Tier` | Exact |
| `ItemDisplayMapQuantityIncrease` | `Item Quantity` | `Map IIQ` | **Mismatch** |
| `ItemDisplayMapRarityIncrease` | `Item Rarity` | `Map IIR` | **Mismatch** |
| `ItemDisplayMapPackSizeIncrease` | `Monster Pack Size` | `Map Packsize` | **Mismatch** |
| `Quality` | `Quality` | `Quality` | Exact |

**Conclusion**: 7 mismatches between GGG's item text and GGG's trade filter text.
These are trade API shortenings — the GGPK confirms our poe-item parser is correct.
The alias table in poe-trade is genuinely needed for these 7 cases.

### Item Status / Influence Lines (ItemPopup*)

| ClientString ID | Display Text | Used in poe-item |
|---|---|---|
| `ItemPopupCorrupted` | `Corrupted` | `StatusKind::Corrupted` |
| `ItemPopupMirrored` | `Mirrored` | `StatusKind::Mirrored` |
| `ItemPopupSplit` | `Split` | `StatusKind::Split` |
| `ItemPopupUnidentified` | `Unidentified` | `StatusKind::Unidentified` |
| `ItemPopupUnmodifiable` | `Unmodifiable` | `StatusKind::Unmodifiable` |
| `ItemPopupShaperItem` | `Shaper Item` | `InfluenceKind::Shaper` |
| `ItemPopupElderItem` | `Elder Item` | `InfluenceKind::Elder` |
| `ItemPopupCrusaderItem` | `Crusader Item` | `InfluenceKind::Crusader` |
| `ItemPopupHunterItem` | `Hunter Item` | `InfluenceKind::Hunter` |
| `ItemPopupRedeemerItem` | `Redeemer Item` | `InfluenceKind::Redeemer` |
| `ItemPopupWarlordItem` | `Warlord Item` | `InfluenceKind::Warlord` |
| `ItemPopupSearingExarchItem` | `Searing Exarch Item` | `InfluenceKind::SearingExarch` |
| `ItemPopupEaterofWorldsItem` | `Eater of Worlds Item` | `InfluenceKind::EaterOfWorlds` |
| `ItemPopupFracturedItem` | `Fractured Item` | `InfluenceKind::Fractured` |
| `ItemPopupSynthesisedItem` | `Synthesised Item` | `InfluenceKind::Synthesised` |
| `ItemPopupFoilUnique` | `Foil Unique` | Not parsed |
| `ItemPopupHellscaped` | `Scourged` | Not parsed (legacy) |
| `ItemPopupAlternateGemItem` | `Transfigured` | `StatusKind::Transfigured` |
| `ItemPopupImbued` | `Imbued` | Not parsed |

**No ItemPopupFoulborn** — Foulborn items don't get a status line at the bottom.

### Mod Header Templates (ModDescriptionLine*)

| ClientString ID | Template | Produces |
|---|---|---|
| `ModDescriptionLinePrefix` | `Prefix Modifier "{0}"` | `{ Prefix Modifier "Merciless" }` |
| `ModDescriptionLineSuffix` | `Suffix Modifier "{0}"` | `{ Suffix Modifier "of the Sage" }` |
| `ModDescriptionLineFractured` | `Fractured {0}` | `{ Fractured Prefix Modifier "Encased" }` |
| `ModDescriptionLineCrafted` | `Master Crafted {0}` | `{ Master Crafted Prefix Modifier }` |
| `ModDescriptionLineImplicit` | `Implicit Modifier` | `{ Implicit Modifier }` |
| `ModDescriptionLineUnique` | `Unique Modifier` | `{ Unique Modifier }` |
| `ModDescriptionLineBrequelMutated` | `Foulborn Unique Modifier` | `{ Foulborn Unique Modifier }` |
| `ModDescriptionLineCleansingFireImplicit` | `Searing Exarch Implicit Modifier ({0})` | `{ Searing Exarch Implicit Modifier (Lesser) }` |
| `ModDescriptionLineGreatTangleImplicit` | `Eater of Worlds Implicit Modifier ({0})` | `{ Eater of Worlds Implicit Modifier (Lesser) }` |
| `ModDescriptionLineCorruptedImplicit` | `Corruption Implicit Modifier` | `{ Corruption Implicit Modifier }` |
| `ModDescriptionLineEnchantmentImplicit` | `Enchantment Modifier` | `{ Enchantment Modifier }` |
| `ModDescriptionLineDesecrated` | `Desecrated {0}` | `{ Desecrated Prefix Modifier }` |
| `ModDescriptionLineHellscape` | `Scourge Modifier` | `{ Scourge Modifier }` |
| `ModDescriptionLineWeaponPassiveTreeAllocated` | `Allocated Crucible Passive Skill` | `{ Allocated Crucible Passive Skill }` |
| `ModDescriptionLineTier` | ` (Tier: {0})` | `(Tier: 2)` |
| `ModDescriptionLineRank` | ` (Rank: {0})` | `(Rank: 1)` |
| `ModDescriptionLineLevel` | ` (Lvl: {0})` | `(Lvl: 83)` |

**Key insight**: GGG internally calls Foulborn "Brequel Mutated" (`BrequelMutated`).
The trade API uses `mutated` as the filter ID. "Foulborn" is the user-facing name.

### Other Useful ClientStrings

| ClientString ID | Text | Use |
|---|---|---|
| `QualityItem` | `Superior {0}` | Quality prefix on item names |
| `SynthesisedItem` | `Synthesised {0}` | Synthesised prefix |
| `MutatedUniqueName` | `Foulborn {0}` | Foulborn item name prefix |
| `ItemDisplayArmourAdaptationRating` | `Adaptation Rating` | New 3.28 property? |

---

## InfluenceTags Table

144 rows. Maps `(ItemClass, InfluenceType) → Tag`.

Only covers the **6 original influences**: Shaper, Elder, Crusader, Hunter (Eyrie),
Redeemer (Basilisk), Warlord (Adjudicator).

**Does NOT include**: Searing Exarch, Eater of Worlds, Synthesised, Fractured.
Those are handled through different mechanisms (eldritch implicits, item flags).

The tag names follow the pattern: `{class}_{influence}` (e.g., `boots_crusader`,
`2h_axe_elder`). These tags are used in `Mods.spawn_weight_tags` to control
which influence-specific mods can roll on which item classes.

---

## ArmourTypes Table

481 rows, 60-byte rows. Maps `BaseItemType → (ArmourMin, ArmourMax, EvasionMin, EvasionMax, ESMin, ESMax, MovementSpeed, WardMin, WardMax)`.

Contains base defence values for all armour, shields, and quivers. Pure armour bases
have AR only, pure evasion have EV only, hybrid bases have both.

**Use case**: DPS/defence calculations, computing total defence from base + local mods.

---

## WeaponTypes Table

365 rows, 40-byte rows. Maps `BaseItemType → (Critical, Speed, DamageMin, DamageMax, RangeMax)`.

- Critical is in hundredths (e.g., 800 = 8.00%)
- Speed is in ms/attack (e.g., 667 = 1000/667 = 1.50 APS)

**Use case**: DPS calculations from base weapon stats + local mods.

---

## ShieldTypes Table

98 rows, 20-byte rows. Maps `BaseItemType → Block`.

Block chance for all shield bases (23-29% range).

**Use case**: Block chance display, trade filter defaults.

---

## ItemClasses Table

99 rows, 153-byte rows. Contains capability flags we currently don't extract:

- `CanBeCorrupted` (offset ~89)
- `CanHaveIncubators` (offset ~90)
- `CanHaveInfluence` (offset ~91)
- `CanBeDoubleCorrupted` (offset ~92)
- `CanBeFractured` (further down, needs precise mapping)
- `CanScourge`
- `CanHaveVeiledMods` (offset 67, already extracted)

Equipment classes (Body Armour, Boots, etc.) show `true` for all.
Currency shows `false` for all. Offsets verified by comparing equipment vs currency rows.

**Use case**: Replace hardcoded `is_group_relevant()` in trade filter schema — determine
which trade filter groups apply per item class from GGPK data.

---

## ComponentAttributeRequirements Table

861 rows, 20-byte rows. Maps `BaseItemType → (ReqStr, ReqDex, ReqInt)`.

Base attribute requirements for all equippable items.

**Use case**: Could provide defaults for requirement trade filters. Currently we parse
these from item text (which already works).

---

## Tables NOT Useful for Trade Mapping

| Table | Why |
|---|---|
| `InfluenceExalts` | 6 rows, maps currency items to influence types. Not useful for item evaluation. |
| `ItemVisualIdentity` | 14MB, visual appearance data. Not relevant for trade. |
| `BuffVisuals` | 1MB, buff effect visuals. Not relevant. |

---

## Automation Opportunities

### What we CAN automate from GGPK data:

1. **Property display names** → Extract from ClientStrings (`ItemDisplay*` patterns).
   These are the authoritative names used in item text.

2. **Status/influence lines** → Extract from ClientStrings (`ItemPopup*` patterns).
   These are exactly what appears at the bottom of items.

3. **Mod header templates** → Extract from ClientStrings (`ModDescriptionLine*`).
   These define the `{ }` header format in Ctrl+Alt+C.

4. **Item class capabilities** → Extract `CanBeCorrupted`, `CanHaveInfluence`,
   `CanBeFractured` from ItemClasses table.

5. **Base defence/weapon values** → ArmourTypes, WeaponTypes, ShieldTypes
   for DPS/defence calculations.

### What we CANNOT automate (trade API conventions):

1. **Trade filter text shortenings** — "Evasion" vs "Evasion Rating", etc.
   These are trade API decisions, not in GGPK. Need alias table (7 entries).

2. **Trade category mapping** — "Boots" → "armour.boots". Not in GGPK.
   Need hardcoded map (or derive from trade API `items.json`).

3. **Trade stat categories** — `explicit.stat_*` vs `fractured.stat_*`.
   Not in GGPK. Comes from trade API `stats.json`.

---

## Additional Tables Explored

### ItemFrameType (14 rows)

Rarity header definitions with exact RGB colors and art paths:

| Rarity | Color | Header Art |
|---|---|---|
| Normal | `rgb(200,200,200)` | `ItemsHeaderWhite` |
| Magic | `rgb(136,136,255)` | `ItemsHeaderMagic` |
| Rare | `rgb(255,255,119)` | `ItemsHeaderRareSingleLine` / `ItemsHeaderRare` |
| Unique | `rgb(175,96,37)` | `ItemsHeaderUnique` |
| Gem | `rgb(27,162,155)` | `ItemsHeaderGem` |
| Currency | (continues...) | `ItemsHeaderCurrency` |

**Use case**: Replace hardcoded rarity colors in overlay CSS with GGPK-sourced values.

### CraftingItemClassCategories (21 rows)

Maps crafting bench categories: `OneHandMelee` → `One Hand Melee`, `BodyArmour` → `Body Armour`, etc.
Internal IDs + display names for the crafting bench UI.

### ItemClassCategories (64 rows)

High-level categories with internal IDs + display names. Includes new league content:
`BrequelFruit` = `Wombgift`, `AtlasRelics` = `Idol`, `NecropolisPack` = `Ember of the Allflame`.

### EssenceType (26 rows)

Essence names: Hatred, Woe, Greed, Contempt, Sorrow, Anger, Torment, Fear, etc.

### PlayerTradeWhisperFormats (4 rows)

Trade whisper templates: `Hi, I would like to buy your {{ITEM}} listed for {{PRICE}} in {{LEAGUE}}`.

### BrequelGrafts (16 rows) — Foulborn Grafts

The graft abilities from the Keepers of the Flame / Foulborn mechanic.
Internal names like `UulNetolGraft1`, `XophGraft1`. Contains art paths and AI script references.

---

## Next Steps

1. **Extract ClientStrings as a data source** — parse `ItemPopup*`, `ItemDisplay*`,
   `ModDescriptionLine*` patterns into poe-data for use by poe-item and poe-trade.

2. **Extract ItemClasses capability flags** — add `CanBeCorrupted`, `CanHaveInfluence`,
   `CanBeFractured` to `ItemClassRow` in poe-dat.

3. **Extract ArmourTypes/WeaponTypes/ShieldTypes** — add row types to poe-dat
   for DPS/defence calculations.

4. **Use ClientStrings in poe-item** — validate/generate status/influence parsing
   from `ItemPopup*` patterns instead of hardcoding.

5. **Use ClientStrings in poe-trade** — generate the property alias table from
   comparing `ItemDisplay*` text with trade `filters.json` text.
