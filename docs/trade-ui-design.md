# Trade UI/UX Design — Phase 5

> Design for price check integration in the overlay. Backend (Phase 4) is done.
> Reference: awakened-poe-trade (APT) for UX patterns that PoE players expect.

## Design Principles

1. **Non-intrusive**: Price check is opt-in, not automatic. Don't slow down the inspect flow.
2. **PoE-native feel**: Match the existing overlay styling (dark brown, warm tan, rarity colors).
3. **Minimal clicks**: One click to price check, one click to open on trade site.
4. **Progressive disclosure**: Show summary first, expand for details.

---

## User Flow

```
Ctrl+I → item evaluation overlay (existing)
    │
    ├── "Price Check" button at bottom of overlay
    │       │
    │       ▼
    │   Loading spinner (1-3 seconds)
    │       │
    │       ▼
    │   Price results section:
    │     • Total listings count
    │     • Price range (cheapest 5-10)
    │     • "Open on trade" link
    │
    └── Item overlay dismisses normally (Escape / click backdrop)
```

**No auto-price-check**: Unlike APT which auto-searches uniques/maps, we always require
a button click. Reasons: rate limit budget, not every inspect needs pricing, and the
user controls when HTTP traffic happens.

---

## Overlay Layout (Updated)

```
┌─────────────────────────────────────┐
│ ══════ Item Header (sprites) ═══════│
│─────────────────────────────────────│
│ Properties (Armour, ES, etc.)       │
│─────────────────────────────────────│
│ Requirements                        │
│─────────────────────────────────────│
│ Sockets · Item Level                │
│─────────────────────────────────────│
│ Enchants (blue)                     │
│─────────────────────────────────────│
│ Implicits                           │
│─────────────────────────────────────│
│ [T1 P] +139 to maximum Life    98% │  ← existing mod lines
│ [T6 P] +113 to Armour          55% │
│ [T3 P] +35 to Armour, +24 Life 40% │
│ [T6 S] Regen 41.1 Life/s       28% │
│ [T2 S] +42% Cold Resistance    0%  │
│ [T3 S] +37% Fire Resistance    25% │
│─────────────────────────────────────│
│ 0 open prefixes · 0 open suffixes  │
│ Prefixes: 3/3 · Suffixes: 3/3      │
│─────────────────────────────────────│
│ Searing Exarch                      │
│─────────────────────────────────────│
│ Score: 72%  ████████░░              │
│  + Has T1 life        - Missing res │
│─────────────────────────────────────│
│ [🔎 Price Check]  [↗ Open Trade]   │  ← NEW: action buttons
│─────────────────────────────────────│
│                                     │
│  ↓ (price results appear here)  ↓   │
│                                     │
│  42 listings found                  │
│  ┌─────────────────────────────┐    │
│  │  50 chaos                   │    │
│  │  55 chaos                   │    │
│  │  60 chaos                   │    │
│  │  1 divine                   │    │
│  │  1.2 divine                 │    │
│  └─────────────────────────────┘    │
│  Searched 8/10 stats (2 unmapped)   │
│                                     │
└─────────────────────────────────────┘
```

---

## Components

### 1. Trade Action Bar (`TradeActionBar`)

Appears below the score section (or below affixes if no score).

```
┌──────────────────────────────────────┐
│  [🔎 Price Check]   [↗ Open Trade]  │
└──────────────────────────────────────┘
```

- **Price Check** button: triggers `price_check()` command, shows results inline
- **Open Trade** button: triggers `trade_search_url()`, opens external browser
- Both disabled with tooltip if trade index not loaded ("Refresh trade stats in Settings")
- Both show loading spinner when active

### 2. Price Results (`TradeResults`)

Appears below the action bar after a successful price check.

```
┌──────────────────────────────────────┐
│ 42 listings                          │
│──────────────────────────────────────│
│  50 chaos                            │
│  55 chaos                            │
│  60 chaos                            │
│  1 divine                            │
│  1.2 divine                          │
│──────────────────────────────────────│
│ 8/10 stats searched (2 unmapped)     │
└──────────────────────────────────────┘
```

**Columns**: Just price amount + currency. No seller, no listing age — keep it minimal.
We're showing "what range should I price this at", not a full trade browser.

**States**:
- **Idle**: Nothing shown (before button click)
- **Loading**: Spinner + "Searching..."
- **Results**: Price list + total count + diagnostics
- **Empty**: "No listings found" message
- **Error**: Error message + retry button
- **Rate limited**: "Rate limited — retry in Xs" with countdown

### 3. Trade Settings (in Settings window)

New section in General Settings or dedicated Trade tab:

```
┌──────────────────────────────────────┐
│ Trade                                │
│──────────────────────────────────────│
│ League:  [Mirage          ▼]        │
│ Relaxation:  [85%  ───●────]        │
│ Online only: [✓]                     │
│                                      │
│ [↻ Refresh Trade Stats]             │
│ Last refreshed: 2 hours ago          │
│ Stats mapped: 10,160 / 11,624        │
└──────────────────────────────────────┘
```

- **League**: Required. Dropdown or text input. Stored in settings.json.
- **Relaxation**: Slider 50%-100%. Default 85%. Tooltip explains "search for items with at least X% of your roll values".
- **Online only**: Checkbox. Default on.
- **Refresh button**: Calls `refresh_trade_stats()`. Shows spinner during fetch.
- **Stats info**: Diagnostic — how many stats are mapped (for debugging coverage gaps).

---

## Data Flow

```
User clicks "Price Check"
    │
    ▼
Frontend reads: itemText (from current payload), config (from settings store)
    │
    ▼
invoke('price_check', { itemText, config: { league, valueRelaxation, onlineOnly, ... } })
    │
    ▼
Rust: parse → resolve → build_query → search → fetch → return PriceCheckResult
    │
    ▼
Frontend receives: { searchId, total, prices: [{amount, currency}], tradeUrl }
    │
    ▼
Render TradeResults component
```

**Item text**: The overlay already has the raw clipboard text (from `item-captured` event
or stored in App.tsx state). Pass it through to the trade command.

**Config**: Loaded from `settings.json` store. `TradeQueryConfig` type is already
ts-rs exported.

---

## Questions to Discuss

### Q1: Per-stat filter toggles?

APT lets users check/uncheck individual stats before searching. This is powerful but
adds significant UI complexity. Options:

- **A) No toggles (v1)**: Search with all stats. Simple. User can tweak on trade site.
- **B) Checkboxes on mod lines**: Add a checkbox to each `ModLine`. Checked = included
  in search. Requires passing selected stats to `build_query`.
- **C) Separate filter panel**: Below the mods, show a compact list of stats with
  checkboxes. More like APT's approach.

**Recommendation**: Start with **A** for Phase 5. Add **B** in Phase 6 — it's the
natural evolution since our mod lines already have badges and roll bars.

### Q2: Where does league config live?

- **Settings only**: User sets league once in Settings → Trade section. Overlay just uses it.
- **Overlay too**: Small league badge/selector in the trade results area.

**Recommendation**: Settings only. League changes once per ~3 months.

### Q3: "Open on trade" — external browser or built-in?

- **External browser** (simpler): `shell.open(tradeUrl)`. Users have trade site bookmarked anyway.
- **Built-in webview** (APT does this): Opens trade site in an embedded browser.

**Recommendation**: External browser. Built-in webview is a huge scope increase for
marginal benefit.

### Q4: Price grouping / stats?

- **Raw list**: Show each listing's price as-is (current design)
- **Grouped**: "5 listings at 50-60c, 3 at 1-1.5 div"
- **Summary**: "Median: 55c, Min: 50c, Max: 1.5 div"

**Recommendation**: Raw list for v1. It's what trade site shows and users understand.
Add summary stats later if needed.

### Q5: Currency normalization?

Prices come in mixed currencies (chaos, divine, exalted, etc.). Should we normalize?

- **No normalization (v1)**: Show as-is. Users know exchange rates.
- **Normalize to chaos/divine**: Requires poe.ninja exchange rates (new dependency).

**Recommendation**: No normalization for v1. Add later with poe.ninja integration.

---

## Implementation Plan

### Step 1: Trade Settings UI
- Add "Trade" section to GeneralSettings (or new TradeSettings tab)
- League input + relaxation slider + online-only toggle
- Refresh trade stats button with status display
- Store config in settings.json

### Step 2: Trade Action Bar
- New `TradeActionBar` component below score section
- Price Check + Open Trade buttons
- Disable if index not loaded (with helpful message)

### Step 3: Price Results
- New `TradeResults` component
- Loading / results / empty / error / rate-limited states
- Compact price list with currency

### Step 4: Wire It Up
- Store raw item text in App.tsx state (for passing to trade commands)
- Build TradeQueryConfig from settings store
- Connect buttons to Tauri commands
- Handle all states (loading, error, rate limit)

### Step 5: Styling
- Match existing overlay aesthetics
- Price list styled like mod section (warm tan text on dark background)
- Action buttons styled like watching-pill buttons (border, hover effects)
- Loading spinner consistent with PoE aesthetic

---

## Future (Phase 6+)

### Interactive overlay as trade query builder

The overlay itself becomes the search configuration UI. Instead of a separate filter
panel, the existing mod lines grow interactive controls when in "trade mode":

- **Checkboxes** on each mod line to include/exclude from search
- **Value sliders/inputs** to adjust min values per stat (pre-filled from relaxation %)
- **Roll bars become editable** — drag to set the search minimum
- **Base type / item level** become toggleable filters
- **"Search" button** updates live as you toggle stats

The item display *is* the query builder. No context switching — the user reads the
item, tweaks what matters, and searches. This leverages the fact that we already render
tier badges, type badges, and roll quality bars on every mod line. Adding a checkbox
and a value adjuster is the same visual language.

### Bulk item exchange

The trade API has two distinct endpoints:

| | Standard search | Bulk exchange |
|--|-----------------|---------------|
| **Endpoint** | `POST /api/trade/search/{league}` | `POST /api/trade/exchange/{league}` |
| **Use case** | Items with mods (gear, maps, gems) | Currency, fragments, div cards, oils, scarabs |
| **Query format** | `stats[]` (stat filters + values) | `{have: ["chaos"], want: ["divine"]}` (trade tags) |
| **Response** | Fixed prices (50 chaos, 1 divine) | Exchange ratios (1:180 rate, with stock) |

**Routing logic** (how APT does it):
1. If any stat filters are enabled → standard search
2. Else if the item has a bulk `tradeTag` → bulk exchange
3. Else → standard search

**What we need**:
- Item class → trade tag mapping in `poe-data/domain.rs` (e.g., `"Currency"` → `"currency"`,
  `"DivinationCard"` → `"card"`, fragments → `"fragment"`, etc.)
- Bulk query builder: `ResolvedItem` → `{have, want}` format
- Bulk client: POST to `/api/trade/exchange/`, parse ratio responses
- UI: exchange ratios displayed differently from fixed prices ("1 = 180c" vs "listed at 50c")

### In-game exchange (no API)

PoE's in-game currency exchange (added ~3.25) has no public API. Items traded through it
are invisible to both `/trade/search/` and `/trade/exchange/` endpoints. This affects:
- Currency (chaos, divine, exalted, etc.)
- Fragments
- Other bulk-tradeable items

**UX handling**: When showing price results for currency/fragment items, display a note:
"Currency is often traded via in-game exchange — prices there may differ from trade site listings."
This sets expectations without requiring any API integration.

### poe.ninja integration

poe.ninja provides historical price data, economy trends, and currency exchange rates.
The API has been reverse-engineered — reference implementation in `poe-agents` repo
(`_reference/poe-agents/` on the Linux disk, see its poe.ninja module).

**Use cases**:
- **Currency normalization**: Convert mixed-currency listings to a common base (chaos/divine)
  using live exchange rates, so "50 chaos" and "0.3 divine" are directly comparable
- **Price history**: Show price trend over time for uniques, div cards, etc.
- **Sanity check**: Compare trade listings against poe.ninja aggregate pricing
- **Rate limit**: 12 requests / 5 minutes (documented in CLAUDE.md Key Data Sources)

### Other future features

- Pseudo stat aggregation (total life, total res)
- Per-stat filter toggles (checkboxes on mod lines)
- Hotkey for instant price check (skip button click)
