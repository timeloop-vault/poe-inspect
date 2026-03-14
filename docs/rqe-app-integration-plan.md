# RQE App Integration Plan

## Goal

Wire the Reverse Query Engine into the poe-inspect overlay app, gated behind
user identity. Players can register "want lists" and see when inspected items
match other players' demands.

## Auth Strategy

Two phases only — no throwaway intermediate auth:

1. **Mock (now):** Login screen in Settings, user enters `AccountName#1234`.
   Validated for PoE format (name + `#` + 4 digits). Stored locally.
   Same identity key used when GGG OAuth goes live — early testers keep
   their data.

2. **GGG OAuth (later):** Apply for OAuth app approval. Users authenticate
   with real PoE account. Requires community mass for approval.

## Implementation Steps

### Step 1: rqe-server — User Identity

- Add `owner` field to `StoredQuery` (the `AccountName#1234` string)
- Server endpoints accept owner from auth token
- `POST /queries` requires owner — queries belong to a user
- `GET /queries` can filter by owner ("my queries")
- `POST /match` response includes owner info for each match
- Database schema: add `owner TEXT` column to queries table

### Step 2: App — Login Gate UI

- New Settings section: "Demand Marketplace" (or similar)
- Login screen: text input for `AccountName#1234`, validate format
- Store login state in app store (persisted)
- All RQE features hidden until logged in
- `poe-rqe-client` sends identity token with requests

### Step 3: App — Query Management UI

- Reuse stat template picker pattern from poe-eval scoring profiles
- `stat_suggestions_for_query()` for fuzzy stat search
- Keys generated using `item_to_entry()` format (stat_ids)
- CRUD: create, edit, delete want lists
- Labels for organization (build names, priorities)

### Step 4: App — Overlay Integration

- On Ctrl+I item inspect: evaluate locally (existing) + async RQE check
- Both run in parallel — local eval is instant, RQE is network
- Overlay shows match count badge: "3 want lists match"
- Click for details: who wants it, their labels
- Gated: only runs if user is logged in

## Caution

Other Claude sessions may be modifying app files concurrently.
Check `git status` before touching app/ files. Coordinate changes
to avoid conflicts in lib.rs, store.ts, settings components.

## Architecture Reference

```
app (Tauri overlay)
  ├─ poe-eval (local scoring — existing)
  └─ poe-rqe-client (RQE service client)
       ├─ item_to_entry() — ResolvedItem → Entry
       └─ RqeClient — HTTP to rqe-server
            └─ rqe-server (domain-free service)
                 └─ poe-rqe (generic matching engine)
```

## Crates involved

| Crate | Changes needed |
|-------|---------------|
| rqe-server | Add owner field, per-user query filtering |
| poe-rqe-client | Add owner to add_query(), match response includes owners |
| rqe-cli | Add --owner flag for testing |
| app | Login gate, query builder UI, overlay badge |
