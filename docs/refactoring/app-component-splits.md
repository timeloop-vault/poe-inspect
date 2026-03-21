# Plan: Split Large Frontend Components

**Priority:** 2
**Status:** TODO
**Effort:** ~8 hours (4h ItemOverlay + 4h ProfileSettings)
**Risk:** Low-medium — need to thread props and trade edit state through new components

## Problem

Two components exceed 1400 lines each, mixing display logic, trade editing, and state management.

## ItemOverlay.tsx (1454 lines)

### Target Structure

```
src/components/overlay/
  ItemOverlay.tsx       — Orchestrator, renders sub-components in order (~200 lines)
  ItemHeader.tsx        — Name, base type, rarity cycling, type scope selector
  ItemProperties.tsx    — Armor/evasion/ES/block with inline trade editing
  ItemSockets.tsx       — Socket display, link count, socket filter editing
  ItemMods.tsx          — Mod lines with tier badges, quality bars, influence badges
  PseudoStats.tsx       — Pseudo stat section with collapsible groups
  AttackProperties.tsx  — DPS, APS, crit, weapon filter editing
  InfluenceIcons.tsx    — Influence/status icon row
```

### Approach

- ItemOverlay becomes a thin wrapper that maps item data to sub-components
- Trade edit state (useTradeFilters) stays in ItemOverlay, passed as props
- Each sub-component receives: item data slice + optional trade edit callbacks
- CSS classes stay in overlay.css (no CSS module migration needed)

## ProfileSettings.tsx (1476 lines)

### Target Structure

```
src/components/settings/
  ProfileSettings.tsx    — Orchestrator, tab layout (~200 lines)
  ProfileList.tsx        — Profile list, CRUD buttons, role/color badges
  ProfileEditor.tsx      — Name, quality colors, display preferences
  ScoringRuleEditor.tsx  — Rule list with add/remove, wraps PredicateEditor
  ModWeightEditor.tsx    — Weight assignment table
  ProfileImportExport.tsx — JSON file dialog handlers
```

### Approach

- ProfileSettings becomes a tab container
- Profile state (selected profile, dirty flag) stays in ProfileSettings
- Each sub-component receives profile data + onChange callbacks
- PredicateEditor is already extracted — ScoringRuleEditor wraps it

## Steps (both)

1. Create component directories
2. Extract smallest/least-coupled component first (InfluenceIcons, ProfileList)
3. Work outward: properties, mods, header
4. Thread props — identify shared state that needs lifting
5. Run `cd app && npx tsc --noEmit`
6. Run `cd app && npx biome check --write --unsafe .`
7. Screenshot-verify overlay rendering hasn't changed
8. Screenshot-verify settings tabs work correctly
