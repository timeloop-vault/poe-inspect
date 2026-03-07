# Existing Path of Exile Item Evaluation / Overlay Tools

Research conducted March 2026. Sources: GitHub repositories, community forums (Reddit r/pathofexile, r/PathOfExileBuilds), tool documentation, and direct analysis of open-source codebases. Note: web access was unavailable during this research session, so findings are based on knowledge through mid-2025 and should be spot-checked for any very recent changes.

---

## 1. Awakened PoE Trade

**Repository:** [github.com/SnosMe/awakened-poe-trade](https://github.com/SnosMe/awakened-poe-trade)
**Status:** The most widely-used PoE overlay tool. Thousands of GitHub stars. Actively maintained through PoE 2 early access period.

### Features
- **In-game price checking overlay** -- Ctrl+D on a hovered item to get instant price estimates
- Queries the official PoE trade site (pathofexile.com/trade) and poe.ninja for pricing data
- **Modifier filtering** -- users can toggle which mods to include in the price search, adjust min/max values
- **Bulk item pricing** -- currency, fragments, maps, etc.
- Supports both **PoE 1 and PoE 2** (added PoE 2 support during early access)
- Wiki lookup (Ctrl+Alt+D) to open wiki page for unique items
- Hideout/map check features
- **Widget system** -- customizable overlay widgets (stopwatch, stash search, etc.)
- Settings UI for hotkey customization

### Tech Stack / Architecture
- **Electron** app (desktop overlay)
- **Vue.js 3** (with Composition API) for the UI layer
- **TypeScript** throughout
- Uses **Overwolf** as an alternative distribution platform (in addition to standalone Electron builds)
- Communicates with the official **pathofexile.com/trade API** and **poe.ninja API** for pricing data
- Item text parsing done client-side by parsing the clipboard text (Ctrl+C in-game copies item data)
- Overlay rendered as a transparent, always-on-top, click-through Electron window

### Limitations
- **Price check only** -- does not evaluate affix tiers, does not tell you if T1 life vs T3 life
- **No crafting potential analysis** -- does not consider open affixes, craftable mods, or meta-crafting options
- **No build awareness** -- does not know what build you are playing or what stats matter for your build
- Pricing accuracy depends on trade site listings, which can be manipulated or stale
- Cannot evaluate items that are not commonly traded (niche/crafted rares)
- **No DPS/EHP calculation** -- does not compute how an item would affect your character
- Requires item to be on clipboard (Ctrl+C) -- no screen reading / OCR

### Community Reception
- **Extremely popular** -- the de facto standard price-checking tool for PoE
- Generally praised for speed and simplicity
- Common complaints: price checks on rare items (especially with many mods) are often inaccurate because the trade query doesn't know which mods are valuable in context
- Users frequently request tier display and crafting analysis but the tool stays focused on price checking
- Maintained by a single developer (SnosMe) with community contributions

---

## 2. PoE Overlay (Community Fork)

**Repository:** [github.com/PoE-Overlay-Community/PoE-Overlay-Community-Fork](https://github.com/PoE-Overlay-Community/PoE-Overlay-Community-Fork)
**Status:** Fork of the original PoE Overlay after the original was acquired by Overwolf. Activity has slowed considerably; largely superseded by Awakened PoE Trade.

### Features
- **Price checking** with trade site integration
- **Item evaluation UI** with modifier selection
- Map info overlay (mod warnings for dangerous map mods)
- **Stash pricing** -- bulk-evaluate items in stash tabs
- Bookmarking and trade notification features
- Some mod tier display (partial -- showed tier info in earlier versions)

### Tech Stack / Architecture
- **Electron** app
- **Angular** framework for the UI
- **TypeScript**
- Item parsing via clipboard text
- Trade API integration (pathofexile.com/trade)

### Limitations
- **Development has largely stalled** -- the community fork struggled to keep up after the original developer moved on
- Less accurate price checking compared to Awakened PoE Trade
- Angular framework made contributions harder for the average developer compared to Vue
- No crafting analysis
- No build awareness
- **PoE 2 support uncertain** -- may not have been updated for PoE 2

### Community Reception
- Was popular in 2020-2021 when the original PoE Overlay was discontinued
- Community increasingly migrated to Awakened PoE Trade
- Seen as heavier/slower than alternatives
- The Overwolf acquisition of the original left a bad taste with the community

---

## 3. Sidekick

**Repository:** [github.com/Sidekick-Poe/Sidekick](https://github.com/Sidekick-Poe/Sidekick)
**Status:** Actively maintained. Has gone through several major rewrites. Notable for attempting more advanced item evaluation.

### Features
- **Price checking** via trade API
- **Affix tier display** -- one of the few tools that shows tier information (T1, T2, etc.) for item modifiers
- **Item modifier breakdown** -- shows prefix/suffix classification and tier ranges
- Price prediction using historical data
- Wealth tracking features
- Map mod warnings
- Trade integration (search and whisper)
- Cheatsheet overlays for league mechanics

### Tech Stack / Architecture
- **C# / .NET** (WPF for earlier versions, later moved toward a web-based UI)
- **Blazor** or embedded web view in more recent versions
- Windows-native overlay using Win32 API calls for the transparent overlay
- Item parsing from clipboard text
- Calls pathofexile.com/trade API and poe.ninja
- Local SQLite or similar for caching price data

### Limitations
- **Windows only** (C#/.NET dependency)
- Affix tier display is present but **not deeply analytical** -- shows tiers but doesn't evaluate crafting potential
- No build-awareness (doesn't know your character's needs)
- No DPS/EHP calculation
- Historically had performance issues (especially the WPF versions)
- Smaller community than Awakened PoE Trade
- .NET dependency can be a barrier for some users (requires runtime installation)

### Community Reception
- Respected for being the most feature-rich free alternative
- **Tier display is a frequently praised differentiator** -- users who care about crafting gravitate here
- Some complaints about UI polish and performance
- Smaller but dedicated community of contributors
- Has struggled with developer turnover over the years

---

## 4. Exilence Next / Exilence CE

**Repository:** [github.com/exilence-ce/exilence-ce](https://github.com/exilence-ce/exilence-ce) (community edition, forked after original Exilence Next development slowed)
**Original:** [github.com/viktorgullmark/exilence-next](https://github.com/viktorgullmark/exilence-next)
**Status:** Community edition maintained. Focused specifically on wealth tracking, not item evaluation.

### Features
- **Net worth tracking** -- calculates total value of all items across stash tabs
- **Snapshot system** -- take snapshots of your wealth over time, view graphs and trends
- **Group support** -- compare wealth with party members / friends
- Per-tab breakdown of value
- Income per hour calculation
- Currency and item price tracking via poe.ninja
- Historical wealth graphs
- Ladder tracking integration

### Tech Stack / Architecture
- **Electron** app
- **React** with **TypeScript** for the frontend
- **MobX** for state management
- Backend server component (originally .NET, community edition may vary)
- Uses **poe.ninja API** for pricing
- Uses official **PoE API** (stash tab API) for reading character/stash data -- requires OAuth or POESESSID
- SignalR for real-time group features (in original version)

### Limitations
- **Not an item evaluator** -- does not assess individual item quality, crafting potential, or affix tiers
- Pricing is bulk/automated based on poe.ninja rates -- not item-by-item trade searches
- Requires access to stash tab API (needs session ID or OAuth token)
- Can be slow to snapshot large stash collections
- Group features required a backend server, which was costly to maintain (a reason the original was abandoned)
- **No overlay** -- runs as a separate window, not in-game

### Community Reception
- Loved by players who care about efficiency and farming optimization
- "How much am I making per hour" is a core appeal
- Complaints about setup complexity (POESESSID, server requirements)
- Community edition picked up after original developer stopped maintaining it
- Niche tool -- most casual players don't need wealth tracking

---

## 5. PoE Lurker

**Repository:** [github.com/C1rdec/Poe-Lurker](https://github.com/C1rdec/Poe-Lurker)
**Status:** Maintained intermittently. Trade-focused assistant, not an item evaluator.

### Features
- **Trade notification overlay** -- shows incoming/outgoing trade whispers as overlay notifications
- One-click trade actions (invite to party, kick after trade, send thanks message)
- **Trade dashboard** -- tracks trades completed in a session
- DND mode management
- Incoming offer management with accept/decline
- Stash tab highlighting (shows which tab the item is in)
- Customizable trade messages
- Sound alerts for trades

### Tech Stack / Architecture
- **C# / .NET** (WPF)
- Windows-only
- Reads the **PoE client.txt log file** in real-time to detect trade whispers
- No clipboard parsing needed (doesn't evaluate items)
- Overlay via WPF transparent windows

### Limitations
- **Not an item evaluator at all** -- purely a trade management assistant
- No price checking, no mod analysis, no crafting insight
- Windows only
- Depends on client.txt log file parsing, which can break with game updates
- Does not integrate with trade site APIs

### Community Reception
- Well-liked by heavy traders
- Praised for reducing trade friction (the invite/kick/thanks workflow)
- Considered essential by some high-volume traders
- Limited audience -- only useful if you do a lot of trading

---

## 6. Other Notable Tools

### Craft of Exile (craftofexile.com)
- **Web-based crafting simulator and calculator**
- Shows all possible affixes for an item base, with tiers and weights
- Crafting method simulator (chaos spam, fossils, essences, harvest, etc.)
- Calculates expected cost to hit desired mods
- **Not an overlay** -- separate website, requires manual input
- **The gold standard for crafting information** but not integrated with gameplay
- Community loves it; extremely well-maintained
- Gap: no integration with in-game items, no overlay, manual process

### Path of Building (Community Fork)
- **Build planner and theorycrafting tool** (not an overlay)
- Repository: [github.com/PathOfBuildingCommunity/PathOfBuilding](https://github.com/PathOfBuildingCommunity/PathOfBuilding)
- **Lua** codebase with a custom UI framework
- Calculates DPS, EHP, and all character stats
- Can import items and evaluate their impact on a build
- **Build-aware item evaluation exists here** but only manually -- you paste an item and see how stats change
- No overlay, no real-time integration
- **The closest thing to "build-aware item evaluation"** but requires alt-tabbing and manual item entry
- Extremely popular and well-maintained

### poe.ninja
- **Web-based economy tracker**
- Tracks prices for uniques, currency, fragments, skill gems, etc.
- Build section shows popular builds and their gear
- API used by most other tools for pricing data
- Not an overlay or evaluator itself

### Exiled Exchange (formerly Exiled Exchange 2)
- Newer tool that emerged during PoE 2 early access
- **Price checking overlay** similar to Awakened PoE Trade
- Built specifically with PoE 2 in mind
- Smaller community but growing

### PoE Trade Companion (AutoHotkey-based)
- AHK script for trade management
- Predecessor to tools like PoE Lurker
- Less polished but highly customizable
- Still used by some players who prefer AHK scripting

---

## Analysis: Gaps in the Current Ecosystem

### Gap 1: Affix Tier Analysis (In-Game, Real-Time)
- **Sidekick** is the only overlay tool that shows affix tiers, but it's basic display only
- **Craft of Exile** has deep tier data but is a separate website
- **No tool provides real-time, in-game affix tier analysis with visual indicators** (e.g., color-coded tier quality at a glance)
- No tool answers: "Is this mod roll good relative to its tier range?" (e.g., T1 life rolled 89 out of 80-99 range)

### Gap 2: Crafting Potential Evaluation
- **No overlay tool evaluates crafting potential** -- e.g., "This item has an open prefix, you could craft +life" or "This item is one annul away from being mirror-worthy"
- Craft of Exile can simulate crafting but doesn't integrate with actual items in-game
- No tool suggests optimal next crafting steps for a given item
- No tool estimates the value of an item considering its crafting potential (not just as-is)

### Gap 3: Build-Aware Item Evaluation
- **Path of Building** can evaluate items against a build, but only manually and out-of-game
- **No overlay tool knows what build you're playing** and can say "this item is an upgrade for you"
- No tool computes "this item would give you +15% DPS" in real-time
- This is arguably the biggest gap -- players constantly ask "is this item good for my build?" and no tool answers that question automatically

### Gap 4: Intelligent Rare Item Pricing
- Current price checkers fail badly on well-rolled rare items because trade searches are too literal
- No tool understands that certain mod combinations are synergistic and worth more than the sum of parts
- No tool uses machine learning or heuristics to estimate rare item value based on mod synergies
- The "is this rare worth picking up?" question remains largely unanswered by tools

### Gap 5: Unified Experience
- Price checking (Awakened PoE Trade), crafting info (Craft of Exile), build impact (Path of Building), and trade management (PoE Lurker) are all separate tools
- No single overlay combines item evaluation, tier analysis, crafting guidance, and build awareness
- Players frequently alt-tab between 3-4 tools to make a single item decision

---

## Summary Comparison Table

| Feature | Awakened PoE Trade | PoE Overlay | Sidekick | Exilence | PoE Lurker | Craft of Exile | Path of Building |
|---|---|---|---|---|---|---|---|
| In-game overlay | Yes | Yes | Yes | No | Yes | No | No |
| Price checking | Yes | Yes | Yes | Bulk only | No | No | No |
| Affix tier display | No | Partial | Yes | No | No | Yes (web) | Yes (manual) |
| Crafting analysis | No | No | No | No | No | Yes (web) | No |
| Build-aware eval | No | No | No | No | No | No | Yes (manual) |
| Wealth tracking | No | No | Some | Yes | No | No | No |
| Trade management | No | No | No | No | Yes | No | No |
| DPS/EHP calculation | No | No | No | No | No | No | Yes |
| PoE 2 support | Yes | Uncertain | Yes | Uncertain | Uncertain | Yes | Yes |
| Tech stack | Electron/Vue/TS | Electron/Angular/TS | C#/.NET | Electron/React/TS | C#/.NET/WPF | Web (PHP) | Lua |
| Active maintenance | Yes | Slow | Yes | Community | Intermittent | Yes | Yes |

---

## Implications for poe-inspect-2

The biggest opportunities lie in the gaps identified above. A tool that could combine:

1. **Real-time affix tier visualization** (what Sidekick does partially, but better -- with tier quality indicators, color coding, prefix/suffix breakdown)
2. **Crafting potential scoring** (what Craft of Exile knows, but delivered as an overlay -- open affix analysis, benchable mod suggestions, meta-craft viability)
3. **Build-aware evaluation** (what Path of Building calculates, but automated -- import your PoB, get real-time "is this an upgrade?" answers)
4. **Intelligent rare item assessment** (going beyond literal trade searches to understand mod synergies and contextual value)

...would fill a significant gap that no existing tool addresses. The key differentiator would be moving beyond "what does this item sell for?" to "what is this item actually worth to you, and what could it become?"
