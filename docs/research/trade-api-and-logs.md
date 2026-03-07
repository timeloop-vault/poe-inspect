# Trade API, Client.txt Log File, and OAuth API Research

> Research output for poe-inspect-2. Based on community documentation, existing tool source code analysis, and GGG's developer docs.
>
> **Caveat**: This document was written from training-data knowledge (cutoff ~early 2025) without live web verification. Some details may have changed — verify endpoints and rate limits against current GGG docs before implementing.

---

## Part 1: GGG Trade API

The official trade site lives at `https://www.pathofexile.com/trade` (PoE1) and `https://www.pathofexile.com/trade2` (PoE2). Both expose a JSON API that the web frontend consumes.

### 1.1 API Endpoints

The trade API follows a two-step search-then-fetch pattern:

#### Search (POST)

```
POST https://www.pathofexile.com/api/trade/search/{league}
Content-Type: application/json
```

- `{league}` is the league name, e.g., `Mirage`, `Standard`, `Hardcore+Mirage`
- Request body contains the search query (filters, stats, sort order)
- Returns a result object with:
  - `id` — a search result ID (hash)
  - `total` — total number of matching listings
  - `result` — array of listing IDs (up to ~100 per search; more available via pagination)

#### Fetch (GET)

```
GET https://www.pathofexile.com/api/trade/fetch/{id1,id2,...}?query={searchId}
```

- Comma-separated listing IDs (max 10 per request)
- `query` parameter is the search ID from the search step
- Returns full listing details: item data, price, account info, stash tab, etc.

#### Stats Endpoint (for mod/stat IDs)

```
GET https://www.pathofexile.com/api/trade/data/stats
```

Returns all searchable stats/mods with their IDs, text patterns, and categories. This is essential for building mod search queries. Categories include:
- `explicit` — explicit mods
- `implicit` — implicit mods
- `crafted` — bench crafted mods
- `fractured` — fractured mods
- `enchant` — enchantments
- `pseudo` — pseudo stats (e.g., "+# total to Maximum Life" combining multiple sources)
- `monster` — map monster mods
- `veiled` — veiled mods

#### Static Data Endpoints

```
GET https://www.pathofexile.com/api/trade/data/items    — base item types and categories
GET https://www.pathofexile.com/api/trade/data/leagues  — available leagues
GET https://www.pathofexile.com/api/trade/data/static   — currency/fragment/etc. IDs for bulk exchange
```

### 1.2 Search Query Structure

A search query body looks like:

```json
{
  "query": {
    "status": { "option": "online" },
    "name": "...",
    "type": "...",
    "stats": [
      {
        "type": "and",
        "filters": [
          {
            "id": "explicit.stat_3299347043",
            "value": { "min": 90, "max": 100 },
            "disabled": false
          }
        ]
      }
    ],
    "filters": {
      "type_filters": {
        "filters": {
          "category": { "option": "weapon.staff" },
          "rarity": { "option": "rare" }
        }
      },
      "socket_filters": { ... },
      "misc_filters": { ... },
      "trade_filters": {
        "filters": {
          "price": { "min": 1, "max": 100, "option": "chaos" }
        }
      },
      "map_filters": { ... },
      "armour_filters": { ... },
      "weapon_filters": { ... },
      "req_filters": { ... }
    }
  },
  "sort": { "price": "asc" }
}
```

### 1.3 Searching by Specific Mods

Mod searching is done via the `stats` array in the query. Key details:

- **Stat IDs**: Each mod has a unique stat ID (e.g., `explicit.stat_3299347043` for "+# to Maximum Life"). Get these from the `/data/stats` endpoint.
- **Value ranges**: You can specify `min` and/or `max` for the mod's rolled value.
- **Logical grouping**: Stats filters support `"type": "and"`, `"type": "or"`, `"type": "count"` (with a `"value": {"min": N}` for "at least N of these"), `"type": "not"`, and `"type": "if"` (weighted sum).
- **Weight-based search**: The `"type": "weight"` filter lets you assign numeric weights to different stats and filter on the weighted sum. This is how tools compute a "DPS-equivalent" or custom scoring. Awakened PoE Trade uses weighted sums for its price predictions.
- **Tier filtering**: The trade API does NOT directly support searching by mod tier (T1, T2, etc.). You search by value range instead. To search for "T1 life on body armour," you'd set the min value to the T1 threshold (e.g., `min: 100` for the T1 range of +# to Maximum Life).
- **Pseudo stats**: Pseudo stats aggregate multiple sources. For example, `pseudo.pseudo_total_life` sums flat life, strength-based life, etc. Very useful for approximate searches.
- **Fractured/crafted/implicit**: Each mod type has its own stat category prefix, so you can specifically search for "fractured +life" vs "explicit +life."

### 1.4 Authentication

The trade API has minimal authentication requirements:

- **No OAuth required** for search/fetch — the API is publicly accessible
- **Session cookie (POESESSID)**: Optional. If provided, enables "online only" filtering and shows your own listings. The cookie is the value from `www.pathofexile.com` after logging in.
- **User-Agent**: GGG requires a descriptive User-Agent header for API consumers. Requests without one may be rate-limited more aggressively.
- **No API key**: There is no formal API key system for the trade API.

**How Awakened PoE Trade handles it**: Awakened PoE Trade (awpoetr) makes requests directly to the trade API endpoints without OAuth. It asks the user for their POESESSID (optional) and includes it as a cookie. It sets a custom User-Agent identifying itself. Source: the Electron app's network layer in its GitHub repo (`SnosMe/awakened-poe-trade`).

### 1.5 Rate Limits

GGG enforces rate limits via response headers. The relevant headers are:

```
X-Rate-Limit-Ip: 12:6:60,16:12:300
X-Rate-Limit-Ip-State: 1:6:0,1:12:0
X-Rate-Limit-Policy: trade-search-request-limit
```

The format is `hits:period:timeout` — meaning "N requests per M seconds, with T second penalty timeout if exceeded."

**Observed/documented limits (approximate, may change)**:

| Endpoint | Limit | Penalty |
|----------|-------|---------|
| Search | ~12 requests per 6 seconds, ~16 per 60 seconds | 60s ban on first tier, longer on repeat |
| Fetch | ~12 requests per 4 seconds, ~16 per 12 seconds | Similar escalating bans |

- Rate limit state is tracked per IP address.
- When you exceed the limit, the API returns `429 Too Many Requests` with a `Retry-After` header.
- The penalty timeout escalates: first violation might be 60s, repeated violations can be minutes or longer.
- **Best practice**: Parse the `X-Rate-Limit-*` headers from every response and throttle accordingly. Do not hardcode limits — they change without notice.
- Awakened PoE Trade implements a request queue with delay based on these headers.

### 1.6 Bulk Exchange API

Yes, there is a separate bulk exchange endpoint:

```
POST https://www.pathofexile.com/api/trade/exchange/{league}
```

Used for currency-to-currency, fragment, essence, scarab, and other bulk item trading. The query format differs from the item search:

```json
{
  "exchange": {
    "status": { "option": "online" },
    "have": ["chaos"],
    "want": ["exalted"]
  }
}
```

- `have` and `want` use static item IDs from `/api/trade/data/static`
- Returns listings with exchange ratios
- Same rate limit policy applies (separate bucket or shared — needs testing)
- The bulk exchange UI is at `https://www.pathofexile.com/trade/exchange/{league}`

### 1.7 PoE2 Trade API

PoE2 has its own trade site at `https://www.pathofexile.com/trade2`. The API structure is very similar but uses different base URLs:

```
POST https://www.pathofexile.com/api/trade2/search/{league}
GET  https://www.pathofexile.com/api/trade2/fetch/{ids}?query={searchId}
GET  https://www.pathofexile.com/api/trade2/data/stats
GET  https://www.pathofexile.com/api/trade2/data/items
GET  https://www.pathofexile.com/api/trade2/data/static
```

- Same query structure and authentication model
- Different stat IDs (PoE2 has different mods/affixes)
- Different item categories and base types
- Same rate limit infrastructure

**For our tool**: Abstract the base URL (`/api/trade/` vs `/api/trade2/`) and stat data, and the rest of the code can be shared.

### 1.8 Community Documentation / Repos

Several community resources document the trade API:

- **`poe-trade-api` GitHub repos**: Multiple community members have documented the API (search GitHub for "poe trade api"). The most thorough documentation tends to be in tool source code.
- **Awakened PoE Trade** (`SnosMe/awakened-poe-trade`): Open source Electron overlay tool. Its source code is the best "documentation" of how to interact with the trade API in practice. Look at `renderer/src/web/price-check/` for search query construction.
- **poe-trade-fetch** / **poeprices.info**: Some community price-checking services that consume the trade API.
- **GGG Forum Dev Manifesto posts**: GGG occasionally posts about trade API changes in the forum. No formal Swagger/OpenAPI spec exists.

---

## Part 2: Client.txt Log File

PoE writes game events to a log file called `Client.txt`. This is a plain-text, append-only log.

### 2.1 File Location

#### PoE1

| Platform | Install Type | Path |
|----------|-------------|------|
| Windows | Standalone | `C:\Program Files (x86)\Grinding Gear Games\Path of Exile\logs\Client.txt` |
| Windows | Steam | `C:\Program Files (x86)\Steam\steamapps\common\Path of Exile\logs\Client.txt` |
| Linux | Steam/Proton | `~/.steam/steam/steamapps/common/Path of Exile/logs/Client.txt` (or via compatdata) |
| macOS | Standalone | `~/Library/Application Support/Path of Exile/logs/Client.txt` |

#### PoE2

| Platform | Install Type | Path |
|----------|-------------|------|
| Windows | Standalone | `C:\Program Files (x86)\Grinding Gear Games\Path of Exile 2\logs\Client.txt` |
| Windows | Steam | `C:\Program Files (x86)\Steam\steamapps\common\Path of Exile 2\logs\Client.txt` |

> Note: The `logs/` subdirectory is standard. Some very old installs may have `Client.txt` in the game root directory instead.

### 2.2 Log Line Format

Each line follows this format:

```
2024/08/15 14:23:45 12345678 abc12345 [INFO Client 1234] <message>
```

Breaking it down:
- `2024/08/15 14:23:45` — Timestamp (local time, `YYYY/MM/DD HH:MM:SS`)
- `12345678` — Monotonic counter or internal tick (not always present in all versions)
- `abc12345` — Internal reference/hash (not always present)
- `[INFO Client 1234]` — Log level (`INFO`, `DEBUG`, `WARN`), component (`Client`), and thread/process ID
- `<message>` — The actual log message

Some lines may have slightly different formats depending on the game version, but the `[INFO Client NNNN]` prefix is consistent and is the main marker to look for.

### 2.3 Useful Events Logged

#### Character Login / Area Changes

```
[INFO Client 1234] : You have entered Lioneye's Watch.
```

This "You have entered X" message fires whenever the player changes areas (zones, hideout, maps, towns, etc.). This is the single most useful log event.

#### Login Events

```
[INFO Client 1234] login: login_name@account
[INFO Client 1234] Connected to realm_server_name
```

Login lines appear when the player authenticates and connects.

#### Character Selection

When a character is loaded, a line like this appears:

```
[INFO Client 1234] : Character name is MyCharacterName in league Mirage
```

> Note: The exact format of the character/league line varies by PoE version. In some versions, the character name is only inferable from the login sequence or from "You have entered" + process state. The most reliable approach tools use is to monitor the login sequence.

#### Trade/Party/Chat Messages

```
[INFO Client 1234] @From PlayerName: Hi, I'd like to buy your Kaom's Heart listed for 50 chaos in Mirage
[INFO Client 1234] @To PlayerName: sold, ty
[INFO Client 1234] : PlayerName has joined the party.
[INFO Client 1234] : PlayerName has left the party.
```

- Incoming whispers: `@From`
- Outgoing whispers: `@To`
- Party events, trade requests, and system messages are all logged
- Trade whisper format includes the item name, price, league, and stash position — tools like Mercury Trade and Trades Companion parse these

#### Map/Zone Information

```
[INFO Client 1234] : You have entered The Blood Aqueduct.
[INFO Client 1234] : You have entered Azurite Mine.
[INFO Client 1234] : You have entered Hideout.
[INFO Client 1234] : You have entered Karui Shores.
```

Zones can be categorized:
- **Towns**: Lioneye's Watch, The Forest Encampment, The Sarn Encampment, Highgate, Overseer's Tower, The Bridge Encampment, Oriath, Karui Shores
- **Hideout**: Contains "Hideout" in the name (e.g., "Celestial Hideout", just "Hideout")
- **Maps**: Map names (e.g., "Strand Map", "Burial Chambers Map") — though the zone name doesn't always include "Map"
- **Campaign zones**: Everything else

#### AFK Mode

```
[INFO Client 1234] : AFK mode is now ON. Returning to this computer will turn it off.
[INFO Client 1234] : AFK mode is now OFF.
```

#### Level Up

```
[INFO Client 1234] : MyCharacterName is now level 85
```

#### Other Events

- Generating area seeds (map generation)
- Connecting to instances
- Abnormal disconnects
- NPC dialogue (some versions)

### 2.4 Auto-Detection Capabilities

Based on log events, we can detect:

| What | How | Reliability |
|------|-----|-------------|
| **Active character** | Parse login sequence or level-up messages. The character name appears in `"is now level"` lines and can sometimes be inferred from the login flow. | Medium — may need to combine with OAuth API for certainty |
| **Current league** | Parse login/character selection line if present, or infer from the trade API league list + zone context | Medium |
| **Current zone type** | Parse `"You have entered X"` and classify by known zone lists | High |
| **In hideout** | Zone name contains "Hideout" | High |
| **In town** | Zone name matches known town list | High |
| **In map** | Zone entered after a map device activation (heuristic) or matches known map names | Medium |
| **Online/AFK status** | AFK on/off messages | High |
| **Trade whispers** | `@From` / `@To` messages with trade format | High |

### 2.5 File Size and Rotation

- **Client.txt grows unbounded**. It is NOT rotated by the game client.
- Over a league, it can grow to hundreds of MB or even multiple GB.
- The file is truncated/reset only when the user manually deletes it, or sometimes on game patches (not guaranteed).
- **Implication for our tool**: Do NOT read the entire file. Instead:
  - On startup, seek to the end of the file
  - Tail new lines only (watch for appends)
  - Optionally do a backward scan for the most recent login/character event to initialize state
  - On Windows, use `ReadDirectoryChangesW` or poll with file size checks. On Linux, use `inotify`. On macOS, use `kqueue`/`FSEvents`.
  - Rust crates: `notify` (cross-platform file watcher) works well for this

### 2.6 Encoding

- Client.txt is UTF-8 encoded
- Line endings are platform-dependent (CRLF on Windows, LF on Linux/macOS)
- Some item names and player names contain non-ASCII Unicode characters

---

## Part 3: GGG OAuth API

GGG provides an official OAuth2 API for accessing account and character data. Documentation is at `https://www.pathofexile.com/developer/docs`.

### 3.1 OAuth Flow

GGG uses standard OAuth 2.0 Authorization Code flow with PKCE (Proof Key for Code Exchange), which is suitable for desktop applications.

#### Registration

- Developers register applications at `https://www.pathofexile.com/developer/apps`
- Registration provides a `client_id`
- Desktop/native apps use the "Confidential" or "Public" client type
- Redirect URI can be `http://localhost:{port}` for desktop apps or a custom URI scheme

#### Authorization Endpoint

```
GET https://www.pathofexile.com/oauth/authorize
  ?client_id={client_id}
  &response_type=code
  &scope={scopes}
  &state={random_state}
  &redirect_uri={redirect_uri}
  &code_challenge={challenge}
  &code_challenge_method=S256
```

This opens the user's browser to GGG's login/consent page. After authorization, GGG redirects to the `redirect_uri` with an authorization code.

#### Token Endpoint

```
POST https://www.pathofexile.com/oauth/token
Content-Type: application/x-www-form-urlencoded

client_id={client_id}
&grant_type=authorization_code
&code={authorization_code}
&redirect_uri={redirect_uri}
&code_verifier={verifier}
```

Returns:
```json
{
  "access_token": "...",
  "token_type": "bearer",
  "expires_in": 2592000,
  "scope": "account:profile account:characters",
  "refresh_token": "..."
}
```

- Access tokens expire (typically 30 days)
- Refresh tokens can be used to get new access tokens without re-authorization

#### Desktop App Flow

For a Tauri/desktop app:
1. Generate PKCE code verifier and challenge
2. Open user's default browser to the authorize URL
3. Start a local HTTP server on `localhost:{port}` to catch the redirect
4. Exchange the authorization code for tokens
5. Store tokens securely (OS keychain via `keyring` crate or similar)

### 3.2 Available Scopes

| Scope | What it grants |
|-------|---------------|
| `account:profile` | Read account name, realm |
| `account:characters` | List characters, read character details (ascendancy, level, league) |
| `account:stashes` | Read stash tab contents |
| `account:item_filter` | Read/write item filters |
| `account:league_accounts` | Read league-specific account info |

### 3.3 API Endpoints (Authenticated)

All authenticated endpoints use Bearer token:

```
Authorization: Bearer {access_token}
```

#### Profile

```
GET https://www.pathofexile.com/api/profile
```

Returns account name, UUID, realm.

#### Character List

```
GET https://www.pathofexile.com/api/character
```

Returns list of all characters on the account with: name, league, class, ascendancy, level, experience.

> Note: The exact endpoint paths may vary. GGG's documentation uses paths like `/character` or `/api/character`. Some community docs reference `/api/profile/characters`. Check current docs.

#### Character Details

```
GET https://www.pathofexile.com/api/character/{characterName}
```

Returns full character data including:
- Equipment (all equipped items with full item data — mods, sockets, links)
- Skill gems and links
- Passive tree (allocated nodes)
- Ascendancy
- Bandits choice

#### Stash Tabs

```
GET https://www.pathofexile.com/api/stash/{league}
```

Returns list of stash tabs (names, types, IDs). Individual tab contents:

```
GET https://www.pathofexile.com/api/stash/{league}/{stashId}
```

Returns all items in the stash tab with full item data.

### 3.4 Rate Limits

The OAuth API has its own rate limits, separate from the trade API:

- Limits are returned via response headers (same `X-Rate-Limit-*` format as trade API)
- Typical limits: ~30-45 requests per minute for character/stash endpoints
- Policy names differ per endpoint (e.g., `character-request-limit`, `stash-request-limit`)
- Same `429` + `Retry-After` behavior when exceeded
- Rate limits are per access token and per IP

### 3.5 PoE2 OAuth API

PoE2 uses the same OAuth infrastructure. The distinction is:

- Characters and stashes are separated by game version
- Some endpoints may use a different base path (e.g., `/api/poe2/character`)
- The same `client_id` and tokens work for both games (the account is shared)
- Item data format differs (PoE2 items have different mod structures, socket system, etc.)

### 3.6 Usefulness for Our Tool

| Use Case | Feasibility | Notes |
|----------|-------------|-------|
| **Auto-detect active character** | Possible but indirect | Can list characters and compare with Client.txt events (level-up, zone changes) to identify which one is active. No "currently playing" endpoint. |
| **Get equipped items** | Yes | Full item data for equipped gear — useful for build context, "what would this upgrade?" analysis |
| **Get passive tree** | Yes | Know which keystones/notables are allocated — enables build-aware evaluation |
| **Get stash contents** | Yes | Could scan stash for comparison — "you already have a better helmet" |
| **Detect league** | Yes | Character list includes league info |
| **Real-time tracking** | No | API is request/response, not streaming. Must poll. Rate limits make frequent polling impractical. |

**Recommended approach for our tool**:
1. Use OAuth API on startup (or periodically) to fetch character list and equipment
2. Use Client.txt log tailing for real-time event detection (zone changes, active character inference)
3. Combine both: OAuth gives us the character/build data, Client.txt gives us real-time state
4. Cache OAuth data aggressively — character builds don't change every second

---

## Summary: What Matters for poe-inspect-2

### Immediate (MVP-relevant)

- **Trade API**: Useful for the "Trade valuation" evaluation layer. Construct search queries from parsed item mods, fetch comparable listings, estimate price. No auth required for basic searches.
- **Client.txt**: Tail for zone detection (hideout/town/map context). Low effort, high value.

### Post-MVP

- **OAuth API**: Enable build-aware evaluation by fetching the player's passive tree and equipped items. Requires app registration and user authorization flow.
- **Bulk Exchange API**: Useful for currency-related items (scarabs, fragments, essences).
- **Trade API weighted search**: Advanced price estimation using weighted stat sums.

### Implementation Priorities

1. **Trade API client** — search + fetch with proper rate limit handling (parse `X-Rate-Limit-*` headers, implement request queue with backoff)
2. **Client.txt log tailer** — cross-platform file watcher, parse zone/character events
3. **OAuth integration** — desktop PKCE flow, token storage, character/build fetching
4. **Stat ID mapping** — fetch `/data/stats` and map our parsed mod text to trade API stat IDs

### Key Risks

- **Rate limits**: The trade API rate limits are tight. A tool that triggers a search on every item inspect will hit limits quickly. Need smart caching and debouncing.
- **API stability**: GGG does not version the trade API and can change it without notice. Community tools break periodically.
- **POESESSID sensitivity**: If we accept the user's session ID, we must handle it securely (never log it, store encrypted, transmit only to GGG endpoints).
- **Client.txt parsing fragility**: Log format is not formally specified and can change between patches. Build robust parsing with fallbacks.
