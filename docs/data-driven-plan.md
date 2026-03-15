# Data-Driven Plan — Replacing Hardcoded PoE Knowledge with GGPK Data

Based on the GGPK deep dive (2026-03-15). See `docs/ggpk-data-deep-dive.md` for the full
table inventory and `_reference/ggpk-data-3.28/TABLE_INVENTORY.txt` for all 911 tables.

## Principle

Every piece of PoE game knowledge in our code should come from GGPK data files.
When it genuinely doesn't exist in the GGPK (trade API conventions, our subjective
interpretations), it's hardcoded with a dated comment citing the source.

---

## Phase 1: Extract ClientStrings into poe-data

**Goal**: Make `ClientStrings` available as a lookup table so poe-item and poe-trade
can validate their parsing patterns against GGG's authoritative display text.

### What to extract

Parse these ClientString ID prefixes into structured data:

| Prefix | Count | Use |
|--------|-------|-----|
| `ItemPopup*` | ~15 | Status/influence line text (Corrupted, Fractured Item, etc.) |
| `ItemDisplay*` | ~30 | Property display names (Armour, Evasion Rating, etc.) |
| `ModDescriptionLine*` | ~15 | Mod header templates (Prefix Modifier "{0}", Foulborn, etc.) |
| `WeaponClassDisplayName*` | ~20 | Weapon class display names |
| `SearchFilter*` | ~5 | GGG's own search aliases (ilvl, tier, etc.) |

### Implementation

1. **poe-dat**: Add `ClientStringRow { id: String, text: String }` to `tables/types.rs`
2. **poe-dat**: Add extraction in `tables/extract.rs` — schema: `Id(string=8), Text(string=8), ...`
   (row size is 52 bytes; we only need the first two string fields)
3. **poe-data**: Add `client_strings: HashMap<String, String>` to `GameData`
4. **poe-data**: Add lookup methods:
   - `client_string(id: &str) -> Option<&str>`
   - `item_popup_text(suffix: &str) -> Option<&str>` (prepends `ItemPopup`)
   - `item_display_text(suffix: &str) -> Option<&str>`
   - `mod_description_template(suffix: &str) -> Option<&str>`

### What this enables

- **poe-item**: Validate `StatusKind::parse()` and `InfluenceKind::parse()` against
  ClientStrings rather than hardcoding. When GGG adds a new status line (e.g., Imbued),
  it appears in ClientStrings automatically.
- **poe-trade**: Generate the property alias table by comparing `ItemDisplay*` text
  with trade `filters.json` text at runtime — reduces the 4-entry hardcoded alias
  table to zero hardcoded entries.
- **poe-item**: Generate `as_item_text()` from ClientStrings rather than hardcoded match.

---

## Phase 2: Extract ItemClasses capability flags

**Goal**: Replace `is_weapon_class()`, `is_armour_class()`, and `is_group_relevant()`
with data from `ItemClasses.CanBeCorrupted`, `CanHaveInfluence`, `CanBeFractured`.

### Implementation

1. **poe-dat**: Add fields to `ItemClassRow`:
   - `can_be_corrupted: bool`
   - `can_have_incubators: bool`
   - `can_have_influence: bool`
   - `can_be_double_corrupted: bool`
   - `can_be_fractured: bool`
   - `can_scourge: bool`
   - `can_upgrade_rarity: bool`
   (Byte offsets verified: bool fields at offsets 89-95 for equipment classes)
2. **poe-data**: Expose via `GameData`:
   - `item_class_can_be_corrupted(class: &str) -> bool`
   - `item_class_can_have_influence(class: &str) -> bool`
   - `item_class_can_be_fractured(class: &str) -> bool`
3. **poe-trade**: Replace `is_weapon_class()` / `is_armour_class()` / `is_group_relevant()`
   with queries against `GameData`
4. **poe-data**: Remove `is_weapon_class()` and `is_armour_class()` from `domain.rs`

### What this enables

- Trade filter groups shown/hidden per item class from GGPK data
- When GGG adds a new item class or changes capabilities, it's automatic

---

## Phase 3: Extract ArmourTypes / WeaponTypes / ShieldTypes

**Goal**: Enable DPS and defence calculations from base item stats.

### Implementation

1. **poe-dat**: Add row types:
   - `ArmourTypeRow { base_item_fk: u64, ar_min, ar_max, ev_min, ev_max, es_min, es_max, ward_min, ward_max }`
   - `WeaponTypeRow { base_item_fk: u64, crit, speed, dmg_min, dmg_max, range }`
   - `ShieldTypeRow { base_item_fk: u64, block }`
2. **poe-data**: Index by base item type FK, expose via `GameData`:
   - `base_armour(base_type: &str) -> Option<&ArmourTypeRow>`
   - `base_weapon(base_type: &str) -> Option<&WeaponTypeRow>`
   - `base_shield_block(base_type: &str) -> Option<u32>`
3. **poe-trade**: Use base stats for DPS/defence trade filter defaults
   (currently marked "skip for now" in `filter_default`)

---

## Phase 4: Add trade API convention comments

**Goal**: Every hardcoded trade API mapping has a dated comment explaining why
it's hardcoded and what GGPK table was checked.

### Files to update

**`crates/poe-data/src/domain.rs`:**
```rust
/// Maps item class name to trade API category URL slug.
///
/// Trade API convention, not in GGPK (verified 2026-03-15).
/// GGG's trade site uses its own category scheme that doesn't appear
/// in any datc64 table. Checked: BaseItemTypes (no TradeMarketCategory
/// field), ItemClasses, ItemClassCategories.
pub fn item_class_trade_category(item_class: &str) -> Option<&'static str> { ... }

/// Suffixes the trade API appends to stat display text.
///
/// Trade API convention, not in GGPK (verified 2026-03-15).
/// The GGPK stat_descriptions.txt only contains non-local stat text.
/// The trade API adds these suffixes to distinguish local variants.
pub const TRADE_STAT_SUFFIXES: &[&str] = &[" (Local)", " (Shields)"];
```

**`crates/poe-trade/src/filter_schema.rs`:**
```rust
/// Trade API uses shorter property names than GGPK item text.
///
/// Trade API convention, not in GGPK (verified 2026-03-15).
/// ClientStrings confirms GGG's item text uses the longer forms.
/// The trade filters.json uses these shortened forms.
const PROPERTY_ALIASES: &[(&str, &str)] = &[
    ("Evasion", "Evasion Rating"),  // ItemDisplayArmourEvasionRating = "Evasion Rating"
    ("Block", "Chance to Block"),   // ItemDisplayShieldBlockChance = "Chance to Block"
    ("Gem Level", "Level"),         // gems show "Level" as property name
    ("Gem Experience %", "Experience"),
];
```

---

## What stays hardcoded (with documentation)

| Item | Why hardcoded | GGPK checked |
|------|--------------|--------------|
| `TRADE_STAT_SUFFIXES` | Trade API convention | No suffix table in GGPK (2026-03-15) |
| `item_class_trade_category()` | Trade API URL scheme | No TradeMarketCategory in BaseItemTypes (2026-03-15) |
| `mod_trade_category()` | Trade API stat category prefixes | No trade category in Mods table (2026-03-15) |
| `PROPERTY_ALIASES` (4 entries) | Trade API shortens names | ClientStrings confirms GGPK uses long forms (2026-03-15) |
| `REQ_ALIASES` (4 entries) | Trade API uses full names, items use short | ClientStrings has both forms (2026-03-15) |
| `TierQuality` / `classify_tier()` | Our subjective interpretation | No quality concept in GGPK |
| `LOCAL_STAT_NONLOCAL_FALLBACKS` | GGPK naming convention for ~3 stats | Stats table has `is_local` but no equivalence map |
| `filter_default()` exception table (6 entries) | Dedicated fields (ilvl, sockets) don't match by text | These are ResolvedItem struct fields, not properties |

---

## Verification process for future changes

When adding a new PoE mechanic or game knowledge:

1. Run `extract_dat --all` to get fresh table data
2. Search `TABLE_INVENTORY.txt` for related keywords
3. Search ClientStrings for display text (`ItemPopup*`, `ItemDisplay*`, `ModDescriptionLine*`)
4. If found: extract in poe-dat → expose in poe-data → use in poe-item/poe-trade
5. If not found: hardcode with comment citing what was checked and when
6. Run `domain-knowledge-reviewer` agent to verify no leaks
