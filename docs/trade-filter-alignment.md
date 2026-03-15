# Trade Filter Alignment — Cross-Crate Naming Strategy

## Problem

The trade API's `filters.json` defines 89 structural filters with labels like
`"Armour"`, `"Item Level"`, `"Corrupted"`. Our item parser (poe-item) produces
properties, statuses, and influences with their own naming. Connecting these
requires a mapping — and the fewer manual mappings, the more maintainable.

## Principle

**poe-item surfaces item data using names from the game text.**
**poe-trade matches filter labels against item data by text.**
**Only genuine GGG naming inconsistencies require explicit mappings.**

## Strategy: Text Matching with Exception Table

### How filter defaults work

When the user inspects an item and enters Edit Search mode, poe-trade's
`trade_edit_schema()` projects the item onto the filter schema. For each
filter from `filters.json`, it determines:
- Does this item have a value for this filter?
- What is the default value?
- Should it start enabled?

### Three matching strategies (tried in order)

1. **Property match**: filter text → `item.properties` by name.
   `"Armour"` matches property `"Armour"` → value `1299`.
   Works for most armour, weapon, map, and misc properties.

2. **Status/influence match**: filter text → `item.statuses` or `item.influences`.
   `"Corrupted"` matches `StatusKind::Corrupted` → enabled.
   `"Fractured Item"` matches `InfluenceKind::Fractured` → enabled.
   `"Searing Exarch Item"` matches `InfluenceKind::SearingExarch` → enabled.

3. **Dedicated field match**: filter text → typed `ResolvedItem` fields.
   `"Item Level"` → `item.item_level`.
   `"Sockets"` → `item.socket_info.total`.
   `"Links"` → `item.socket_info.max_link`.

### Exception table (genuine GGG naming mismatches)

These are cases where the PoE item text uses a different name than the
trade API filter label:

| Trade filter text | poe-item source | Reason |
|---|---|---|
| `"Evasion"` | property `"Evasion Rating"` | GGG uses full name in item, short in trade |
| `"Block"` | property `"Chance to Block"` | Same pattern |
| `"Sockets"` | `socket_info.total` | Not a property, computed field |
| `"Links"` | `socket_info.max_link` | Not a property, computed field |

This table is ~4 entries, not ~30. When GGG adds a new property-based filter,
text matching picks it up automatically.

## What lives where

### poe-item owns
- Parsing item text into structured data (properties, statuses, influences)
- **Socket metadata** (total, max_link, groups) — not raw string
- **Quality** as a typed field — not buried in properties
- Property names exactly as PoE displays them (no renaming)

### poe-data owns
- PoE domain knowledge (item classes, mod domains, trade categories)
- `is_weapon_class()`, `is_armour_class()` for filter group relevance

### poe-trade owns
- Parsing `filters.json` into `FilterIndex`
- The text matching logic (filter label → item data)
- The exception table for naming mismatches
- Building `TradeEditSchema` per item
- All trade API query construction

### app owns
- Rendering `TradeEditSchema` generically (range → inputs, option → dropdown)
- Sending user values back to poe-trade
- Zero knowledge of what filters exist or what they mean

## Implementation Steps

1. **poe-item: Add `SocketInfo` to `ResolvedItem`** — parse socket string during
   resolve, expose `total`, `max_link`, socket groups. Remove raw `sockets` string
   or keep alongside for display.

2. **poe-item: Add `quality` field to `ResolvedItem`** — extract from properties
   during resolve. Still keep the property for display.

3. **poe-trade: Refactor `filter_default()`** — replace hardcoded match table with
   text matching against properties/statuses/influences + small exception map.

4. **poe-trade: Remove `parse_socket_info()` and `extract_quality()`** — use the
   pre-computed fields from `ResolvedItem` instead.

## Future

When GGG adds a new filter to `filters.json`:
- If it matches a property name → works automatically
- If it matches a status/influence → works automatically
- If it's a new computed value or naming mismatch → add one line to exception table
