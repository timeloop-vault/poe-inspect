# RQE Settings UI Design

## Overview

New "Demand Marketplace" tab in Settings. Gated behind mock login (`AccountName#1234`).
After login, shows query management UI with stat picker (reusing existing patterns).

## Tab Registration

In `SettingsApp.tsx`:
- Add `"marketplace"` to `Section` type union
- Add `{ id: "marketplace", label: "Marketplace" }` to sections array
- Render `<MarketplaceSettings />` when active

## Component Structure

```
MarketplaceSettings.tsx
├─ [Not logged in] → LoginGate
│   └─ Account name input (AccountName#1234)
│       Validate format, store locally, connect to rqe-server
│
└─ [Logged in] → MarketplacePanel
    ├─ AccountBar (identity + logout + server status)
    ├─ QueryList (your want lists)
    │   ├─ Each query: label, condition count, edit/delete
    │   └─ "+ Add Want List" button
    └─ QueryEditor (when editing/adding)
        ├─ Name/label input
        ├─ Condition list (reuse CompoundGroupEditor pattern)
        │   └─ Each condition → PredicateRow (reuse existing)
        │       └─ StatValueEditor for stat picks (reuse existing)
        └─ Save / Cancel bar
```

## States

### 1. LoginGate (not logged in)

```
┌─────────────────────────────────────────────┐
│  Demand Marketplace                         │
│                                             │
│  Connect to the reverse query engine to     │
│  register items you're looking for and get  │
│  notified when they appear.                 │
│                                             │
│  PoE Account Name                           │
│  ┌───────────────────────────────────────┐  │
│  │ PlayerName#1234                       │  │
│  └───────────────────────────────────────┘  │
│  Format: Name#0000                          │
│                                             │
│  Server URL                                 │
│  ┌───────────────────────────────────────┐  │
│  │ http://localhost:8080                 │  │
│  └───────────────────────────────────────┘  │
│                                             │
│  [Connect]                                  │
│                                             │
└─────────────────────────────────────────────┘
```

**Validation:**
- Account name: regex `^[A-Za-z0-9_-]+#\d{4}$`
- Server URL: basic URL validation
- On Connect: `GET /health` to verify server is reachable
- Store in `store.ts` as `MarketplaceSettings`

### 2. AccountBar (logged in header)

```
┌─────────────────────────────────────────────┐
│  Demand Marketplace                         │
│  ┌─────────────────────────────────────────┐│
│  │ ● Connected as Stefan#1234    [Logout] ││
│  │   3 want lists · 12 DAG nodes          ││
│  └─────────────────────────────────────────┘│
```

- Green dot = connected, red = disconnected
- Stats from `/health` endpoint (query count, node count)
- Logout clears stored credentials

### 3. QueryList (want lists)

```
│  Your Want Lists                            │
│  ┌─────────────────────────────────────────┐│
│  │ 🔍 Fast Cold Res Boots    [Edit] [Del] ││
│  │   4 conditions · build:ci-flicker      ││
│  ├─────────────────────────────────────────┤│
│  │ 🔍 Life Body Armour       [Edit] [Del] ││
│  │   3 conditions · build:rf-jugg         ││
│  ├─────────────────────────────────────────┤│
│  │ 🔍 +1 Gem Level Wand      [Edit] [Del] ││
│  │   5 conditions                         ││
│  └─────────────────────────────────────────┘│
│                                             │
│  [+ Add Want List]                          │
```

- Loaded from `GET /queries?owner=Stefan#1234` (needs server endpoint)
- Each row shows name (from first label), condition count
- Edit opens QueryEditor
- Delete confirms then `DELETE /queries/{id}`

### 4. QueryEditor

```
│  ┌─────────────────────────────────────────┐│
│  │ Want List Name                         ││
│  │ ┌─────────────────────────────────────┐││
│  │ │ Fast Cold Res Boots                 │││
│  │ └─────────────────────────────────────┘││
│  │                                        ││
│  │ Build Label (optional)                 ││
│  │ ┌─────────────────────────────────────┐││
│  │ │ build:ci-flicker                    │││
│  │ └─────────────────────────────────────┘││
│  │                                        ││
│  │ Conditions                             ││
│  │ ┌─ Match ALL of: ─────────────────────┐││
│  │ │                                     │││
│  │ │  Item Class  = [Boots         ▾]    │││
│  │ │  Rarity      = [Non-Unique    ▾]    │││
│  │ │  Cold Res    ≥ [30            ]     │││
│  │ │  Move Speed  ≥ [25            ]     │││
│  │ │                                     │││
│  │ │  [+ Add Condition]                  │││
│  │ └─────────────────────────────────────┘││
│  │                                        ││
│  │         [Cancel]  [Save Want List]     ││
│  └─────────────────────────────────────────┘│
```

**Condition types available:**
- Item Class (string select from known classes)
- Rarity Class (select: Non-Unique / Unique)
- Stat threshold (reuse StatValueEditor autocomplete)
- Boolean flags (corrupted, fractured, etc.)

**Stat picker reuse:**
- Same `invoke("get_stat_suggestions")` call
- Same two-phase autocomplete (template → hybrid detection)
- But key format uses `item_to_entry()` convention:
  `"explicit.base_maximum_life"` not `"base_maximum_life"`
- Need a mapping function or the suggestions include the Entry key format

## Store Types (`store.ts`)

```typescript
interface MarketplaceSettings {
    accountName: string | null;   // "Stefan#1234"
    serverUrl: string;            // "http://localhost:8080"
    apiKey: string | null;        // for auth
    enabled: boolean;             // enable RQE check on item inspect
}

// Default
const DEFAULT_MARKETPLACE: MarketplaceSettings = {
    accountName: null,
    serverUrl: "http://localhost:8080",
    apiKey: null,
    enabled: true,
};
```

Query data is NOT stored locally — it lives on the rqe-server.
The app fetches queries on login and caches them in component state.

## Backend Commands (Rust → Tauri)

New Tauri commands in `lib.rs` (or a dedicated module):

```rust
#[tauri::command]
async fn rqe_health(url: String) -> Result<HealthResponse, String>

#[tauri::command]
async fn rqe_add_query(url: String, api_key: Option<String>,
    conditions: Vec<Condition>, labels: Vec<String>,
    owner: String) -> Result<u64, String>

#[tauri::command]
async fn rqe_list_queries(url: String, owner: String) -> Result<Vec<StoredQuery>, String>

#[tauri::command]
async fn rqe_delete_query(url: String, api_key: Option<String>,
    id: u64) -> Result<bool, String>

#[tauri::command]
async fn rqe_match_item(url: String, api_key: Option<String>,
    entry: Entry) -> Result<MatchResponse, String>
```

These use `poe_rqe_client::RqeClient` internally.

## Server Endpoint Needed

Currently missing: `GET /queries?owner=X` to list a user's queries.
Add to rqe-server before the UI work.

## CSS Classes (new)

```css
.marketplace-login { /* centered card */ }
.marketplace-account-bar { /* connected status strip */ }
.marketplace-status-dot { /* green/red connection dot */ }
.query-list { /* list container */ }
.query-list-item { /* individual want list row */ }
.query-editor { /* editor panel */ }
```

Follow existing `.setting-group`, `.setting-row` patterns.
Reuse `.pred-*`, `.compound-*`, `.scoring-rule-*` classes for the condition editor.

## Implementation Order

1. Add `GET /queries?owner=X` endpoint to rqe-server
2. Add `MarketplaceSettings` type + load/save to `store.ts`
3. Create `MarketplaceSettings.tsx` with LoginGate
4. Create AccountBar + QueryList (fetch on login)
5. Create QueryEditor (reuse PredicateEditor/CompoundGroupEditor)
6. Add Tauri commands for RQE operations
7. Wire overlay: async RQE check on item inspect
