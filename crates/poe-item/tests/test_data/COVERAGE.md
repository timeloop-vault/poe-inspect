# Fixture Coverage — poe-item Parser

Fixtures are the **primary validation tool** for the entire parsing pipeline. When
something doesn't parse correctly, the fix is: add a fixture, reproduce the failure,
fix the grammar/resolver, verify the test passes. This applies not just to poe-item
but to any stage of the pipeline that processes item text.

## How to add fixtures

1. In-game: hover item → Ctrl+Alt+C (advanced copy with mod headers)
2. Paste into a new `.txt` file in this directory
3. Name: `{rarity}-{slot/type}-{notable-feature}.txt`
4. `all_fixtures_parse` auto-discovers all `.txt` files — no list to update
5. If the fixture exposes a new section type or edge case, add targeted assertions

## Current coverage (41 fixtures)

### Equipment — Rare (15)
- battered-foil-rare-ess-craft.txt (1H sword, essence craft, master craft)
- rare-axe-fractured.txt (2H axe, fractured mod)
- rare-axe-shaper.txt (2H axe, Shaper influence)
- rare-belt-crafted.txt (belt, implicit, master crafted)
- rare-body-armour-enchanted.txt (body, lab enchant, Redeemer)
- rare-boots-crafted.txt (boots, master crafted)
- rare-boots-eater-exarch.txt (boots, dual influence, trailing markers)
- rare-gloves-eater-exarch.txt (gloves, dual influence)
- rare-gloves-warlord.txt (gloves, Warlord influence)
- rare-helmet-crafted.txt (helmet, ES base, master crafted)
- rare-ring-synthesised.txt (ring, Synthesised)
- rare-shield-crafted.txt (shield, Chance to Block, implicit, master crafted)
- rare-sword-essence-crafted.txt (1H sword, essence craft)
- rare-wand-standard.txt (wand, no influence)
- rare-amulet-talisman-corrupted.txt (amulet, talisman, corrupted, enchant, flavor text)

### Equipment — Unique (4)
- unique-bow-short-bow.txt (bow, weapon sub-header, 7 unique mods, flavor text)
- unique-flask-doedres-flask.txt (mana flask, flask props + unique mods + flavor)
- unique-quiver-soul-strike.txt (quiver, flavor text, corrupted, Relic Unique)
- unique-ring-ventors-gamble.txt (ring, negative ranges, unique mods)

### Equipment — Normal (1)
- normal-staff-elder.txt (staff, Elder influence, implicit)

### Equipment — Magic (2)
- magic-axe-two-handed.txt (warstaff, implicit + prefix, B.1 ambiguity test)
- magic-jewel-cobalt.txt (cobalt jewel, prefix + suffix)

### Flasks (4)
- magic-flask-life.txt (life flask, prefix + suffix, locale decimal 3,50)
- magic-flask-mana.txt (mana flask, prefix + suffix, multi-line reminder)
- magic-flask-utility.txt (Normal quicksilver flask, no mods, properties only)
- unique-flask-doedres-flask.txt (unique mana flask, flask props + unique mods)

### Jewels (2)
- magic-cluster-jewel-large.txt (large cluster, multi-line enchants + prefix + suffix)
- magic-cluster-jewel-normal.txt (Normal medium cluster, enchants only, no mods)

### Maps (9)
- normal-map-alleyways.txt (T1, minimal)
- rare-map-shore.txt (T6)
- rare-map-city-square-currency.txt (T16)
- rare-map-city-square-delirium.txt (T16, delirium enchant, corrupted)
- rare-map-forge-phoenix.txt (T16, guardian)
- rare-map-maze-legacy.txt (T16)
- rare-map-residence-reward.txt (T17)
- rare-map-abomination-t17.txt (T17, many mods)
- unique-map-actons-nightmare.txt (unique map, flavor text)

### Gems (3)
- leap-slam.txt (attack gem, blank line in section, experience)
- gem-support-faster-casting.txt (support gem, Cost & Reservation Multiplier)
- gem-skill-transfigured-consecrated-path-of-endurance.txt (transfigured gem, cooldown, Transfigured status)

### Currency (1)
- coffin.txt (Filled Coffin, Necropolis-specific)

### Divination Cards (1)
- divination-card-hunters-resolve.txt (stack size, reward hint, flavor text)

## MISSING — Remaining gaps

Priority ordered by impact on the parser/resolver:

### P1 — Important item types with no coverage
- [ ] **Unique jewel** — e.g., Watcher's Eye (multi-mod with no tier)
- [ ] **Unique armour** — e.g., Shavronne's Wrappings (armour + unique mods)
- [ ] **Vaal gem** — has both base and Vaal skill sections
- [ ] **Unidentified item** — no mod section at all, "Unidentified" status
- [ ] **Abyssal jewel** — different Item Class ("Abyss Jewel")

### P2 — Coverage gaps in existing categories
- [ ] **Dagger/claw** — no dagger or claw
- [ ] **Veiled item** — veiled mods display differently
- [ ] **Mirrored item** — Mirrored status marker
- [ ] **Crusader/Hunter influence** — in grammar but untested

### P3 — Niche / league-specific
- [ ] **Regular currency** (Chaos Orb, Divine, etc.) — simpler than coffin
- [ ] **Fragment/Scarab** — different Item Class
- [ ] **Heist contract/blueprint** — unique section structure
- [ ] **Awakened support gem** — different level/quality display?
- [ ] **Split item** — Split status marker (in grammar, untested)
- [ ] **Enchanted item without mod headers** — Ctrl+C (non-advanced) format

## Feature matrix — what the parser handles

| Feature | Grammar | Tree walker | Resolver | Tested |
|---------|---------|-------------|----------|--------|
| Header (class, rarity, names) | ✅ | ✅ | — | ✅ |
| Rare/Unique 2-line header | ✅ | ✅ | — | ✅ |
| Normal/Magic/Gem/Currency 1-line header | ✅ | ✅ | — | ✅ |
| Divination Card header | ✅ | ✅ | — | ✅ |
| Magic base type extraction | — | — | TODO | ❌ |
| Requirements section | ✅ | ✅ | — | ✅ |
| Sockets section | ✅ | ✅ | — | ✅ |
| Item Level section | ✅ | ✅ | — | ✅ |
| Monster Level section (maps) | ✅ | ✅ | — | ✅ |
| Talisman Tier section | ✅ | ✅ | — | ✅ |
| Experience section (gems) | ✅ | ✅ | — | ✅ |
| Mod headers (prefix/suffix/implicit/unique) | ✅ | ✅ | — | ✅ |
| Master Crafted mods | ✅ | ✅ | — | ✅ |
| Mod tiers (Tier: N) | ✅ | ✅ | — | ✅ |
| Mod ranks (Rank: N) | ✅ | ✅ | — | ✅ |
| Mod tags | ✅ | ✅ | — | ✅ |
| Influence implicit mods (Exarch/Eater) | ✅ | ✅ | — | ✅ |
| Influence tier (Greater/Grand/etc) | ✅ | ✅ | — | ✅ |
| Multi-line mod body | ✅ | ✅ | — | ✅ |
| Reminder text (parenthesized) | ✅ | ✅ | — | ✅ |
| (implicit)/(crafted)/(enchant)/(fractured) suffixes | ✅ | ✅ | — | ✅ |
| Unscalable Value annotation | ✅ | ✅ | — | ✅ |
| Influence markers (standalone section) | ✅ | ✅ | — | ✅ |
| Influence markers (trailing after mods) | ✅ | ✅ | — | ✅ |
| Corrupted/Transfigured status | ✅ | ✅ | — | ✅ |
| Fractured/Synthesised markers | ✅ | ✅ | — | ✅ |
| Relic Unique marker | ✅ | ✅ | — | ✅ |
| Flavor text (generic section) | ✅ | ✅ | — | ✅ |
| Weapon sub-header + properties | ✅ (generic) | ✅ | — | ✅ |
| Defence properties | ✅ (generic) | ✅ | — | ✅ |
| Map properties | ✅ (generic) | ✅ | — | ✅ |
| Flask properties (Recovers/Consumes/Charges) | ✅ (generic) | ✅ | — | ✅ |
| Cluster jewel enchants | ✅ (generic) | ✅ | — | ✅ |
| Enchant lines | ✅ (generic) | ✅ | — | ✅ |
| Blank lines within sections (gems) | ✅ | ✅ | — | ✅ |
| Locale decimals (3,50 vs 3.50) | ✅ (generic) | ✅ | — | ✅ |
| Stat line → stat ID resolution | — | — | TODO | ❌ |
| Flask property vs modifier disambiguation | — | — | TODO | ❌ |
| Value range parsing (e.g., +68(65-68)) | — | — | TODO | ❌ |
| Negative ranges (e.g., 1(10--10)%) | — | — | TODO | ❌ |
