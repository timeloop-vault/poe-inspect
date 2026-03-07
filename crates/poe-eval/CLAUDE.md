# poe-eval

Evaluates parsed items against user-defined filter rules and profiles. The "brain" that answers "is this item good?".

## Scope

- Define rule/profile format for item evaluation
- Score items against profiles (tier quality, stat weights, build relevance)
- Crafting potential analysis: open affixes, deterministic craft suggestions
- Trade value estimation integration point (actual API calls are in `app`)
- Friend/party wishlist matching

## Does NOT own

- Item parsing — that's `poe-item`
- Game data — that's `poe-data`
- UI/overlay — that's `app`
- Network requests — this crate is pure evaluation logic

## Key Design Decisions

- **Profiles are data, not code**: Evaluation rules are user-configurable (JSON/TOML), not hardcoded. Users define what stats matter and their weights.
- **Layered evaluation**: Tier coloring (instant) → profile scoring (instant) → crafting potential (instant) → trade value (needs network, provided externally) → meta awareness (needs network).
- **Shareable profiles**: Rule format designed for import/export so the community can share configurations.

## Evaluation Layers (from HYPOTHESIS.md)

| Layer | Input | Output | Speed |
|-------|-------|--------|-------|
| Tier coloring | ParsedItem | Per-affix tier + color | Instant |
| Profile matching | ParsedItem + Profile | Score + reasoning | Instant |
| Crafting potential | ParsedItem + GameData | Open affixes, craft suggestions | Instant |
| Trade valuation | ParsedItem + external prices | Estimated value | External |
| Meta awareness | ParsedItem + external builds | Build usage stats | External |
| Friend wishlist | ParsedItem + wishlists | Matches | Instant |

## Dependencies

- `poe-item` — for `ParsedItem` types
- `poe-data` — for mod/craft lookups during evaluation

## Plan

1. Define `Profile` and `Rule` types (serializable)
2. Tier coloring evaluator (simplest — just map mod tier to color)
3. Weighted stat scoring against a profile
4. Open affix detection + basic craft suggestion
5. Profile import/export (JSON)
6. Tests with real parsed items + sample profiles
