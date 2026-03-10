# poe-eval

Evaluates parsed items against user-defined filter rules and profiles. The "brain" that answers "is this item good?".

## Status

**Foundation done** — Predicate/Rule/Evaluate + Scoring Profiles + Tier Analysis + evaluate_item(). 32 fixture tests. Supports DivinationCard rarity, Unidentified status.

## Scope

- Define rule/profile format for item evaluation
- Score items against profiles (tier quality, stat weights, build relevance)
- Crafting potential: open affixes, deterministic craft suggestions (see `docs/crafting-tiers.md`)
- Probabilistic crafting strategy is OUT OF SCOPE — that's future `poe-craft` crate
- Trade value estimation integration point (actual API calls are in `app`)
- Friend/party wishlist matching

## Does NOT own

- Item parsing — that's `poe-item`
- Game data / PoE domain knowledge — that's `poe-data`
- UI/overlay — that's `app`
- Network requests — this crate is pure evaluation logic

## Key Design Decisions

- **Zero PoE domain knowledge**: All game-specific lookups go through `GameData`. poe-eval is pure logic — predicates, rules, and matching.
- **Profiles are data, not code**: Evaluation rules are user-configurable (JSON/TOML), not hardcoded.
- **Serializable rules**: `Predicate` and `Rule` types derive `Serialize`/`Deserialize` for import/export.
- **Own enum types**: poe-eval defines its own `RarityValue`, `InfluenceValue`, `StatusValue`, `ModSlotKind` enums (serializable) rather than depending on poe-item's non-serde types.

## Architecture

```
src/
  lib.rs          — public API
  affix.rs        — open affix analysis (used/max/open slots, modifiability, crafted detection)
  evaluate.rs     — evaluate(item, rule, game_data) -> bool, score(item, profile, game_data)
  item_result.rs  — evaluate_item() + display types (EvaluatedItem, Modifier, ScoreInfo, etc.)
  predicate.rs    — atomic conditions (Rarity, ItemClass, ModCount, StatValue, etc.)
  profile.rs      — scoring profiles (weighted rule sets)
  rule.rs         — combinators (All, Any, Not) over predicates
  schema.rs       — predicate schema for dynamic UI building
  tier.rs         — tier quality analysis (mod tier → Best/Great/Good/Mid/Low)
tests/
  evaluate_fixtures.rs — tests against real item fixtures (32 tests)
```

### Predicate types

| Predicate | Tests against |
|-----------|--------------|
| `Rarity` | Rarity comparison (==, >=, etc.) |
| `ItemClass` | Item class string match |
| `BaseType` | Base type exact match |
| `BaseTypeContains` | Base type substring match |
| `ItemLevel` | Item level comparison |
| `ModCount` | Count of prefix/suffix/implicit mods |
| `OpenMods` | Available mod slots (uses GameData for max) |
| `HasModNamed` | Whether any mod has a specific name |
| `HasStatText` | Whether any stat line contains text |
| `HasStatId` | Whether any stat has a resolved stat ID (language-independent) |
| `ModTier` | Mod tier comparison by name |
| `StatValue` | Rolled value of a matching stat |
| `RollPercent` | How close a roll is to max (0-100%) |
| `HasInfluence` | Specific influence check |
| `HasStatus` | Specific status check (Corrupted, etc.) |
| `InfluenceCount` | Total number of influences |

### Rule combinators

- `Rule::Pred(p)` — single predicate
- `Rule::All { rules }` — AND
- `Rule::Any { rules }` — OR
- `Rule::Not { rule }` — negation

## Evaluation Layers (from HYPOTHESIS.md)

| Layer | Input | Output | Speed |
|-------|-------|--------|-------|
| Tier coloring | ResolvedItem | Per-affix tier + color | Instant |
| Profile matching | ResolvedItem + Profile | Score + reasoning | Instant |
| Crafting potential | ResolvedItem + GameData | Open affixes, craft suggestions | Instant |
| Trade valuation | ResolvedItem + external prices | Estimated value | External |
| Meta awareness | ResolvedItem + external builds | Build usage stats | External |
| Friend wishlist | ResolvedItem + wishlists | Matches | Instant |

## Dependencies

- `poe-item` — for `ResolvedItem` types
- `poe-data` — for mod/craft lookups during evaluation
- `serde` — serializable rules

## Plan

1. ~~Define `Predicate` and `Rule` types (serializable)~~ ✓
2. ~~Evaluate engine: `evaluate(item, rule, game_data) -> bool`~~ ✓
3. ~~Test with real parsed items~~ ✓
4. ~~Scoring profiles: `score(item, profile, game_data) -> ScoreResult`~~ ✓
5. ~~Profile JSON serialization round-trip~~ ✓
6. ~~Tier quality analysis: `analyze_tiers(item) -> ItemTierSummary`~~ ✓
7. ~~Open affix detection: `analyze_affixes(item, game_data) -> AffixSummary`~~ ✓
8. ~~Display-ready evaluation: `evaluate_item(item, gd, profile, watching) -> EvaluatedItem`~~ ✓
9. ~~ts-rs exports for all public types (feature-gated `ts` feature)~~ ✓
10. Deterministic craft suggestions (needs `CraftingBenchOptions` in poe-data — see `docs/crafting-tiers.md`)
