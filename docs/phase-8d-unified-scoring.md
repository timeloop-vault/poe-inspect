# Phase 8D: Unified Scoring — Design Exploration

## Problem

Two tabs ("Scoring Rules" + "Mod Weights") both contribute to the same item score.
Users must look in two places to understand why an item scored what it did.
They're conceptually identical: "if item has X → award Y points."

## Current State

```
┌─ Scoring Rules tab ──────────────────────────────────────┐
│                                                          │
│  ┌ Has open prefix                    pts [10] ▼ × ┐    │
│  └─────────────────────────────────────────────────┘    │
│  ┌ Item level >= 84                   pts  [5] ▼ × ┐    │
│  └─────────────────────────────────────────────────┘    │
│  ┌ Magic, Life > 100, Prefix = 0     pts[100] ▲ × ┐    │
│  │  [Single] [Group]                               │    │
│  │  ▼ Match ALL of: 2                              │    │
│  │  │ Rarity  >  Magic                             │    │
│  │  │ AND                                          │    │
│  │  │ ...                                          │    │
│  └─────────────────────────────────────────────────┘    │
│                                                          │
│  Complex predicates, numeric weights, compound groups    │
└──────────────────────────────────────────────────────────┘

┌─ Mod Weights tab ────────────────────────────────────────┐
│                                                          │
│  [Search stats to add...                            ]    │
│                                                          │
│  +# to maximum Life           [████░] High               │
│  +#% Fire Resistance          [███░░] Med                │
│  +# to Spell Damage           [██░░░] Low                │
│  +#% Movement Speed           [█████] Crit               │
│                                                          │
│  Simple stat matching, bar-based weights                 │
└──────────────────────────────────────────────────────────┘
```

**Confusion:** Both feed into the same `evaluate_item()` score.
A user editing "Scripter" profile must jump between tabs to see the full picture.

---

## Option A: Unified List (mixed)

One list. Stat entries use the bar widget. Complex rules use the card widget.
Both sorted together by weight (highest first) or manual order.

```
┌─ Scoring ────────────────────────────────────────────────┐
│                                                          │
│  [Search stats to add...                    ] [+ Rule ▾] │
│                                                          │
│  ── Stats ───────────────────────────────────────────    │
│                                                          │
│  +# to maximum Life           [████░] High          ×    │
│  +#% Fire Resistance          [███░░] Med           ×    │
│  +# to Spell Damage           [██░░░] Low           ×    │
│  +#% Movement Speed           [█████] Crit          ×    │
│                                                          │
│  ── Rules ───────────────────────────────────────────    │
│                                                          │
│  ┌ Has open prefix                    pts [10] ▼ × ┐    │
│  └─────────────────────────────────────────────────┘    │
│  ┌ Item level >= 84                   pts  [5] ▼ × ┐    │
│  └─────────────────────────────────────────────────┘    │
│  ┌ Magic, Life > 100, Prefix = 0     pts[100] ▲ × ┐    │
│  │  ...                                            │    │
│  └─────────────────────────────────────────────────┘    │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

**Pro:** Everything in one place. Stats are quick to add.
**Con:** Two visual styles in one list. "Stats" section vs "Rules" section
       is really just two tabs in a trenchcoat.

---

## Option B: Single list, stat rules get bar widget

No sections. Every entry is a ScoringRule. But stat-only rules
(HasStatId predicate) get the compact bar UI automatically.
Complex rules get the card UI. They're interleaved by creation order.

```
┌─ Scoring ────────────────────────────────────────────────┐
│                                                          │
│  [Search stats to add...              ] [+ Rule] [+ Grp] │
│                                                          │
│  +# to maximum Life           [████░] High          ×    │
│  ┌ Has open prefix                    pts [10] ▼ × ┐    │
│  └─────────────────────────────────────────────────┘    │
│  +#% Fire Resistance          [███░░] Med           ×    │
│  ┌ Item level >= 84                   pts  [5] ▼ × ┐    │
│  └─────────────────────────────────────────────────┘    │
│  +# to Spell Damage           [██░░░] Low           ×    │
│  +#% Movement Speed           [█████] Crit          ×    │
│  ┌ Magic, Life > 100, Prefix = 0     pts[100] ▲ × ┐    │
│  │  ▼ Match ALL of: 2                              │    │
│  │  ...                                            │    │
│  └─────────────────────────────────────────────────┘    │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

**Pro:** Truly unified. No mental model split.
**Con:** Visual rhythm is messy — two very different row heights interleaved.
       What if user wants to see "just my stats" or "just my rules"?

---

## Option C: All rules use the bar widget

The bar widget IS the weight selector for everything. No numeric input.
Even complex rules use [░░░░░] Low/Med/High/Crit to set weight.

```
┌─ Scoring ────────────────────────────────────────────────┐
│                                                          │
│  [Search stats to add...              ] [+ Rule] [+ Grp] │
│                                                          │
│  +# to maximum Life           [████░] High          ×    │
│  +#% Fire Resistance          [███░░] Med           ×    │
│  +# to Spell Damage           [██░░░] Low           ×    │
│  +#% Movement Speed           [█████] Crit          ×    │
│  Has open prefix              [███░░] Med      ▼    ×    │
│  Item level >= 84             [██░░░] Low      ▼    ×    │
│  Magic, Life>100, Pfx=0      [█████] Crit     ▲    ×    │
│  │  ▼ Match ALL of: 2                              │    │
│  │  │ Rarity  >  Magic                             │    │
│  │  │  AND                                         │    │
│  │  │ ▼ Match ALL of: 2                            │    │
│  │  │ ...                                          │    │
│  │                                                 │    │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

**Pro:** Completely unified look. Every row has the same weight widget.
       4 weight levels (Low=5, Med=15, High=50, Crit=100) cover 95% of cases.
       One mental model. Simple.
**Con:** Loses fine-grained numeric weights (10, 25, 75, etc).
       But do users actually need that granularity? PoE trade doesn't offer it.

---

## Option D: Bar widget with optional numeric override

Default is bar (4 levels). Click the label to type a custom number.

```
  +# to maximum Life           [████░] High          ×
  Item level >= 84             [██░░░] Low           ×
  Magic, Life>100, Pfx=0      [█████] 100      ▲    ×
                                       ^^^
                                  click to edit number,
                                  or click bars for preset
```

**Pro:** Best of both worlds. Simple by default, precise when needed.
**Con:** Slightly more complex interaction (click label to switch mode).

---

## Decision: Option D — IMPLEMENTED

**Option D** (bar + optional numeric override) was chosen.

- Bar levels: Low=5, Med=15, High=50, Crit=100
- Click the level label to toggle to numeric input (0-999)
- Non-standard weights auto-show in numeric mode
- Stat-search autocomplete shows stat text (kept from Mod Weights tab)
- Drag-to-reorder enabled (HTML5 DnD, manual order)

## Data Migration — IMPLEMENTED

On profile load, `mergeModWeightsIntoScoring()` converts existing `modWeights`
into `ScoringRule`s with `HasStatId` predicates:
```
ModWeight { template: "+# to Life", statIds: [...], level: "high" }
  → ScoringRule { label: "+# to Life", weight: 50, rule: Pred(HasStatId) }
```

The `modWeights` array on StoredProfile becomes empty/deprecated.
All scoring lives in `evalProfile.scoring`. The separate "Mod Weights" tab
was removed — everything is in the unified "Scoring" tab.
