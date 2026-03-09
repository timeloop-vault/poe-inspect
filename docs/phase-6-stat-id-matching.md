# Phase 6: Stat ID Matching

> Replace substring-based stat matching with proper stat ID matching.
> Mod weights and scoring rules use internal stat IDs (language-independent,
> unambiguous) instead of display text substrings.

## Problem

`HasStatText` does substring matching against display text. This breaks when:
- Template keys use `#` placeholders ("+# to maximum Life" doesn't match "+92 to maximum Life")
- Language changes (stat IDs are the same in all languages, display text is not)
- Ambiguous substrings match unintended stats

## Solution

Use stat IDs from the reverse index as the canonical matching key.
The reverse index already maps template keys → stat IDs (e.g.,
`"+# to maximum Life"` → `["base_maximum_life"]`). Items already have
resolved stat IDs on each `ResolvedStatLine`.

## Steps

### Step 1: Add `HasStatId` predicate to poe-eval

New `Predicate::HasStatId { stat_id: String }` variant.
Evaluation: any mod's stat line has `stat_ids` containing the target.
Add schema entry (category "Stats", text field with suggestions from `stat_ids`).

### Step 2: Expose template → stat_id mapping

Add `ReverseIndex::template_to_stat_ids()` → `HashMap<String, Vec<String>>`.
Add Tauri command `resolve_stat_template(template) -> Vec<String>` so the
frontend can look up stat IDs when the user picks a template from search.

### Step 3: Update ModWeight to store stat IDs

Change `ModWeight` from `{ text, level }` to `{ template, statIds, level }`.
- `template`: human-readable display text ("+# to maximum Life")
- `statIds`: internal stat IDs (["base_maximum_life"])
- `level`: weight level

Frontend calls `resolve_stat_template` when adding a new weight.
Store migration: re-resolve existing weights or reset to empty.

### Step 4: Sync uses HasStatId

`syncActiveProfile` generates `HasStatId` rules from mod weights instead
of `HasStatText`. Each mod weight's first stat ID becomes the predicate target.

### Step 5: Update default profile

Convert the 5 HasStatText rules in `generic.json` to HasStatId with proper
stat IDs. These are the built-in scoring rules for life, resistances, movement speed.

## Out of scope

- Removing `HasStatText` entirely (still useful for freeform text matching in scoring rules)
- Multi-stat predicate (matching multiple stat IDs together) — single stat ID is sufficient for now
- Value-aware matching (e.g., "+# to maximum Life >= 50") — that's `StatValue` predicate, already exists
