# Local vs Non-Local Stat ID Resolution

## Problem Statement

PoE has paired stat IDs for many stats — a **local** version that modifies the item's
base property, and a **non-local** (global) version that applies to the character:

| Local stat_id | Non-local stat_id | Distinction |
|---|---|---|
| `local_base_physical_damage_reduction_rating` | `base_physical_damage_reduction_rating` | Armour on the item vs global armour |
| `local_base_evasion_rating` | `base_evasion_rating` | Evasion on the item vs global evasion |
| `local_energy_shield` | `base_maximum_energy_shield` | ES on the item vs global ES |
| `local_minimum_added_physical_damage` / `local_maximum_added_physical_damage` | `global_minimum_added_physical_damage` / `global_maximum_added_physical_damage` | Weapon base damage vs flat added |
| `local_attack_speed_+%` | `attack_speed_+%` | Weapon attack speed vs global |
| `local_critical_strike_chance_+%` | `critical_strike_chance_+%` | Weapon crit vs global |

The Stats table in the GGPK has an `is_local` flag on each StatRow that authoritatively
identifies which stats are local. There is no GGPK mapping table between local and non-local
equivalents — the relationship is implicit in the naming convention (usually `local_` prefix,
but not always: `local_energy_shield` → `base_maximum_energy_shield`).

### Why it matters

- **DPS calculations**: Local weapon stats (flat phys, attack speed, crit chance) modify
  the weapon's base values multiplicatively. Global versions stack additively with other
  sources. Getting this wrong produces wildly incorrect DPS numbers.
- **Defence calculations**: Local armour/evasion/ES apply to the item's base defence.
  Global versions are separate additive bonuses.
- **Trade API**: GGG's trade site appends `(Local)` to local stat descriptions to
  disambiguate. Our `poe-trade` crate needs to know which stats are local.
- **Crafting**: When evaluating "what can I craft on this item?", local and global stats
  occupy different mod pools. Conflating them leads to wrong craft suggestions.

## Current State (the workaround)

### Reverse Index gives non-local stat_ids

The reverse index (`poe-dat::stat_desc::ReverseIndex`) maps display text → stat_ids.
It's built from `stat_descriptions.txt`, which only contains non-local stat IDs.
Local stats don't appear in stat_descriptions.txt because PoE renders them as base
item properties, not as stat description text.

So when poe-item's resolver processes `"+51 to Armour"`, the reverse index returns
`base_physical_damage_reduction_rating` (non-local), even though the actual mod
("Oyster's", a prefix on armour) uses `local_base_physical_damage_reduction_rating`.

### Mods table has the real stat_ids

The `Mods.datc64` table has `stat_keys` (FK → Stats table) that point to the **actual**
stat IDs used by each mod. For armour/defence mods, these are the local versions.

### Canonicalization workaround

In `game_data.rs::stat_suggestions_for_query()`, hybrid suggestions canonicalize the
Mods table's stat_ids through the reverse index to get the non-local equivalents.
This makes hybrid evaluation work (items and suggestions use the same non-local IDs)
but loses the local semantic information.

### Local stat template fallback

`set_reverse_index()` extends `stat_id_to_templates` with entries for local stat_ids
by checking for a non-local equivalent (strip `local_` prefix, or hardcoded fallback
via `LOCAL_STAT_NONLOCAL_FALLBACKS`). This lets `templates_for_stat()` return display
templates for local stats, used in hybrid suggestion `other_templates`.

---

## The Proper Approach: Base-Type-Anchored Resolution

### Core Insight

The reverse index is a **text matcher**, not a stat identity system. It answers "what
stat template does this text match?" — always returning non-local IDs because that's
what stat_descriptions.txt contains. It has no item context — it doesn't know if "+# to
Armour" is on a body armour (local) or a jewel (global).

A mod name lookup alone (e.g., "Oyster's" → ModRow) is also insufficient — it's still
a guess. We'd pick the first matching mod without verifying it can actually appear on
this item. Same "pick first" problem the reverse index has.

The **base item type** is the ground truth anchor. The GGPK's mod eligibility system
tells us exactly which mods (and therefore which stat_ids) are valid for a given item:

```
BaseItemType.tags  ←→  Mod.spawn_weight_tags (weight > 0)  →  Mod.stat_keys
```

Each base item has a tag list (e.g., Iron Greaves → `[default, boots, str_armour]`).
Each mod has `spawn_weight_tags` + `spawn_weight_values` — parallel arrays where
tag presence + weight > 0 means the mod can roll on items with that tag. Weight = 0
means explicitly blocked.

By combining:
- **Base type** (from item header) → tag set → eligible mods
- **Mod name** (from `{ }` header) → narrow to specific mod within eligible set
- **Reverse index** (from display text) → values + template matching

...we get **confirmed** stat_ids, not guessed ones.

### Why mod name alone is not enough

A mod named "X" might exist as both a local armour mod (spawns on body armour) and a
global armour mod (spawns on jewels) with different stat_keys. Without the base type
to anchor, we'd pick whichever ModRow we find first — the same "pick first" problem
the reverse index already has.

With the base type, we narrow to: "mods named 'X' that can actually spawn on this
base type." This is the definitive answer.

### Data we already extract

| Table | Field | Status |
|-------|-------|--------|
| `BaseItemTypeRow.tags` | FK list to Tags | Extracted |
| `BaseItemTypeRow.implicit_mods` | FK list to Mods (implicit mods for this base) | Extracted |
| `ModRow.spawn_weight_tags` | FK list to Tags | Extracted |
| `ModRow.spawn_weight_values` | Parallel weight values | Extracted |
| `ModRow.stat_keys` | FK to Stats (the actual stat_ids) | Extracted |
| `StatRow.is_local` | Whether this stat is local to the item | Extracted |

All the data is ready. We just need to build the intersection logic.

---

## Attack Plan

**Principle**: Build correct from GGPK up. Upper layers adapt to truth.
No backward-compat hacks. Profiles can be recreated — app is not live.

### Phase 1: Foundation — correct data layer (poe-data)

**Goal**: GameData can answer "given this base type and mod name, what are the real stat_ids?"

#### 1a. `mods_by_name` index in GameData

Add `mods_by_name: HashMap<String, Vec<usize>>` built during `load()`.
Multiple tiers share a name; they all have the same stat_keys.

#### 1b. `find_eligible_mod(base_type, mod_name)` in GameData

```rust
pub fn find_eligible_mod(&self, base_type: &str, mod_name: &str) -> Option<&ModRow> {
    let base = self.base_item_by_name(base_type)?;
    let base_tags: HashSet<u64> = base.tags.iter().copied().collect();
    let mod_indices = self.mods_by_name.get(mod_name)?;

    mod_indices.iter().find_map(|&idx| {
        let m = &self.mods[idx];
        let eligible = m.spawn_weight_tags.iter().zip(&m.spawn_weight_values)
            .any(|(&tag, &weight)| weight > 0 && base_tags.contains(&tag));
        eligible.then_some(m)
    })
}
```

### Phase 2: Items carry truth (poe-item)

**Goal**: `ResolvedStatLine.stat_ids` contains the real stat_ids from the GGPK,
not the reverse index's best guess.

#### 2a. `resolve_mod()` gets base type context

Currently `resolve_mod(group, game_data)`. Add base_type parameter (already
resolved in the header by this point).

After reverse index stat line resolution (which gives us values + template match):

1. Call `game_data.find_eligible_mod(base_type, mod_name)`
2. Get mod's `stat_keys` → resolve to stat_id strings via `game_data.stat_id(fk)`
3. Replace each stat line's reverse-index stat_ids with the confirmed stat_ids
   from the mod's stat_keys

The reverse index still provides **values** and **template matching**. The mod
lookup provides **confirmed stat identity**.

Fallback: if eligible mod not found, keep reverse index stat_ids (graceful
degradation for unknown mods/bases).

### Phase 3: Suggestions carry truth (poe-data)

**Goal**: `stat_suggestions_for_query()` returns real stat_ids from Mods table.

#### 3a. Remove canonicalization workaround

Delete the `canonical_other_stat_ids` mapping. Use Mods table stat_keys directly.
The suggestions now carry the same stat_ids that items will carry.

#### 3b. Remove `resolve_stat_template` workaround in app

This command currently returns reverse index stat_ids (non-local). With suggestions
carrying real stat_ids, the UI should use those instead. `resolve_stat_template`
either returns real stat_ids or gets removed — the suggestion already has them.

### Phase 4: Upper layers adapt (poe-eval + app)

**Goal**: Evaluation and UI work with real stat_ids.

Items and suggestions now both carry real stat_ids. The evaluator's `eval_stat_value()`
does exact `stat_id == stat_id` matching — this **should just work** because both
sides now carry the same real IDs.

If something breaks, it's an upper-layer bug to fix in the upper layer. No
equivalence hacks in the data layer.

#### What about `resolve_stat_template` for single stat picks?

Currently the user types in the autocomplete, picks a template, and we call
`resolve_stat_template` to get the stat_id for the rule. This returns the reverse
index stat_id (non-local). With items carrying real stat_ids, this won't match.

The fix is in the upper layer: the suggestion dropdown already returns
`StatSuggestion.stat_ids` — use those (which are now real) instead of calling
`resolve_stat_template` separately. The `resolve_stat_template` command may
become unnecessary.

### Phase 5: Trade and DPS (future)

Trade API uses `(Local)` suffix — `is_local` from the stat_id or Stats table
enables correct matching. DPS calculations use `is_local` to know which stats
modify the weapon/armour base vs adding globally.

---

## Edge Cases

1. **Base type not found**: New league bases not in our table. Fall back to
   mod-name-only lookup → reverse index. Graceful degradation.

2. **Mod not found by name**: New league mods. Fall back to reverse index stat_id.

3. **Mod name + base type → no eligible mod**: Mod exists but spawn weights say it
   can't appear on this base type. Corrupted data or GGPK update. Log warning,
   fall back to reverse index.

4. **Multiple eligible tiers**: Several tiers of "Oyster's" are all eligible.
   They all share stat_keys — pick any. No ambiguity.

5. **Unique item mods**: `generation_type=3`. May not have standard spawn weights.
   Use mod-name-only lookup as fallback.

6. **Crafted mods**: `{ Master Crafted ... }` — bench crafts have Mods table
   entries with spawn weights. Should resolve correctly.

7. **No `{ }` header (Ctrl+C format)**: No mod name. Reverse index only. Already
   a degraded path.

---

## What Changes (summary)

| Change | Where | Size |
|--------|-------|------|
| `mods_by_name` index | `poe-data/game_data.rs` | ~15 lines |
| `find_eligible_mod()` | `poe-data/game_data.rs` | ~15 lines |
| Base-type-anchored resolve | `poe-item/resolver.rs` | ~25 lines in `resolve_mod()` |
| Remove canonicalization | `poe-data/game_data.rs` | Delete ~10 lines |
| Suggestions use real stat_ids | `poe-data/game_data.rs` | ~5 lines |
| UI uses suggestion stat_ids | `app/PredicateEditor.tsx` | Adapt to new data |
| Remove/update `resolve_stat_template` | `app/src-tauri/lib.rs` | TBD |

Saved profiles will need to be recreated (stat_ids change). App is not live.
