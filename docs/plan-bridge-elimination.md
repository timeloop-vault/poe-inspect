# Plan: Eliminate bridge.rs — Domain Logic Cleanup

## Problem

`app/src-tauri/src/bridge.rs` (~400 lines) contains parsing, classification, and domain
logic that belongs in poe-item and poe-eval. `app/src/types.ts` has manually defined types
that should be generated from poe-eval via ts-rs.

The app should be a thin orchestrator: call poe-* functions, serialize results, render.

## Pipeline (target)

```
clipboard text
    → poe_item::parse() → RawItem
    → poe_item::resolve(raw, gd) → ResolvedItem (rich, pre-classified)
    → poe_eval::evaluate_item(item, gd, profile, watching) → EvaluatedItem (display-ready)
    → serde_json::to_value() → emit to frontend
```

## Phase 1: poe-item — Enrich ResolvedItem

### Changes to `crates/poe-item/src/types.rs`

**New types:**

```rust
pub struct ItemProperty { name, value, augmented }   // replaces bridge::ItemProperty
pub enum ModDisplayType { Prefix, Suffix, Implicit, Enchant, Unique, Crafted }  // replaces BridgeModType
pub enum TierDisplayKind { Tier, Rank }              // replaces BridgeTierKind
```

All with conditional `serde` + `ts` derives (same pattern as `Rarity`).

**New methods:**

```rust
impl ResolvedMod {
    pub fn display_type(&self) -> ModDisplayType;    // maps ModSlot + ModSource → flat type
}
impl ModTierKind {
    pub fn display_kind(&self) -> TierDisplayKind;   // Tier(_) → Tier, Rank(_) → Rank
    pub fn number(&self) -> u32;                     // extract the tier/rank number
}
```

**ResolvedMod changes:**

```rust
pub struct ResolvedMod {
    pub header: ModHeader,
    pub stat_lines: Vec<ResolvedStatLine>,
    pub is_fractured: bool,   // NEW — detected from raw_text "(fractured)" suffix
}
```

**ResolvedItem changes:**

```rust
pub struct ResolvedItem {
    pub header: ResolvedHeader,
    pub item_level: Option<u32>,
    pub requirements: Vec<Requirement>,
    pub sockets: Option<String>,
    pub properties: Vec<ItemProperty>,       // CHANGED: was Vec<Vec<String>>
    pub implicits: Vec<ResolvedMod>,         // NEW: split from flat mods
    pub explicits: Vec<ResolvedMod>,         // NEW: split from flat mods
    pub enchants: Vec<ResolvedMod>,          // NEW: empty for now
    pub influences: Vec<InfluenceKind>,
    pub statuses: Vec<StatusKind>,
    pub is_corrupted: bool,                  // NEW: convenience
    pub is_fractured: bool,                  // NEW: convenience
    pub flavor_text: Option<String>,         // NEW: classified from generic sections
    pub unclassified_sections: Vec<Vec<String>>,  // RENAMED: leftovers
    // Removed: mods, monster_level, talisman_tier, experience (keep if needed)
}

impl ResolvedItem {
    /// All mods in order (for poe-eval iteration).
    pub fn all_mods(&self) -> impl Iterator<Item = &ResolvedMod>;
}
```

**Add serde/ts derives** to all public types: `InfluenceKind`, `StatusKind`, `Requirement`,
`ValueRange`, `ResolvedStatLine`, `ResolvedMod`, `ResolvedHeader`, `ResolvedItem`.

### Changes to `crates/poe-item/src/resolver.rs`

Move logic from bridge.rs:
- Parse properties from generic sections (`: ` splitting, `(augmented)` detection)
- Classify flavor text (last generic section with no colon lines)
- Split mods into implicits/explicits during collection
- Detect fractured per-mod from stat line text
- Set is_corrupted/is_fractured convenience bools

### Update poe-eval callers

All `item.mods` references → `item.all_mods()`:
- `evaluate.rs` — count_mods_in_slot, find_matching_stats, eval_predicate
- `tier.rs` — analyze_tiers
- `affix.rs` — count_slot

### Tests

- Update poe-item resolver tests for new ResolvedItem shape
- Add property parsing tests
- Add flavor text classification tests
- Verify poe-eval tests still pass with all_mods()

## Phase 2: poe-eval — ts-rs Exports + evaluate_item()

### Cargo.toml

```toml
[features]
ts = ["dep:ts-rs", "poe-item/ts", "poe-data/ts"]

[dependencies]
ts-rs = { version = "12", optional = true }
```

`serde` stays as a hard dependency (profiles are JSON).

### Add ts-rs derives to all public types

Files: `predicate.rs`, `rule.rs`, `profile.rs`, `schema.rs`, `tier.rs`, `affix.rs`

Types: `Predicate`, `Cmp`, `Rule`, `Profile`, `ScoringRule`, `ScoreResult`, `MatchedRule`,
`UnmatchedRule`, `PredicateSchema`, `PredicateField`, `FieldKind`, `EnumOption`,
`ModSlotKind`, `RarityValue`, `InfluenceValue`, `StatusValue`, `ItemTierSummary`,
`ModTierInfo`, `QualityCounts`, `AffixSummary`, `SlotSummary`, `Modifiability`

### New: evaluate_item() function

Single entry point replacing bridge::build_evaluated_item(). Lives in new module
`crates/poe-eval/src/item_result.rs`.

```rust
pub fn evaluate_item(
    item: &ResolvedItem,
    gd: &GameData,
    profile: Option<&Profile>,
    watching: &[(String, String, Profile)],  // (name, color, profile)
) -> EvaluatedItem { ... }
```

Returns `EvaluatedItem` with `#[derive(Serialize, TS)]` — the exact type the frontend
consumes. Contains: item display data + tier info + affix counts + scores.

Sub-types: `DisplayMod` (replaces bridge::Modifier), `ScoreInfo`, `RuleMatch`,
`WatchingScoreInfo`.

### Backward compatibility

Adding ts-rs derives does NOT change serde format. Existing saved profiles.json
will deserialize without changes.

## Phase 3: app — Delete bridge.rs

### Delete `app/src-tauri/src/bridge.rs`

Remove `mod bridge;` from lib.rs.

### Simplify lib.rs handler

```rust
let result = poe_eval::evaluate_item(&resolved, &gd, profile.as_ref(), &watching);
app.emit("item-captured", serde_json::to_value(&result).unwrap());
```

### Remove ts-rs direct dependency

App gets ts-rs transitively through poe-item/poe-eval features.

### Regenerate TypeScript types

`cargo test` in app/src-tauri generates all .ts files. New files from poe-eval:
Profile.ts, ScoringRule.ts, Rule.ts, Predicate.ts, Cmp.ts, PredicateSchema.ts,
PredicateField.ts, FieldKind.ts, EnumOption.ts, etc.

### Update types.ts

Becomes pure re-exports from `./generated/`. Only manual code: type guards
(`isCompoundRule`, `isPredRule`).

### Frontend components

Zero changes expected — type aliases in types.ts maintain backward compatibility.

**Risk:** ts-rs generated `Rule` type shape may differ from manual definition.
The manual type uses `Record<string, unknown>` for Pred variants. Generated type
will be more specific. Type guards bridge this. Test after generation.

## Implementation Order

1. ~~Phase 1: poe-item (breaking change to ResolvedItem → must update poe-eval callers)~~ ✓ DONE
2. ~~Phase 2: poe-eval (additive — new function, new derives)~~ ✓ DONE
3. ~~Phase 3: app (deletion — remove bridge.rs, thin lib.rs)~~ ✓ DONE

All three phases completed in a single session. bridge.rs deleted, types.ts is 100% generated re-exports.

## What This Eliminates

| bridge.rs function | Lines | Moved to |
|---|---|---|
| extract_properties() | 357-377 | poe-item resolver |
| extract_flavor_text() | 408-416 | poe-item resolver |
| build_modifier() | 282-353 | poe-eval evaluate_item() using poe-item methods |
| build_score_info() | 249-280 | poe-eval evaluate_item() |
| build_evaluated_item() | 140-247 | poe-eval evaluate_item() |
| BridgeModType enum | 385-396 | poe-item ModDisplayType |
| BridgeTierKind enum | 399-405 | poe-item TierDisplayKind |
| EvaluatedItem struct | 19-53 | poe-eval EvaluatedItem |
| Modifier struct | 106-137 | poe-eval DisplayMod |
| ScoreInfo struct | 65-82 | poe-eval ScoreInfo |
| All other bridge structs | 56-104 | poe-eval or poe-item |

**Result:** bridge.rs deleted entirely. ~400 lines of domain leakage → 0.
