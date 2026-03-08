# Phase 5: poe-eval Integration & Profile UI

> Wire poe-eval's evaluation capabilities into the app so users can configure
> profiles, see scores, and get actionable feedback on items.

## Prerequisites (done)

- [x] poe-eval: Predicate/Rule/Profile types (serializable)
- [x] poe-eval: Tier quality analysis (analyze_tiers)
- [x] poe-eval: Open affix analysis (analyze_affixes)
- [x] bridge.rs: EvaluatedItem with quality/tierKind/maxPrefixes/maxSuffixes
- [x] Frontend: Tier coloring driven by quality from poe-data (no domain logic in app)
- [x] Game data files committed for collaborators

## Steps

### Step 1: Verify display coloring end-to-end

Confirm the tier colors, roll bars, tier/rank badges, and affix summary all
render correctly with varied data. Use mock items that exercise every quality
level and edge case. Fix any CSS/visual issues.

**Test cases:**
- All 5 quality levels (best/great/good/mid/low) visible with distinct colors
- Rank badge shows "R1" not "T1" for crafted mods
- Roll bar colors match (high/mid/low thresholds)
- Open affix display works for magic items (1+1) not just rares (3+3)
- Unique mods show without tier badges
- Multi-line mods render correctly

### Step 2: Expose poe-eval scoring through the bridge

Add scoring to the pipeline: `score(item, profile, gd) -> ScoreResult`.
Send the score alongside the item data to the frontend.

- Add `score` and `matched_rules` fields to `EvaluatedItem` in bridge.rs
- Create a default "Generic" profile in poe-eval that scores basic desirables
  (life, resistances, movement speed) — proves the pipeline works
- Frontend displays the score (simple bar or number) on the overlay

### Step 3: Expose poe-eval predicate schema as Tauri command

The app needs to know what predicates poe-eval supports without hardcoding them.
Add a `get_predicate_schema` Tauri command that returns metadata:

```json
[
  { "type": "Rarity", "comparisons": ["==", ">=", "<="], "values": ["Normal", "Magic", "Rare", "Unique"] },
  { "type": "ItemClass", "input": "string", "suggestions": ["Body Armours", "Boots", ...] },
  { "type": "ItemLevel", "comparisons": [">=", "<=", "=="], "input": "number" },
  { "type": "ModTier", "comparisons": ["<=", ">="], "input": "number", "requires": "modName" },
  ...
]
```

This is the contract between poe-eval and the app. The app builds UI from this
schema — never hardcodes predicate types.

### Step 4: Profile store migration

Replace the placeholder `Profile` type in `store.ts` with poe-eval's actual
`Profile` JSON format. Split the current `Profile` into:

- `EvalProfile` — poe-eval's Profile serialized as JSON (rules, predicates, weights)
- `DisplayPrefs` — app-owned visual settings (tier colors, dim/highlight toggles)

Keep them linked by profile ID but stored separately.

### Step 5: Basic profile builder UI

Start with simple predicates that are easy to build UI for:

- Rarity filter (dropdown)
- Item class filter (dropdown/search, values from poe-data)
- Item level range (min/max number inputs)
- Has mod named (text search against known mod names from poe-data)

Each predicate row: [type dropdown] [comparison] [value input] [remove button]
Rules combine with AND/OR/NOT via a visual builder (start with AND-only).

### Step 6: Mod weight editor

Build the mod weight UI shown in the Phase 3 design (app-design.md).
Mod list comes from poe-data (not hardcoded). Search/filter by category.

This drives the scoring weights in the eval profile — heavier-weighted mods
contribute more to the item score.

### Step 7: Score display on overlay

Show the profile score on the overlay with visual treatment:
- Score bar (0-100) with color gradient
- Per-rule match indicators (which rules matched, which didn't)
- "Why" tooltip: expand to see which mods contributed how much

## Out of scope (future phases)

- Trade price estimation (needs poe.ninja API integration)
- Probabilistic crafting advice (needs poe-craft crate)
- Build-specific scoring from PoB (needs PoB CLI integration)
- Friend wishlists / RQE integration

## Key principle

The app reads what poe-eval supports and presents it. The app never
defines evaluation logic. If a new predicate type is added to poe-eval,
the schema command returns it and the UI adapts.
