# Hybrid Mod Picker — Plan

## Goal

When a user searches for a stat (e.g., "life") in the stat picker, show both:
- **Individual stat**: `+# to maximum Life` → creates a simple `StatValue` predicate
- **Hybrid mod combos**: `+# to maximum Life / +# to Armour` (Urchin's) → auto-creates a `Rule::All` with two `StatValue` predicates

The GGPK `Mods` table knows which mods have multiple stats. This is real game data, not guessing.

## Data Flow

```
poe-data (mod stat index)  →  app backend (suggestions)  →  frontend picker
```

## Implementation Steps

### Step 1: poe-data — Build stat-to-mod index

Add a reverse index from stat ID → mods that contain that stat.

**File**: `crates/poe-data/src/game_data.rs`

- New field: `stat_to_mods: HashMap<String, Vec<usize>>` (stat_id → indices into `self.mods`)
- Built during `GameData::load()` by iterating all mods and their `stat_keys`
- Need to resolve stat FKs to stat IDs (join `stat_keys` with `stats` table)
- New method: `mods_with_stat(&self, stat_id: &str) -> Vec<&ModRow>`
- New method: `hybrid_mods_with_stat(&self, stat_id: &str) -> Vec<HybridModInfo>`
  where `HybridModInfo` = `{ mod_name, generation_type, stat_templates: Vec<String> }`

This is pure game data indexing — belongs in poe-data.

### Step 2: poe-data — Expose hybrid mod suggestions

New method that returns enriched stat suggestions:

```rust
pub struct StatSuggestion {
    pub template: String,        // "+# to maximum Life"
    pub stat_ids: Vec<String>,   // ["base_maximum_life"]
    pub kind: StatSuggestionKind,
}

pub enum StatSuggestionKind {
    Single,
    Hybrid {
        mod_name: String,              // "Urchin's"
        other_templates: Vec<String>,  // ["+# to Armour"]
        other_stat_ids: Vec<String>,   // ["base_armour"]
    },
}
```

Method: `stat_suggestions_for_query(&self, query: &str) -> Vec<StatSuggestion>`

Returns both single-stat matches and hybrid mod combos that include the queried stat.
This is still game data — no evaluation logic.

### Step 3: App backend — New Tauri command

**File**: `app/src-tauri/src/lib.rs`

New command `get_stat_suggestions(query: String)` → returns `Vec<StatSuggestion>`.
Calls `poe_data::GameData::stat_suggestions_for_query()`.

The existing `get_suggestions("stat_texts")` stays for backwards compat.
The new command is richer — returns structured data with single/hybrid metadata.

### Step 4: App frontend — Enhanced picker

**File**: `app/src/components/` (profile editor / predicate editor)

When rendering stat search results:
- Single stats: show as today, create simple `StatValue` predicate
- Hybrid combos: show with visual indicator (e.g., "Urchin's: +# Life / +# Armour")
- Selecting a hybrid auto-creates a `Rule::All` containing two `StatValue` predicates

### No changes needed in poe-eval

The picker just creates the right predicate/rule structure. No new predicate type.

## What stays the same

- `StatValue` predicate works as today (matches individual stat lines)
- Existing profiles/rules are unaffected
- The individual stat option still exists alongside hybrid options
