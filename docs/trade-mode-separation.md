# Trade Mode Separation

Split the overlay into distinct inspect and trade modes with clear responsibilities.

## Problem

The current inspect overlay (Ctrl+I) bundles item evaluation with trade buttons (Edit Search, Price Check, Open Trade). This conflates two workflows: quickly checking an item's quality vs. pricing it on the trade site.

## Design

Four overlay modes, each with its own hotkey:

| Hotkey | Mode | Behavior |
|--------|------|----------|
| `Ctrl+I` | `inspect` | Item data + eval. No trade panel. |
| `Ctrl+Shift+I` | `compact` | Compact pill with quick summary. Unchanged. |
| `Ctrl+T` | `trade` | Inspect + auto price check. Trade panel with Edit/Price Check/Open Trade buttons for follow-up. |
| `Ctrl+Shift+T` | `tradeEdit` | Inspect + trade edit mode (inline filter editing before searching). |

### Inspect mode (`inspect`)

Pure item evaluation. Shows header, properties, mods with tier analysis, pseudo stats, scoring. No trade panel rendered at all.

Previously called `full` — renamed to `inspect` for clarity.

### Trade mode (`trade`)

Auto-fires a price check as soon as the item is parsed. The trade panel is visible with all buttons (Edit Search, Price Check, Open Trade) so the user can refine and re-search.

Rate limiting: if the user rapidly Ctrl+T's multiple items, the auto-search respects cooldown. The second search queues and fires when cooldown expires. If a third item arrives while queued, only the latest item is searched (supersedes the pending one).

```
Ctrl+T item1 → search fires immediately
Ctrl+T item2 (1s later, cooldown active) → queued
  → cooldown expires → search fires for item2
Ctrl+T item3 (while item2 queued) → item3 replaces item2 in queue
  → cooldown expires → search fires for item3
```

### Trade Edit mode (`tradeEdit`)

Auto-enters edit mode on the trade filters (checkboxes, min/max values on mods, socket filters, etc.). User customizes filters, then manually clicks Price Check or Open Trade.

This is the current `trade` mode behavior, moved to a new hotkey.

## Implementation

### Backend (Rust)

- `HotkeySettings` struct: add `trade_edit_inspect` field
- `hotkey.rs`: register 4th shortcut, emit `inspect-mode` with `"tradeEdit"`
- `inspect.rs`: pass `"tradeEdit"` mode through to frontend

### Frontend

- **App.tsx**: handle 4 modes — `inspect` hides TradePanel, `trade` passes `autoSearch` prop, `tradeEdit` triggers edit mode
- **TradePanel.tsx**: accept `autoSearch` prop, auto-fire `priceCheck` on new item, queue if cooldown active
- **useTradeFilters.ts**: `pendingAutoEdit` triggers on `tradeEdit` instead of `trade`
- **store.ts**: add `tradeEditInspect` to `HotkeySettings` and defaults (`"Ctrl+Shift+T"`)
- **HotkeySettings.tsx**: add new hotkey row
