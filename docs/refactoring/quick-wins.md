# Quick Wins

**Priority:** 4
**Status:** TODO
**Effort:** ~1 hour total
**Risk:** Minimal

Small improvements that can be done in a single session.

## 1. poe-data: Extract find_eligible_mod_candidates() (~15 min)

**File:** `crates/poe-data/src/game_data.rs:510-589`

`find_eligible_mod()` and `find_eligible_mods()` share ~60 lines of identical filtering logic
(base tag resolution, domain filtering, spawn weight calculation). Extract to:

```rust
fn find_eligible_mod_candidates(
    &self,
    base_type: &str,
    mod_name: &str,
    item_class: &str,
) -> Vec<(&ModRow, i32)> { /* shared logic */ }
```

Then `find_eligible_mod()` calls `.into_iter().next()` and `find_eligible_mods()` calls `.collect()`.

## 2. poe-eval: Extract rarity ID helper (~5 min)

**Files:** `crates/poe-eval/src/affix.rs:86,111` and `evaluate.rs:250`

Same pattern repeated 3x:
```rust
let rarity_str = format!("{:?}", item.header.rarity);
let Some(rarity_id) = poe_data::domain::rarity_to_ggpk_id(&rarity_str) else { ... };
```

Extract to a helper in poe-eval (or add a `Rarity::to_ggpk_id()` method in poe-data):
```rust
fn rarity_ggpk_id(rarity: Rarity) -> Option<i32> {
    poe_data::domain::rarity_to_ggpk_id(&format!("{rarity:?}"))
}
```

## 3. poe-data: set_reverse_index() local stat helper (~15 min)

**File:** `crates/poe-data/src/game_data.rs:283-356`

Extract the local stat fallback logic (lines 297-340) into:
```rust
fn build_local_stat_templates(
    &self,
    map: &mut HashMap<String, Vec<String>>,
    native_templates: &HashMap<String, Vec<String>>,
)
```

Reduces `set_reverse_index()` from 73 to ~30 lines.

## 4. poe-dat: Add row_size validation (~5 min)

**File:** `crates/poe-dat/src/dat_reader.rs:80`

Add a check after computing row_size:
```rust
if row_count > 0 && rows_total_size % row_count as usize != 0 {
    return Err(DatError::InconsistentRowSize);
}
```

Defensive against corrupted dat files (GGG files are well-formed, but good hygiene).
