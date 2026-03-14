# poe-rqe-client — Client for RQE Service

## Purpose

Client library for the rqe-server HTTP service. Converts PoE items (from poe-item) into
the flat Entry format that rqe-server accepts, and provides an async HTTP client for all
RQE endpoints.

Analogous to poe-trade (client for GGG's trade API). Both are adapter crates that bridge
the PoE domain (poe-item's `ResolvedItem`) to an external service's wire format.

## Status

Working: `ResolvedItem → Entry` conversion + async HTTP client. 8 conversion tests.

## Architecture

```
poe-item (ResolvedItem)
    ├─ poe-trade     → TradeSearchBody  (GGG trade API)
    └─ poe-rqe-client → Entry           (rqe-server)
```

Each downstream crate owns its conversion from `ResolvedItem`. The RQE service
(rqe-server + poe-rqe) stays domain-free.

```
convert.rs  — item_to_entry(): ResolvedItem → Entry
client.rs   — RqeClient: async HTTP client for rqe-server endpoints
```

## Conversion: `ResolvedItem` → `Entry`

Entry is a flat `HashMap<String, EntryValue>` (string/integer/boolean values).

### Key format

| Key | Type | Example |
|-----|------|---------|
| `item_class` | string | `"Boots"` |
| `rarity` | string | `"Rare"` |
| `rarity_class` | string | `"Non-Unique"` or `"Unique"` |
| `base_type` | string | `"Titan Greaves"` |
| `name` | string | `"Test Boots"` |
| `item_level` | integer | `75` |
| `corrupted` | boolean | `false` |
| `fractured` | boolean | `false` |
| `socket_count` | integer | `4` |
| `max_link` | integer | `3` |
| `requirement_level` | integer | `68` |
| `influence.Shaper` | boolean | `true` |
| `influence_count` | integer | `1` |
| `implicit_count` | integer | `1` |
| `explicit_count` | integer | `5` |
| `{source}.{stat_id}` | integer | `explicit.base_maximum_life`: `32` |
| `{source}.{template}` | integer | `explicit.+# to maximum Life`: `32` (fallback) |

### Stat resolution strategy

1. **Preferred**: Use `stat_ids` from poe-item's resolver (stable, language-independent).
   Each stat_id gets its own entry with its corresponding value.
2. **Fallback**: When stat_ids are not resolved, extract a template from `display_text`
   by replacing numeric values with `#`.

## Dependencies

- `poe-item` — `ResolvedItem` types (with serde feature)
- `poe-rqe` — `Entry`, `EntryValue`, `Condition`, `QueryId` types
- `reqwest` — async HTTP client (rustls)
- `serde`, `serde_json` — serialization
- `thiserror` — error types
