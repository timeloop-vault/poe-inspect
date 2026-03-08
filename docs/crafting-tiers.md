# Crafting Knowledge Tiers

How crafting intelligence fits into the crate architecture.

## The Problem

Evaluating an item's worth isn't just about what it has — it's about what it *could become*.
A 5-mod rare with one open suffix and no life roll is worth more than the same item corrupted,
because you can bench-craft life on it. A high-ilvl base with 3 great prefixes and an open
suffix is worth meta-crafting. But "meta-craft → exalt → scour → repeat" is complex strategy
with probability and expected cost — that's a different scope than "this item has an open slot".

## Three Tiers

### Tier 1: Open Affix Detection (poe-eval)

**Status:** Mostly done (`open_mod_count` exists).

What it answers:
- "This item has 1 open prefix, 2 open suffixes"
- "This item is full — no room for crafts"
- "This item is corrupted — can't be modified"

Pure evaluation logic. Uses `GameData::max_prefixes()`/`max_suffixes()` from poe-data.
No crafting knowledge needed.

### Tier 2: Deterministic Craft Suggestions (poe-eval + poe-data)

**Status:** Future. Requires `CraftingBenchOptions.datc64` extraction.

What it answers:
- "You can bench-craft +70 life in the open prefix"
- "Available bench crafts for this base: [list]"
- "This item has an open suffix — craft resist before using"

**This is a must-have for poe-eval scoring.** Deterministic crafts (bench crafts) have
guaranteed outcomes. An item with an open prefix that can receive a life bench craft is
strictly better than the same item evaluated without it. poe-eval should factor deterministic
crafts into its score — they represent the item's floor, not a gamble.

Data needed in poe-data:
- `CraftingBenchOptions.datc64` — recipes, costs, which mods they grant
- Filtering: base type + item class + ilvl → eligible bench crafts

Logic in poe-eval:
- Match open slots against available bench crafts
- Include best deterministic outcome in scoring

### Tier 3: Probabilistic Crafting Strategy (poe-craft)

**Status:** Future crate. Not planned for initial release.

What it answers:
- "Your best path: meta-craft → exalt → meta-craft → scour, expected cost 15 div"
- "Fossil crafting with Pristine + Scorched has 23% chance of hitting T1 life + T2 fire res"
- "This item is worth slamming (68% chance of useful mod)"
- "WARNING: this craft has a 40% chance of bricking the item"

**This is extra, not required for core evaluation.** Probabilistic outcomes are useful
information ("this item could be better but may brick") but shouldn't be mixed into the
base score. They belong in a separate "crafting potential" section of the overlay.

This is CraftOfExile territory — mod pool filtering, weighting, probability math,
multi-step strategy trees, expected cost calculations.

Crate: `poe-craft`
- Depends on: `poe-data` (mod pools, weightings, tags, generation types)
- Consumed by: `poe-eval` (optional crafting potential score), `app` (craft advisor UI)

Data needed in poe-data (beyond tier 2):
- Full mod pool data: `Mods.datc64` generation types, spawn weights, mod groups
- Currency effects: what each orb/essence/fossil does (partially in GGPK, partially hardcoded)
- Meta-craft modifiers: "Prefixes Cannot Be Changed" etc.

## Scoring Integration

```
poe-eval score = base_score + deterministic_craft_bonus
                              ↑ tier 2 (must-have)

poe-craft potential = { probability, expected_cost, risk_level }
                       ↑ tier 3 (extra, shown separately)
```

Deterministic crafts raise the floor — they're guaranteed improvements.
Probabilistic crafts show the ceiling — they're gambles with expected value.

The overlay should distinguish these clearly:
- **Score** includes deterministic potential (what you CAN do for sure)
- **Craft advisor** (separate panel) shows probabilistic paths (what you COULD do)
