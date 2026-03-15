# Domain Knowledge Violations

Tracked violations found by the `domain-knowledge-reviewer` agent. Fix when touching these files.

## V1 — Frontend hardcodes influence → trade filter ID mapping

**File:** `app/src/components/ItemOverlay.tsx` — `influenceFilterId()`
**Severity:** Medium
**Issue:** Maps influence display names to trade API filter IDs ("Shaper" → "shaper_item", "Elder" → "elder_item", etc.) in TypeScript. This is trade API knowledge that belongs in the backend.
**Fix:** Have `TradeEditSchema` resolve these mappings so the frontend matches by filter text against the schema, not by hardcoded IDs.

## V2 — Frontend duplicates property → trade filter ID aliases

**File:** `app/src/components/ItemOverlay.tsx` — `PROPERTY_ALIASES`
**Severity:** Medium
**Issue:** Maps property names to trade API filter IDs ("Evasion Rating" → "ev", "Chance to Block" → "block") in TypeScript. Same concept already exists in Rust at `poe-trade/src/filter_schema.rs:PROPERTY_ALIASES`.
**Fix:** Backend should annotate which properties map to which filters in the schema, eliminating frontend trade API knowledge.

## V3 — Trade API convention value lacks comment

**File:** `crates/poe-trade/src/query.rs:469`
**Severity:** Low
**Issue:** `"nonunique"` rarity value used without the standard trade API convention comment. Same value is correctly documented in `filter_schema.rs:561`.
**Fix:** Add `// Trade API convention, not in GGPK (verified 2026-03-15)` comment.
